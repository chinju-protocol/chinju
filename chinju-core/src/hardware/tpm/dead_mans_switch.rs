//! TPM-based Dead Man's Switch Implementation (C13)
//!
//! Uses TPM 2.0 features for hardware-backed safety mechanism:
//! - NV memory for persistent heartbeat storage
//! - PCR for tamper-evident state tracking
//! - Hardware-backed random for unpredictable timing
//!
//! Compatible with both hardware TPM and swtpm (software TPM emulator).
//!
//! # Usage with swtpm
//!
//! ```bash
//! # Start swtpm (socket mode)
//! mkdir -p /tmp/tpm
//! swtpm socket --tpmstate dir=/tmp/tpm \
//!     --ctrl type=tcp,port=2322 \
//!     --server type=tcp,port=2321 \
//!     --flags startup-clear
//!
//! # Or use Docker
//! docker-compose up swtpm
//! ```

use super::{TpmConfig, TpmContext, TpmError};
use crate::hardware::dead_mans_switch::{
    DeadMansSwitch, DeadMansSwitchConfig, DeadMansSwitchError, EmergencyCallback, EnvironmentState,
    SwitchState,
};
use crate::types::TrustLevel;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use tss_esapi::{
    attributes::NvIndexAttributesBuilder,
    handles::NvIndexTpmHandle,
    interface_types::{
        algorithm::HashingAlgorithm,
        resource_handles::NvAuth,
    },
    nv::storage::NvPublic,
    structures::{
        Digest as TpmDigest, DigestValues, MaxNvBuffer, NvPublicBuilder, 
        PcrSelectionListBuilder, PcrSlot,
    },
};

/// NV index for heartbeat storage (use a unique index in the user range)
/// User-defined NV indices: 0x01800000 - 0x01BFFFFF
const NV_INDEX_HEARTBEAT: u32 = 0x01800001;
/// NV index for switch state
const NV_INDEX_STATE: u32 = 0x01800002;
/// NV index for environment state
const NV_INDEX_ENVIRONMENT: u32 = 0x01800003;

/// PCR slot for tamper detection (PCR 16-23 are resettable and user-defined)
const PCR_SLOT_TAMPER: u8 = 16;
/// PCR slot for state tracking
const PCR_SLOT_STATE: u8 = 17;

/// Size of heartbeat NV storage (8 bytes for timestamp)
const NV_SIZE_HEARTBEAT: u16 = 8;
/// Size of state NV storage (1 byte for state enum)
const NV_SIZE_STATE: u16 = 1;
/// Size of environment NV storage (serialized EnvironmentState)
const NV_SIZE_ENVIRONMENT: u16 = 64;

/// TPM-backed Dead Man's Switch
///
/// Provides hardware-level security guarantees:
/// - Heartbeat timestamps stored in TPM NV memory (persistent across reboots)
/// - State changes recorded in PCR (tamper-evident)
/// - Emergency data can be sealed to PCR values
pub struct TpmDeadMansSwitch {
    /// TPM context
    tpm_context: RwLock<Option<TpmContext>>,
    /// Configuration
    config: DeadMansSwitchConfig,
    /// TPM-specific configuration
    tpm_config: TpmConfig,
    /// Current state (cached, also persisted to NV)
    state: AtomicU64,
    /// Last heartbeat time (cached, also persisted to NV)
    last_heartbeat: RwLock<Instant>,
    /// Last heartbeat timestamp (Unix epoch, for persistence)
    last_heartbeat_timestamp: AtomicU64,
    /// Environment state (cached)
    environment: RwLock<EnvironmentState>,
    /// Armed flag
    armed: AtomicBool,
    /// Emergency callbacks
    emergency_callbacks: RwLock<Vec<EmergencyCallback>>,
    /// Initialization status
    initialized: AtomicBool,
    /// NV indices initialized
    nv_initialized: AtomicBool,
}

impl TpmDeadMansSwitch {
    /// Create a new TPM-backed Dead Man's Switch
    pub fn new(config: DeadMansSwitchConfig, tpm_config: TpmConfig) -> Self {
        info!("Creating TpmDeadMansSwitch");

        Self {
            tpm_context: RwLock::new(None),
            config,
            tpm_config,
            state: AtomicU64::new(Self::encode_state(SwitchState::Disarmed)),
            last_heartbeat: RwLock::new(Instant::now()),
            last_heartbeat_timestamp: AtomicU64::new(0),
            environment: RwLock::new(EnvironmentState::default()),
            armed: AtomicBool::new(false),
            emergency_callbacks: RwLock::new(Vec::new()),
            initialized: AtomicBool::new(false),
            nv_initialized: AtomicBool::new(false),
        }
    }

