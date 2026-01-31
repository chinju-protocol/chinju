//! Dead Man's Switch - Physical Safety Mechanism (C13)
//!
//! Monitors heartbeat signals and environmental conditions.
//! Triggers emergency data erasure when anomalies are detected.
//!
//! Security levels:
//! - Soft: Software-only simulation (for development/testing)
//! - Hard: Hardware-backed with TPM/sensors (production)

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::types::TrustLevel;

/// Dead Man's Switch errors
#[derive(Debug, Error)]
pub enum DeadMansSwitchError {
    #[error("Heartbeat timeout: no signal for {0} seconds")]
    HeartbeatTimeout(u64),

    #[error("Environmental anomaly: {0}")]
    EnvironmentalAnomaly(String),

    #[error("Emergency shutdown triggered")]
    EmergencyShutdown,

    #[error("Switch already armed")]
    AlreadyArmed,

    #[error("Switch not armed")]
    NotArmed,

    #[error("Hardware error: {0}")]
    HardwareError(String),
}

/// Environmental sensor readings
#[derive(Debug, Clone, Default)]
pub struct EnvironmentState {
    /// Temperature in Celsius
    pub temperature: Option<f32>,
    /// Acceleration in G-force
    pub acceleration: Option<f32>,
    /// Whether enclosure is opened
    pub enclosure_opened: bool,
    /// Network connectivity status
    pub network_connected: bool,
    /// Power status (true = on mains, false = on battery)
    pub on_mains_power: bool,
}

impl EnvironmentState {
    /// Check if any environmental anomaly is detected
    pub fn has_anomaly(&self, config: &DeadMansSwitchConfig) -> Option<String> {
        // Temperature check
        if let Some(temp) = self.temperature {
            if temp < config.min_temperature {
                return Some(format!(
                    "Temperature too low: {}°C (min: {}°C)",
                    temp, config.min_temperature
                ));
            }
            if temp > config.max_temperature {
                return Some(format!(
                    "Temperature too high: {}°C (max: {}°C)",
                    temp, config.max_temperature
                ));
            }
        }

        // Acceleration check (tampering detection)
        if let Some(accel) = self.acceleration {
            if accel > config.max_acceleration {
                return Some(format!(
                    "Excessive acceleration: {}G (max: {}G)",
                    accel, config.max_acceleration
                ));
            }
        }

        // Enclosure opened check
        if self.enclosure_opened && !config.allow_enclosure_open {
            return Some("Enclosure opened - possible tampering".to_string());
        }

        None
    }
}

/// Configuration for Dead Man's Switch
#[derive(Debug, Clone)]
pub struct DeadMansSwitchConfig {
    /// Heartbeat interval (how often heartbeat should be sent)
    pub heartbeat_interval: Duration,
    /// Heartbeat timeout (how long to wait before triggering)
    pub heartbeat_timeout: Duration,
    /// Minimum allowed temperature (Celsius)
    pub min_temperature: f32,
    /// Maximum allowed temperature (Celsius)
    pub max_temperature: f32,
    /// Maximum allowed acceleration (G-force)
    pub max_acceleration: f32,
    /// Allow enclosure to be opened
    pub allow_enclosure_open: bool,
    /// Grace period before emergency action
    pub grace_period: Duration,
}

impl Default for DeadMansSwitchConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(30),
            heartbeat_timeout: Duration::from_secs(90), // 3 missed heartbeats
            min_temperature: 0.0,
            max_temperature: 50.0,
            max_acceleration: 1.0, // 1G = normal gravity, higher = movement/shock
            allow_enclosure_open: false,
            grace_period: Duration::from_secs(10),
        }
    }
}

/// Dead Man's Switch state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchState {
    /// Not armed, monitoring inactive
    Disarmed,
    /// Armed and monitoring
    Armed,
    /// Grace period active (anomaly detected)
    GracePeriod,
    /// Emergency triggered
    Triggered,
}

/// Callback for emergency actions
pub type EmergencyCallback = Arc<dyn Fn() + Send + Sync>;

/// Dead Man's Switch trait
pub trait DeadMansSwitch: Send + Sync {
    /// Get the security level of this implementation
    fn security_level(&self) -> TrustLevel;

    /// Arm the switch (start monitoring)
    fn arm(&self) -> Result<(), DeadMansSwitchError>;

    /// Disarm the switch (stop monitoring)
    fn disarm(&self) -> Result<(), DeadMansSwitchError>;

    /// Send heartbeat signal
    fn heartbeat(&self) -> Result<(), DeadMansSwitchError>;