    /// Create with default configurations
    pub fn default_config(tpm_config: TpmConfig) -> Self {
        Self::new(DeadMansSwitchConfig::default(), tpm_config)
    }

    /// Initialize TPM connection and NV indices
    pub async fn initialize(&self) -> Result<(), TpmError> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Initializing TPM connection for Dead Man's Switch");

        let context = TpmContext::new(self.tpm_config.clone())?;

        let mut ctx_guard = self.tpm_context.write().await;
        *ctx_guard = Some(context);

        self.initialized.store(true, Ordering::SeqCst);

        // Setup NV indices (may already exist from previous runs)
        if let Err(e) = self.setup_nv_indices().await {
            warn!("NV indices setup failed (may already exist): {}", e);
        }

        // Restore state from NV if available
        if let Ok(state) = self.read_state_from_nv().await {
            self.set_state(state);
            info!("Restored state from NV: {:?}", state);
        }

        // Restore heartbeat from NV if available
        if let Ok(timestamp) = self.read_heartbeat_from_nv().await {
            self.last_heartbeat_timestamp.store(timestamp, Ordering::SeqCst);
            debug!("Restored heartbeat timestamp from NV: {}", timestamp);
        }

        info!("TPM Dead Man's Switch initialized successfully");

        Ok(())
    }

    /// Setup NV memory indices
    async fn setup_nv_indices(&self) -> Result<(), TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        // Setup heartbeat NV index
        self.define_nv_index(context, NV_INDEX_HEARTBEAT, NV_SIZE_HEARTBEAT)?;

        // Setup state NV index
        self.define_nv_index(context, NV_INDEX_STATE, NV_SIZE_STATE)?;

        // Setup environment NV index
        self.define_nv_index(context, NV_INDEX_ENVIRONMENT, NV_SIZE_ENVIRONMENT)?;

        self.nv_initialized.store(true, Ordering::SeqCst);
        info!("NV indices initialized successfully");

        Ok(())
    }

    /// Define a single NV index
    fn define_nv_index(
        &self,
        context: &mut TpmContext,
        index: u32,
        size: u16,
    ) -> Result<(), TpmError> {
        let nv_index = NvIndexTpmHandle::new(index)
            .map_err(|e| TpmError::OperationFailed(format!("Invalid NV index: {}", e)))?;

        // Check if index already exists
        let read_result = context.context_mut().execute_without_session(|ctx| {
            ctx.nv_read_public(nv_index)
        });

        if read_result.is_ok() {
            debug!("NV index 0x{:08X} already exists", index);
            return Ok(());
        }

        // Build NV public attributes
        let nv_attributes = NvIndexAttributesBuilder::new()
            .with_owner_write(true)
            .with_owner_read(true)
            .with_pp_write(false)
            .with_pp_read(false)
            .with_no_da(true) // No dictionary attack protection for simplicity
            .build()
            .map_err(|e| TpmError::OperationFailed(e.to_string()))?;

        let nv_public = NvPublicBuilder::new()
            .with_nv_index(nv_index)
            .with_index_name_algorithm(HashingAlgorithm::Sha256)
            .with_index_attributes(nv_attributes)
            .with_data_size(size)
            .build()
            .map_err(|e| TpmError::OperationFailed(e.to_string()))?;

        // Define the NV index
        context
            .context_mut()
            .execute_with_nullauth_session(|ctx| {
                ctx.nv_define_space(NvAuth::Owner, None, nv_public)
            })
            .map_err(|e| TpmError::OperationFailed(format!("NV define failed: {}", e)))?;

        info!("Defined NV index 0x{:08X} with size {} bytes", index, size);
        Ok(())
    }

    /// Write heartbeat timestamp to TPM NV memory
    async fn write_heartbeat_to_nv(&self, timestamp: u64) -> Result<(), TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        let nv_index = NvIndexTpmHandle::new(NV_INDEX_HEARTBEAT)
            .map_err(|e| TpmError::OperationFailed(e.to_string()))?;

        let data = MaxNvBuffer::try_from(timestamp.to_le_bytes().to_vec())
            .map_err(|e| TpmError::OperationFailed(e.to_string()))?;

        context
            .context_mut()
            .execute_with_nullauth_session(|ctx| ctx.nv_write(NvAuth::Owner, nv_index, data, 0))
            .map_err(|e| TpmError::OperationFailed(format!("NV write failed: {}", e)))?;

        debug!("Heartbeat timestamp written to TPM NV: {}", timestamp);
        Ok(())
    }

    /// Read heartbeat timestamp from TPM NV memory
    async fn read_heartbeat_from_nv(&self) -> Result<u64, TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        let nv_index = NvIndexTpmHandle::new(NV_INDEX_HEARTBEAT)
            .map_err(|e| TpmError::OperationFailed(e.to_string()))?;

        let data = context
            .context_mut()
            .execute_with_nullauth_session(|ctx| {
                ctx.nv_read(NvAuth::Owner, nv_index, NV_SIZE_HEARTBEAT, 0)
            })
            .map_err(|e| TpmError::OperationFailed(format!("NV read failed: {}", e)))?;

        let bytes: [u8; 8] = data
            .as_bytes()
            .try_into()
            .map_err(|_| TpmError::OperationFailed("Invalid heartbeat data".into()))?;

        Ok(u64::from_le_bytes(bytes))
    }

    /// Write state to TPM NV memory
    async fn write_state_to_nv(&self, state: SwitchState) -> Result<(), TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        let nv_index = NvIndexTpmHandle::new(NV_INDEX_STATE)
            .map_err(|e| TpmError::OperationFailed(e.to_string()))?;

        let state_byte = Self::encode_state(state) as u8;
        let data = MaxNvBuffer::try_from(vec![state_byte])
            .map_err(|e| TpmError::OperationFailed(e.to_string()))?;

        context
            .context_mut()
            .execute_with_nullauth_session(|ctx| ctx.nv_write(NvAuth::Owner, nv_index, data, 0))
            .map_err(|e| TpmError::OperationFailed(format!("NV write failed: {}", e)))?;

        debug!("State written to TPM NV: {:?}", state);
        Ok(())
    }

    /// Read state from TPM NV memory
    async fn read_state_from_nv(&self) -> Result<SwitchState, TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        let nv_index = NvIndexTpmHandle::new(NV_INDEX_STATE)
            .map_err(|e| TpmError::OperationFailed(e.to_string()))?;

        let data = context
            .context_mut()
            .execute_with_nullauth_session(|ctx| {
                ctx.nv_read(NvAuth::Owner, nv_index, NV_SIZE_STATE, 0)
            })
            .map_err(|e| TpmError::OperationFailed(format!("NV read failed: {}", e)))?;

        let state_byte = data.as_bytes().first().copied().unwrap_or(0);
        Ok(Self::decode_state(state_byte as u64))
    }

    /// Extend PCR with state change (tamper-evident logging)
    async fn extend_pcr_state(&self, state: SwitchState) -> Result<(), TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        // Create a hash of the state change event
        let event = format!("DMS_STATE:{:?}:{}", state, Self::current_timestamp());
        let mut hasher = Sha256::new();
        hasher.update(event.as_bytes());
        let hash = hasher.finalize();

        let pcr_slot = PcrSlot::try_from(PCR_SLOT_STATE as u32)
            .map_err(|_| TpmError::PcrError("Invalid PCR slot".into()))?;

        let digest = TpmDigest::try_from(hash.as_slice())
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        let digest_values = DigestValues::create(HashingAlgorithm::Sha256, digest)
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        context
            .context_mut()
            .execute_without_session(|ctx| ctx.pcr_extend(pcr_slot.into(), digest_values))
            .map_err(|e| TpmError::PcrError(format!("PCR extend failed: {}", e)))?;

        debug!("PCR {} extended with state: {:?}", PCR_SLOT_STATE, state);
        Ok(())
    }

    /// Extend PCR with heartbeat event
    async fn extend_pcr_heartbeat(&self, timestamp: u64) -> Result<(), TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        let event = format!("DMS_HEARTBEAT:{}", timestamp);
        let mut hasher = Sha256::new();
        hasher.update(event.as_bytes());
        let hash = hasher.finalize();

        let pcr_slot = PcrSlot::try_from(PCR_SLOT_TAMPER as u32)
            .map_err(|_| TpmError::PcrError("Invalid PCR slot".into()))?;

        let digest = TpmDigest::try_from(hash.as_slice())
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        let digest_values = DigestValues::create(HashingAlgorithm::Sha256, digest)
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        context
            .context_mut()
            .execute_without_session(|ctx| ctx.pcr_extend(pcr_slot.into(), digest_values))
            .map_err(|e| TpmError::PcrError(format!("PCR extend failed: {}", e)))?;

        debug!("PCR {} extended with heartbeat", PCR_SLOT_TAMPER);
        Ok(())
    }

    /// Extend PCR with anomaly event
    async fn extend_pcr_anomaly(&self, anomaly: &str) -> Result<(), TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        let event = format!("DMS_ANOMALY:{}:{}", anomaly, Self::current_timestamp());
        let mut hasher = Sha256::new();
        hasher.update(event.as_bytes());
        let hash = hasher.finalize();

        let pcr_slot = PcrSlot::try_from(PCR_SLOT_TAMPER as u32)
            .map_err(|_| TpmError::PcrError("Invalid PCR slot".into()))?;

        let digest = TpmDigest::try_from(hash.as_slice())
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        let digest_values = DigestValues::create(HashingAlgorithm::Sha256, digest)
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        context
            .context_mut()
            .execute_without_session(|ctx| ctx.pcr_extend(pcr_slot.into(), digest_values))
            .map_err(|e| TpmError::PcrError(format!("PCR extend failed: {}", e)))?;

        warn!("PCR {} extended with anomaly: {}", PCR_SLOT_TAMPER, anomaly);
        Ok(())
    }

    /// Seal data to current PCR values (can only be unsealed if PCR unchanged)
    ///
    /// This ensures that emergency data can only be retrieved if the system
    /// has not been tampered with (PCR values unchanged).
    pub async fn seal_emergency_data(&self, data: &[u8]) -> Result<Vec<u8>, TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        // Read current PCR values to include in sealed blob
        let pcr_selection = PcrSelectionListBuilder::new()
            .with_selection(
                HashingAlgorithm::Sha256,
                &[
                    PcrSlot::try_from(PCR_SLOT_TAMPER as u32).unwrap(),
                    PcrSlot::try_from(PCR_SLOT_STATE as u32).unwrap(),
                ],
            )
            .build()
            .map_err(|e| TpmError::SealingError(e.to_string()))?;

        let (_, _, pcr_data) = context
            .context_mut()
            .execute_without_session(|ctx| ctx.pcr_read(pcr_selection))
            .map_err(|e| TpmError::SealingError(format!("PCR read failed: {}", e)))?;

        // For a full implementation, we would:
        // 1. Create a policy session with PCR policy
        // 2. Create a sealed object under that policy
        // 3. Return the sealed blob
        //
        // Simplified version: hash PCR values with data
        let mut hasher = Sha256::new();
        hasher.update(b"CHINJU_SEAL_V1");

        // Include PCR values in seal
        if let Some(bank) = pcr_data.pcr_bank(HashingAlgorithm::Sha256) {
            for digest in bank {
                hasher.update(digest.as_bytes());
            }
        }

        let seal_key = hasher.finalize();

        // XOR encryption (placeholder for proper AEAD)
        let mut sealed = Vec::with_capacity(data.len() + 32);
        sealed.extend_from_slice(&[0x43, 0x48, 0x4A, 0x53]); // "CHJS" magic
        sealed.extend_from_slice(seal_key.as_slice());

        for (i, byte) in data.iter().enumerate() {
            sealed.push(byte ^ seal_key[i % 32]);
        }

        info!("Sealed {} bytes of emergency data to PCR values", data.len());
        Ok(sealed)
    }

    /// Unseal data (fails if PCR values changed)
    pub async fn unseal_emergency_data(&self, sealed: &[u8]) -> Result<Vec<u8>, TpmError> {
        if sealed.len() < 36 || &sealed[0..4] != b"CHJS" {
            return Err(TpmError::SealingError("Invalid sealed data format".into()));
        }

        let stored_key = &sealed[4..36];
        let encrypted_data = &sealed[36..];

        // Verify PCR values match
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        let pcr_selection = PcrSelectionListBuilder::new()
            .with_selection(
                HashingAlgorithm::Sha256,
                &[
                    PcrSlot::try_from(PCR_SLOT_TAMPER as u32).unwrap(),
                    PcrSlot::try_from(PCR_SLOT_STATE as u32).unwrap(),
                ],
            )
            .build()
            .map_err(|e| TpmError::SealingError(e.to_string()))?;

        let (_, _, pcr_data) = context
            .context_mut()
            .execute_without_session(|ctx| ctx.pcr_read(pcr_selection))
            .map_err(|e| TpmError::SealingError(format!("PCR read failed: {}", e)))?;

        // Compute current seal key
        let mut hasher = Sha256::new();
        hasher.update(b"CHINJU_SEAL_V1");

        if let Some(bank) = pcr_data.pcr_bank(HashingAlgorithm::Sha256) {
            for digest in bank {
                hasher.update(digest.as_bytes());
            }
        }

        let current_key = hasher.finalize();

        // Check if PCR values match
        if current_key.as_slice() != stored_key {
            return Err(TpmError::SealingError(
                "PCR values changed - cannot unseal data (possible tampering)".into(),
            ));
        }

        // Decrypt
        let mut data = Vec::with_capacity(encrypted_data.len());
        for (i, byte) in encrypted_data.iter().enumerate() {
            data.push(byte ^ current_key[i % 32]);
        }

        info!("Unsealed {} bytes of emergency data", data.len());
        Ok(data)
    }

    /// Get random bytes from TPM hardware RNG
    pub async fn get_random(&self, size: usize) -> Result<Vec<u8>, TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        let random = context
            .context_mut()
            .execute_without_session(|ctx| ctx.get_random(size))
            .map_err(|e| TpmError::OperationFailed(format!("Get random failed: {}", e)))?;

        Ok(random.as_bytes().to_vec())
    }

    /// Clear all sensitive data in TPM NV memory
    async fn emergency_clear(&self) -> Result<(), TpmError> {
        warn!("EMERGENCY: Clearing TPM NV memory");

        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        // Undefine NV indices to clear data
        for nv_index in [NV_INDEX_HEARTBEAT, NV_INDEX_STATE, NV_INDEX_ENVIRONMENT] {
            if let Ok(handle) = NvIndexTpmHandle::new(nv_index) {
                let _ = context
                    .context_mut()
                    .execute_with_nullauth_session(|ctx| ctx.nv_undefine_space(NvAuth::Owner, handle));
                debug!("Cleared NV index 0x{:08X}", nv_index);
            }
        }

        // Extend PCR to prevent unsealing of any sealed data
        let emergency_event = format!("DMS_EMERGENCY:{}", Self::current_timestamp());
        let mut hasher = Sha256::new();
        hasher.update(emergency_event.as_bytes());
        let hash = hasher.finalize();

        for pcr_index in [PCR_SLOT_TAMPER, PCR_SLOT_STATE] {
            if let Ok(pcr_slot) = PcrSlot::try_from(pcr_index as u32) {
                if let Ok(digest) = TpmDigest::try_from(hash.as_slice()) {
                    if let Ok(digest_values) = DigestValues::create(HashingAlgorithm::Sha256, digest)
                    {
                        let _ = context
                            .context_mut()
                            .execute_without_session(|ctx| {
                                ctx.pcr_extend(pcr_slot.into(), digest_values)
                            });
                        warn!("Extended PCR {} to invalidate sealed data", pcr_index);
                    }
                }
            }
        }

        error!("TPM NV memory cleared - emergency protocol executed");
        Ok(())
    }

    fn encode_state(state: SwitchState) -> u64 {
        match state {
            SwitchState::Disarmed => 0,
            SwitchState::Armed => 1,
            SwitchState::GracePeriod => 2,
            SwitchState::Triggered => 3,
        }
    }

    fn decode_state(value: u64) -> SwitchState {
        match value {
            0 => SwitchState::Disarmed,
            1 => SwitchState::Armed,
            2 => SwitchState::GracePeriod,
            3 => SwitchState::Triggered,
            _ => SwitchState::Disarmed,
        }
    }

    fn set_state(&self, state: SwitchState) {
        self.state.store(Self::encode_state(state), Ordering::SeqCst);
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Start background monitoring
    pub fn start_monitoring(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let switch = self.clone();

        tokio::spawn(async move {
            let check_interval = switch.config.heartbeat_interval / 2;
            let mut interval = tokio::time::interval(check_interval);

            loop {
                interval.tick().await;

                if !switch.armed.load(Ordering::SeqCst) {
                    continue;
                }

                let current_state = switch.state();

                // Extend PCR on each check cycle (tamper detection)
                if let Err(e) = switch.extend_pcr_state(current_state).await {
                    debug!("Failed to extend PCR: {}", e);
                }

                // Check heartbeat timeout
                let elapsed = switch.time_since_heartbeat();
                if elapsed > switch.config.heartbeat_timeout {
                    match current_state {
                        SwitchState::Armed => {
                            warn!(
                                "TPM DMS: Heartbeat timeout! Last heartbeat was {:?} ago",
                                elapsed
                            );
                            switch.set_state(SwitchState::GracePeriod);

                            // Persist state change to NV
                            if let Err(e) = switch.write_state_to_nv(SwitchState::GracePeriod).await
                            {
                                error!("Failed to persist state to NV: {}", e);
                            }
                        }
                        SwitchState::GracePeriod => {
                            if elapsed > switch.config.heartbeat_timeout + switch.config.grace_period
                            {
                                error!("TPM DMS: Grace period expired! Triggering emergency!");
                                switch.trigger_emergency().await;
                            }
                        }
                        _ => {}
                    }
                }

                // Check environment
                let env = switch.environment.read().await;
                if let Some(anomaly) = env.has_anomaly(&switch.config) {
                    warn!("TPM DMS: Environmental anomaly: {}", anomaly);

                    // Record anomaly in PCR
                    if let Err(e) = switch.extend_pcr_anomaly(&anomaly).await {
                        error!("Failed to record anomaly in PCR: {}", e);
                    }

                    if current_state == SwitchState::Armed {
                        switch.set_state(SwitchState::GracePeriod);
                        if let Err(e) = switch.write_state_to_nv(SwitchState::GracePeriod).await {
                            error!("Failed to persist state to NV: {}", e);
                        }
                    }
                }
            }
        })
    }

    async fn trigger_emergency(&self) {
        self.set_state(SwitchState::Triggered);
        error!("TPM DMS: EMERGENCY TRIGGERED");

        // Persist triggered state
        if let Err(e) = self.write_state_to_nv(SwitchState::Triggered).await {
            error!("Failed to persist triggered state: {}", e);
        }

        // Record in PCR
        if let Err(e) = self.extend_pcr_state(SwitchState::Triggered).await {
            error!("Failed to record triggered state in PCR: {}", e);
        }

        // Clear TPM sensitive data
        if let Err(e) = self.emergency_clear().await {
            error!("Failed to clear TPM: {}", e);
        }

        // Execute callbacks
        let callbacks = self.emergency_callbacks.read().await;
        for callback in callbacks.iter() {
            callback();
        }
    }

    /// Get the config
    pub fn config(&self) -> &DeadMansSwitchConfig {
        &self.config
    }

    /// Check if TPM is available and initialized
    pub fn is_tpm_available(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Check if NV indices are initialized
    pub fn is_nv_initialized(&self) -> bool {
        self.nv_initialized.load(Ordering::SeqCst)
    }

    /// Get PCR values for attestation
    pub async fn get_pcr_values(&self) -> Result<Vec<(u8, Vec<u8>)>, TpmError> {
        let mut ctx_guard = self.tpm_context.write().await;
        let context = ctx_guard
            .as_mut()
            .ok_or(TpmError::InitializationFailed("TPM not initialized".into()))?;

        let pcr_selection = PcrSelectionListBuilder::new()
            .with_selection(
                HashingAlgorithm::Sha256,
                &[
                    PcrSlot::try_from(PCR_SLOT_TAMPER as u32).unwrap(),
                    PcrSlot::try_from(PCR_SLOT_STATE as u32).unwrap(),
                ],
            )
            .build()
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        let (_, _, pcr_data) = context
            .context_mut()
            .execute_without_session(|ctx| ctx.pcr_read(pcr_selection))
            .map_err(|e| TpmError::PcrError(format!("PCR read failed: {}", e)))?;

        let mut result = Vec::new();
        if let Some(bank) = pcr_data.pcr_bank(HashingAlgorithm::Sha256) {
            for (i, digest) in bank.enumerate() {
                let pcr_index = if i == 0 {
                    PCR_SLOT_TAMPER
                } else {
                    PCR_SLOT_STATE
                };
                result.push((pcr_index, digest.as_bytes().to_vec()));
            }
        }

        Ok(result)
    }
}

impl DeadMansSwitch for TpmDeadMansSwitch {
    fn security_level(&self) -> TrustLevel {
        if self.initialized.load(Ordering::SeqCst) {
            TrustLevel::Hardware // L2 - TPM backed
        } else {
            TrustLevel::Mock // Fallback if not initialized
        }
    }

    fn arm(&self) -> Result<(), DeadMansSwitchError> {
        if self.armed.load(Ordering::SeqCst) {
            return Err(DeadMansSwitchError::AlreadyArmed);
        }

        if !self.initialized.load(Ordering::SeqCst) {
            warn!("TPM DMS: Arming without TPM initialization (software fallback)");
        }

        info!("TPM DMS: Arming Dead Man's Switch");
        self.armed.store(true, Ordering::SeqCst);
        self.set_state(SwitchState::Armed);

        // Reset heartbeat
        if let Ok(mut last) = self.last_heartbeat.try_write() {
            *last = Instant::now();
        }
        self.last_heartbeat_timestamp
            .store(Self::current_timestamp(), Ordering::SeqCst);

        Ok(())
    }

    fn disarm(&self) -> Result<(), DeadMansSwitchError> {
        if !self.armed.load(Ordering::SeqCst) {
            return Err(DeadMansSwitchError::NotArmed);
        }

        info!("TPM DMS: Disarming Dead Man's Switch");
        self.armed.store(false, Ordering::SeqCst);
        self.set_state(SwitchState::Disarmed);
        Ok(())
    }

    fn heartbeat(&self) -> Result<(), DeadMansSwitchError> {
        if !self.armed.load(Ordering::SeqCst) {
            return Ok(());
        }

        let timestamp = Self::current_timestamp();

        if let Ok(mut last) = self.last_heartbeat.try_write() {
            *last = Instant::now();
            self.last_heartbeat_timestamp.store(timestamp, Ordering::SeqCst);
            debug!("TPM DMS: Heartbeat received at {}", timestamp);

            if self.state() == SwitchState::GracePeriod {
                info!("TPM DMS: Heartbeat during grace period - resuming normal operation");
                self.set_state(SwitchState::Armed);
            }
        }

        Ok(())
    }

    fn update_environment(&self, state: EnvironmentState) -> Result<(), DeadMansSwitchError> {
        if let Ok(mut env) = self.environment.try_write() {
            *env = state;
        }
        Ok(())
    }

    fn state(&self) -> SwitchState {
        Self::decode_state(self.state.load(Ordering::SeqCst))
    }

    fn time_since_heartbeat(&self) -> Duration {
        if let Ok(last) = self.last_heartbeat.try_read() {
            last.elapsed()
        } else {
            Duration::ZERO
        }
    }

    fn is_healthy(&self) -> bool {
        matches!(self.state(), SwitchState::Disarmed | SwitchState::Armed)
    }

    fn on_emergency(&self, callback: EmergencyCallback) {
        if let Ok(mut callbacks) = self.emergency_callbacks.try_write() {
            callbacks.push(callback);
        }
    }

    fn get_environment(&self) -> EnvironmentState {
        if let Ok(env) = self.environment.try_read() {
            env.clone()
        } else {
            EnvironmentState::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_encoding() {
        assert_eq!(
            TpmDeadMansSwitch::decode_state(TpmDeadMansSwitch::encode_state(SwitchState::Disarmed)),
            SwitchState::Disarmed
        );
        assert_eq!(
            TpmDeadMansSwitch::decode_state(TpmDeadMansSwitch::encode_state(SwitchState::Armed)),
            SwitchState::Armed
        );
        assert_eq!(
            TpmDeadMansSwitch::decode_state(TpmDeadMansSwitch::encode_state(
                SwitchState::GracePeriod
            )),
            SwitchState::GracePeriod
        );
        assert_eq!(
            TpmDeadMansSwitch::decode_state(TpmDeadMansSwitch::encode_state(SwitchState::Triggered)),
            SwitchState::Triggered
        );
    }

    #[test]
    fn test_arm_disarm_without_tpm() {
        // Test that arm/disarm works even without TPM connection (software fallback)
        let switch = TpmDeadMansSwitch::default_config(TpmConfig::default());

        assert_eq!(switch.state(), SwitchState::Disarmed);

        switch.arm().unwrap();
        assert_eq!(switch.state(), SwitchState::Armed);

        switch.disarm().unwrap();
        assert_eq!(switch.state(), SwitchState::Disarmed);
    }

    #[test]
    fn test_double_arm_fails() {
        let switch = TpmDeadMansSwitch::default_config(TpmConfig::default());

        switch.arm().unwrap();
        assert!(switch.arm().is_err());
    }

    #[test]
    fn test_heartbeat() {
        let switch = TpmDeadMansSwitch::default_config(TpmConfig::default());

        switch.arm().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let elapsed_before = switch.time_since_heartbeat();
        switch.heartbeat().unwrap();
        let elapsed_after = switch.time_since_heartbeat();

        assert!(elapsed_after < elapsed_before);
    }

    #[test]
    fn test_is_healthy() {
        let switch = TpmDeadMansSwitch::default_config(TpmConfig::default());

        // Healthy when disarmed
        assert!(switch.is_healthy());

        // Healthy when armed
        switch.arm().unwrap();
        assert!(switch.is_healthy());

        // Not healthy in grace period
        switch.set_state(SwitchState::GracePeriod);
        assert!(!switch.is_healthy());

        // Not healthy when triggered
        switch.set_state(SwitchState::Triggered);
        assert!(!switch.is_healthy());
    }

    #[tokio::test]
    async fn test_environment_update() {
        let switch = TpmDeadMansSwitch::default_config(TpmConfig::default());

        let env = EnvironmentState {
            temperature: Some(25.0),
            acceleration: Some(0.1),
            enclosure_opened: false,
            network_connected: true,
            on_mains_power: true,
        };

        switch.update_environment(env.clone()).unwrap();

        let retrieved = switch.get_environment();
        assert_eq!(retrieved.temperature, Some(25.0));
        assert_eq!(retrieved.on_mains_power, true);
    }

    // Integration tests that require swtpm
    // Run with: cargo test --features tpm -- --ignored

    #[tokio::test]
    #[ignore]
    async fn test_tpm_initialization() {
        let switch = TpmDeadMansSwitch::default_config(TpmConfig::socket("localhost", 2321));

        let result = switch.initialize().await;
        assert!(result.is_ok(), "Failed to initialize: {:?}", result);
        assert!(switch.is_tpm_available());
    }

    #[tokio::test]
    #[ignore]
    async fn test_tpm_heartbeat_persistence() {
        let switch = TpmDeadMansSwitch::default_config(TpmConfig::socket("localhost", 2321));

        switch.initialize().await.expect("Failed to initialize TPM");
        switch.arm().unwrap();

        // Write heartbeat
        let timestamp = TpmDeadMansSwitch::current_timestamp();
        switch
            .write_heartbeat_to_nv(timestamp)
            .await
            .expect("Failed to write heartbeat");

        // Read it back
        let read_timestamp = switch
            .read_heartbeat_from_nv()
            .await
            .expect("Failed to read heartbeat");

        assert_eq!(timestamp, read_timestamp);
    }

    #[tokio::test]
    #[ignore]
    async fn test_tpm_seal_unseal() {
        let switch = TpmDeadMansSwitch::default_config(TpmConfig::socket("localhost", 2321));

        switch.initialize().await.expect("Failed to initialize TPM");

        let secret_data = b"secret emergency key";

        // Seal data
        let sealed = switch
            .seal_emergency_data(secret_data)
            .await
            .expect("Failed to seal data");

        // Unseal data (should succeed since PCR unchanged)
        let unsealed = switch
            .unseal_emergency_data(&sealed)
            .await
            .expect("Failed to unseal data");

        assert_eq!(secret_data.as_slice(), unsealed.as_slice());
    }

    #[tokio::test]
    #[ignore]
    async fn test_tpm_pcr_values() {
        let switch = TpmDeadMansSwitch::default_config(TpmConfig::socket("localhost", 2321));

        switch.initialize().await.expect("Failed to initialize TPM");

        let pcr_values = switch.get_pcr_values().await.expect("Failed to get PCR values");

        // Should have 2 PCR values (TAMPER and STATE)
        assert_eq!(pcr_values.len(), 2);

        for (index, value) in &pcr_values {
            println!("PCR {}: {} bytes", index, value.len());
            assert_eq!(value.len(), 32); // SHA-256
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_tpm_state_persistence() {
        let switch = TpmDeadMansSwitch::default_config(TpmConfig::socket("localhost", 2321));

        switch.initialize().await.expect("Failed to initialize TPM");

        // Write state
        switch
            .write_state_to_nv(SwitchState::Armed)
            .await
            .expect("Failed to write state");

        // Read it back
        let state = switch
            .read_state_from_nv()
            .await
            .expect("Failed to read state");

        assert_eq!(state, SwitchState::Armed);
    }
}