    /// Update environment state (for hardware implementations)
    fn update_environment(&self, state: EnvironmentState) -> Result<(), DeadMansSwitchError>;

    /// Get current state
    fn state(&self) -> SwitchState;

    /// Get time since last heartbeat
    fn time_since_heartbeat(&self) -> Duration;

    /// Check if switch is healthy
    fn is_healthy(&self) -> bool;

    /// Register emergency callback
    fn on_emergency(&self, callback: EmergencyCallback);

    /// Get current environment state
    fn get_environment(&self) -> EnvironmentState;
}

/// Software-only Dead Man's Switch implementation (L0 - Mock)
///
/// This implementation runs entirely in software and simulates
/// hardware behavior for development and testing purposes.
pub struct SoftDeadMansSwitch {
    config: DeadMansSwitchConfig,
    state: AtomicU64, // Encoded SwitchState
    last_heartbeat: RwLock<Instant>,
    environment: RwLock<EnvironmentState>,
    armed: AtomicBool,
    emergency_callbacks: RwLock<Vec<EmergencyCallback>>,
}

impl SoftDeadMansSwitch {
    /// Create a new soft dead man's switch
    pub fn new(config: DeadMansSwitchConfig) -> Self {
        info!(
            "Initializing SoftDeadMansSwitch: heartbeat_interval={:?}, timeout={:?}",
            config.heartbeat_interval, config.heartbeat_timeout
        );

        Self {
            config,
            state: AtomicU64::new(Self::encode_state(SwitchState::Disarmed)),
            last_heartbeat: RwLock::new(Instant::now()),
            environment: RwLock::new(EnvironmentState::default()),
            armed: AtomicBool::new(false),
            emergency_callbacks: RwLock::new(Vec::new()),
        }
    }

    /// Create with default config
    pub fn default_config() -> Self {
        Self::new(DeadMansSwitchConfig::default())
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

    /// Start background monitoring task
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

                // Check heartbeat timeout
                let elapsed = switch.time_since_heartbeat();
                if elapsed > switch.config.heartbeat_timeout {
                    match current_state {
                        SwitchState::Armed => {
                            warn!(
                                "Heartbeat timeout! Last heartbeat was {:?} ago",
                                elapsed
                            );
                            switch.set_state(SwitchState::GracePeriod);
                        }
                        SwitchState::GracePeriod => {
                            if elapsed > switch.config.heartbeat_timeout + switch.config.grace_period
                            {
                                error!("Grace period expired! Triggering emergency!");
                                switch.trigger_emergency().await;
                            }
                        }
                        _ => {}
                    }
                }

                // Check environment
                let env = switch.environment.read().await;
                if let Some(anomaly) = env.has_anomaly(&switch.config) {
                    warn!("Environmental anomaly detected: {}", anomaly);
                    if current_state == SwitchState::Armed {
                        switch.set_state(SwitchState::GracePeriod);
                    }
                }
            }
        })
    }

    async fn trigger_emergency(&self) {
        self.set_state(SwitchState::Triggered);
        error!("EMERGENCY TRIGGERED - Executing emergency callbacks");

        let callbacks = self.emergency_callbacks.read().await;
        for callback in callbacks.iter() {
            callback();
        }
    }

    /// Simulate environmental conditions (for testing)
    pub async fn simulate_environment(&self, state: EnvironmentState) {
        let mut env = self.environment.write().await;
        *env = state;
        debug!("Environment simulated: {:?}", env);
    }

    /// Get config
    pub fn config(&self) -> &DeadMansSwitchConfig {
        &self.config
    }
}

impl DeadMansSwitch for SoftDeadMansSwitch {
    fn security_level(&self) -> TrustLevel {
        TrustLevel::Mock // L0 - Software only
    }

    fn arm(&self) -> Result<(), DeadMansSwitchError> {
        if self.armed.load(Ordering::SeqCst) {
            return Err(DeadMansSwitchError::AlreadyArmed);
        }

        info!("Arming Dead Man's Switch");
        self.armed.store(true, Ordering::SeqCst);
        self.set_state(SwitchState::Armed);

        // Reset heartbeat timer
        if let Ok(mut last) = self.last_heartbeat.try_write() {
            *last = Instant::now();
        }

        Ok(())
    }

    fn disarm(&self) -> Result<(), DeadMansSwitchError> {
        if !self.armed.load(Ordering::SeqCst) {
            return Err(DeadMansSwitchError::NotArmed);
        }

        info!("Disarming Dead Man's Switch");
        self.armed.store(false, Ordering::SeqCst);
        self.set_state(SwitchState::Disarmed);
        Ok(())
    }

    fn heartbeat(&self) -> Result<(), DeadMansSwitchError> {
        if !self.armed.load(Ordering::SeqCst) {
            return Ok(()); // Silently accept heartbeats when disarmed
        }

        if let Ok(mut last) = self.last_heartbeat.try_write() {
            *last = Instant::now();
            debug!("Heartbeat received");

            // Reset to armed if in grace period
            if self.state() == SwitchState::GracePeriod {
                info!("Heartbeat received during grace period - resuming normal operation");
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

impl Default for SoftDeadMansSwitch {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_environment_anomaly_detection() {
        let config = DeadMansSwitchConfig::default();

        // Normal state
        let normal = EnvironmentState {
            temperature: Some(25.0),
            acceleration: Some(0.5),
            enclosure_opened: false,
            network_connected: true,
            on_mains_power: true,
        };
        assert!(normal.has_anomaly(&config).is_none());

        // High temperature
        let hot = EnvironmentState {
            temperature: Some(60.0),
            ..Default::default()
        };
        assert!(hot.has_anomaly(&config).is_some());

        // Low temperature
        let cold = EnvironmentState {
            temperature: Some(-10.0),
            ..Default::default()
        };
        assert!(cold.has_anomaly(&config).is_some());

        // High acceleration (shock/tampering)
        let shock = EnvironmentState {
            acceleration: Some(5.0),
            ..Default::default()
        };
        assert!(shock.has_anomaly(&config).is_some());

        // Enclosure opened
        let opened = EnvironmentState {
            enclosure_opened: true,
            ..Default::default()
        };
        assert!(opened.has_anomaly(&config).is_some());
    }

    #[test]
    fn test_switch_state_encoding() {
        assert_eq!(
            SoftDeadMansSwitch::decode_state(SoftDeadMansSwitch::encode_state(SwitchState::Disarmed)),
            SwitchState::Disarmed
        );
        assert_eq!(
            SoftDeadMansSwitch::decode_state(SoftDeadMansSwitch::encode_state(SwitchState::Armed)),
            SwitchState::Armed
        );
        assert_eq!(
            SoftDeadMansSwitch::decode_state(SoftDeadMansSwitch::encode_state(SwitchState::GracePeriod)),
            SwitchState::GracePeriod
        );
        assert_eq!(
            SoftDeadMansSwitch::decode_state(SoftDeadMansSwitch::encode_state(SwitchState::Triggered)),
            SwitchState::Triggered
        );
    }

    #[test]
    fn test_arm_disarm() {
        let switch = SoftDeadMansSwitch::default_config();

        assert_eq!(switch.state(), SwitchState::Disarmed);
        assert!(!switch.armed.load(Ordering::SeqCst));

        // Arm
        switch.arm().unwrap();
        assert_eq!(switch.state(), SwitchState::Armed);
        assert!(switch.armed.load(Ordering::SeqCst));

        // Can't arm twice
        assert!(switch.arm().is_err());

        // Disarm
        switch.disarm().unwrap();
        assert_eq!(switch.state(), SwitchState::Disarmed);
        assert!(!switch.armed.load(Ordering::SeqCst));

        // Can't disarm twice
        assert!(switch.disarm().is_err());
    }

    #[test]
    fn test_heartbeat() {
        let switch = SoftDeadMansSwitch::default_config();

        // Heartbeat works when disarmed (no-op)
        switch.heartbeat().unwrap();

        // Arm and check heartbeat
        switch.arm().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let elapsed = switch.time_since_heartbeat();
        assert!(elapsed >= std::time::Duration::from_millis(10));

        // Send heartbeat
        switch.heartbeat().unwrap();
        let elapsed_after = switch.time_since_heartbeat();
        assert!(elapsed_after < elapsed);
    }

    #[test]
    fn test_emergency_callback() {
        let switch = SoftDeadMansSwitch::default_config();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        switch.on_emergency(Arc::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));

        // Callback should be registered but not called
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_is_healthy() {
        let switch = SoftDeadMansSwitch::default_config();

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
    async fn test_simulate_environment() {
        let switch = SoftDeadMansSwitch::default_config();

        let state = EnvironmentState {
            temperature: Some(30.0),
            acceleration: Some(0.2),
            enclosure_opened: false,
            network_connected: true,
            on_mains_power: true,
        };

        switch.simulate_environment(state.clone()).await;

        let retrieved = switch.get_environment();
        assert_eq!(retrieved.temperature, Some(30.0));
        assert_eq!(retrieved.on_mains_power, true);
    }
}
