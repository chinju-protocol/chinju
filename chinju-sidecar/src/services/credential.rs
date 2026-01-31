//! Human Credential Service Implementation
//!
//! Provides ZKP-based humanity verification per C12 specification.
//! Key features:
//! - Humanity proof verification (biometric, cognitive)
//! - Capability score (HCAL) assessment
//! - Degradation score validation (human finiteness)
//! - Consciousness continuity chain
//! - Hardware-backed signature generation (Phase 4.4)

use crate::gen::chinju::api::credential::credential_service_server::CredentialService as CredentialServiceTrait;
use crate::gen::chinju::api::credential::*;
use crate::gen::chinju::common::{
    Hash, HashAlgorithm, HardwareAttestation, Identifier, Signature, SignatureAlgorithm,
    Timestamp, TrustLevel, ValidityPeriod,
};
use crate::gen::chinju::credential::{
    CapabilityScore, CertificationLevel, CertificationStatus, ChainLink, ChainProof,
    CredentialState, CredentialStateTransition, DegradationScore, HumanCredential, HumanityProof,
    MeasurementContext, ProofType, RevocationReason, RevokedCredential,
};
use crate::services::capability_test::{CapabilityTestManager, ChallengeGenerator};
use crate::services::signature::SigningService;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Minimum capability score required for credential issuance
const MIN_CAPABILITY_SCORE: f64 = 0.3;

/// Minimum capability score for STANDARD certification
const STANDARD_THRESHOLD: f64 = 0.5;

/// Minimum capability score for ADVANCED certification
const ADVANCED_THRESHOLD: f64 = 0.7;

/// Minimum capability score for EXPERT certification
const EXPERT_THRESHOLD: f64 = 0.85;

/// Credential Service for human verification
#[derive(Clone)]
pub struct CredentialServiceImpl {
    /// Stored credentials (in production: database)
    credentials: Arc<RwLock<HashMap<String, StoredCredential>>>,
    /// Revocation list
    revoked: Arc<RwLock<Vec<RevokedCredential>>>,
    /// Pending credential requests
    pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
    /// Capability test manager (P5: HCAL)
    test_manager: Arc<CapabilityTestManager>,
    /// Active test sessions (session_id -> subject_id mapping)
    active_tests: Arc<RwLock<HashMap<String, String>>>,
    /// Signing service for hardware-backed signatures (Phase 4.4)
    signing_service: Option<Arc<RwLock<SigningService>>>,
}

/// Internal credential storage
#[derive(Clone)]
struct StoredCredential {
    credential: HumanCredential,
    state: CredentialState,
    certification: CertificationStatus,
}

/// Pending credential request
#[derive(Clone)]
#[allow(dead_code)]
struct PendingRequest {
    subject_id: Option<Identifier>,
    created_at: Timestamp,
    proof_submitted: bool,
}

impl CredentialServiceImpl {
    /// Create a new credential service (mock signing)
    pub fn new() -> Self {
        info!("Initializing CHINJU Credential Service with HCAL testing (mock signing)");
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            revoked: Arc::new(RwLock::new(Vec::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            test_manager: Arc::new(CapabilityTestManager::new()),
            active_tests: Arc::new(RwLock::new(HashMap::new())),
            signing_service: None,
        }
    }

    /// Create a new credential service with hardware-backed signing
    pub fn with_signing_service(signing_service: SigningService) -> Self {
        info!(
            "Initializing CHINJU Credential Service with hardware-backed signing (trust level: {:?})",
            signing_service.trust_level()
        );
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            revoked: Arc::new(RwLock::new(Vec::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            test_manager: Arc::new(CapabilityTestManager::new()),
            active_tests: Arc::new(RwLock::new(HashMap::new())),
            signing_service: Some(Arc::new(RwLock::new(signing_service))),
        }
    }

    /// Get current timestamp
    fn now() -> Option<Timestamp> {
        let now = chrono::Utc::now();
        Some(Timestamp {
            seconds: now.timestamp(),
            nanos: now.timestamp_subsec_nanos() as i32,
        })
    }

    /// Generate a mock ZKP verification result
    fn verify_zkp_proof(&self, proof: &HumanityProof) -> bool {
        // In production: actual ZKP verification
        // For mock: check that proof data exists
        !proof.zkp_data.is_empty()
    }

    /// Verify degradation score is within human range
    fn verify_degradation(&self, degradation: &DegradationScore) -> bool {
        // Humans exhibit fatigue and variance; AI doesn't
        degradation.fatigue > 0.0
            && degradation.fatigue <= 1.0
            && degradation.attention_decay > 0.0
            && degradation.response_variance > 0.05 // AI has near-zero variance
            && degradation.within_human_range
    }

    /// Calculate certification level from capability score
    fn calculate_certification_level(&self, score: f64) -> CertificationLevel {
        if score >= EXPERT_THRESHOLD {
            CertificationLevel::Expert
        } else if score >= ADVANCED_THRESHOLD {
            CertificationLevel::Advanced
        } else if score >= STANDARD_THRESHOLD {
            CertificationLevel::Standard
        } else if score >= MIN_CAPABILITY_SCORE {
            CertificationLevel::Basic
        } else {
            CertificationLevel::Unspecified
        }
    }

    /// Verify a credential's validity
    pub fn verify_credential_internal(
        &self,
        credential: &HumanCredential,
        options: &VerifyOptions,
    ) -> VerifyCredentialResponse {
        let mut errors = Vec::new();
        let mut signature_valid = false;
        let mut not_expired = false;
        let capability_score = credential
            .capability
            .as_ref()
            .map(|c| c.total)
            .unwrap_or(0.0);
        let chain_length = credential
            .chain_proof
            .as_ref()
            .map(|c| c.chain_length)
            .unwrap_or(0);

        // 1. Verify signature
        if let Some(sig) = &credential.issuer_signature {
            // In production: actual Ed25519 verification
            signature_valid = !sig.signature.is_empty() && !sig.public_key.is_empty();
            if !signature_valid {
                errors.push(VerifyError {
                    code: "INVALID_SIGNATURE".to_string(),
                    message: "Credential signature is invalid".to_string(),
                    field: "issuer_signature".to_string(),
                });
            }
        } else {
            errors.push(VerifyError {
                code: "MISSING_SIGNATURE".to_string(),
                message: "Credential signature is missing".to_string(),
                field: "issuer_signature".to_string(),
            });
        }

        // 2. Check expiry
        if let Some(validity) = &credential.validity {
            if let (Some(not_before), Some(not_after)) =
                (&validity.not_before, &validity.not_after)
            {
                let now = chrono::Utc::now().timestamp();
                not_expired = now >= not_before.seconds && now <= not_after.seconds;
                if !not_expired {
                    errors.push(VerifyError {
                        code: "EXPIRED".to_string(),
                        message: "Credential has expired".to_string(),
                        field: "validity".to_string(),
                    });
                }
            }
        }

        // 3. Check capability score
        if capability_score < options.min_capability_score {
            errors.push(VerifyError {
                code: "LOW_CAPABILITY".to_string(),
                message: format!(
                    "Capability score {} below required {}",
                    capability_score, options.min_capability_score
                ),
                field: "capability".to_string(),
            });
        }

        // 4. Check chain length
        if chain_length < options.min_chain_length {
            errors.push(VerifyError {
                code: "SHORT_CHAIN".to_string(),
                message: format!(
                    "Chain length {} below required {}",
                    chain_length, options.min_chain_length
                ),
                field: "chain_proof".to_string(),
            });
        }

        // 5. Check hardware attestation if required
        let trust_level = if options.require_hardware_attestation {
            if let Some(attestation) = &credential.attestation {
                attestation.trust_level()
            } else {
                errors.push(VerifyError {
                    code: "MISSING_ATTESTATION".to_string(),
                    message: "Hardware attestation required but not present".to_string(),
                    field: "attestation".to_string(),
                });
                TrustLevel::Mock
            }
        } else {
            credential
                .attestation
                .as_ref()
                .map(|a| a.trust_level())
                .unwrap_or(TrustLevel::Mock)
        };

        VerifyCredentialResponse {
            valid: errors.is_empty(),
            errors,
            details: Some(VerifyDetails {
                signature_valid,
                not_expired,
                not_revoked: true, // Would check revocation list in production
                capability_score,
                chain_length,
                trust_level: trust_level.into(),
            }),
        }
    }

    /// Generate signature for credential data
    /// Uses hardware-backed signing if available, otherwise falls back to mock
    async fn sign_credential_data(&self, data: &[u8]) -> (Signature, Option<HardwareAttestation>) {
        let now = Self::now();

        if let Some(ref signing_service) = self.signing_service {
            let mut svc = signing_service.write().await;

            // Ensure issuer key exists
            if let Err(e) = svc.ensure_issuer_key().await {
                warn!("Failed to ensure issuer key: {}, falling back to mock", e);
                return self.mock_signature(now);
            }

            // Sign the data
            match svc.sign_as_issuer(data).await {
                Ok(sig) => {
                    let attestation = svc.get_attestation_proto().ok();
                    debug!(
                        key_id = %svc.issuer_key_id(),
                        trust_level = ?svc.trust_level(),
                        "Signed credential with hardware-backed key"
                    );
                    (sig, attestation)
                }
                Err(e) => {
                    warn!("Hardware signing failed: {}, falling back to mock", e);
                    self.mock_signature(now)
                }
            }
        } else {
            self.mock_signature(now)
        }
    }

    /// Generate a mock signature (for development/testing)
    fn mock_signature(&self, now: Option<Timestamp>) -> (Signature, Option<HardwareAttestation>) {
        (
            Signature {
                algorithm: SignatureAlgorithm::Ed25519.into(),
                public_key: vec![0u8; 32],
                signature: vec![0u8; 64],
                signed_at: now,
                key_id: "chinju-mock-key-001".to_string(),
            },
            None,
        )
    }

    /// Get hardware attestation if signing service is available
    pub async fn get_attestation(&self) -> Option<HardwareAttestation> {
        if let Some(ref signing_service) = self.signing_service {
            let svc = signing_service.read().await;
            svc.get_attestation_proto().ok()
        } else {
            None
        }
    }
}

impl Default for CredentialServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl CredentialServiceTrait for CredentialServiceImpl {
    async fn request_credential(
        &self,
        request: Request<RequestCredentialRequest>,
    ) -> Result<Response<RequestCredentialResponse>, Status> {
        let req = request.into_inner();
        let request_id = format!("req_{}", Uuid::new_v4());

        info!(request_id = %request_id, "New credential request");

        // Store pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(
                request_id.clone(),
                PendingRequest {
                    subject_id: req.subject_id,
                    created_at: Self::now().unwrap_or_default(),
                    proof_submitted: false,
                },
            );
        }

        // Expire in 1 hour
        let expires_at = {
            let now = chrono::Utc::now() + chrono::Duration::hours(1);
            Some(Timestamp {
                seconds: now.timestamp(),
                nanos: 0,
            })
        };

        Ok(Response::new(RequestCredentialResponse {
            request_id,
            next_step: NextStep::SubmitProof.into(),
            available_proof_types: vec![
                ProofType::BiometricResponse.into(),
                ProofType::CognitivePattern.into(),
                ProofType::Composite.into(),
            ],
            expires_at,
        }))
    }

    async fn submit_proof(
        &self,
        request: Request<SubmitProofRequest>,
    ) -> Result<Response<SubmitProofResponse>, Status> {
        let req = request.into_inner();
        let request_id = req.request_id.clone();

        info!(request_id = %request_id, "Processing humanity proof");

        // Check pending request exists
        let pending = {
            let pending = self.pending_requests.read().await;
            pending.get(&request_id).cloned()
        };

        let pending = pending.ok_or_else(|| Status::not_found("Request not found or expired"))?;

        // Verify the proof
        let proof = req
            .proof
            .ok_or_else(|| Status::invalid_argument("Proof is required"))?;

        if !self.verify_zkp_proof(&proof) {
            return Ok(Response::new(SubmitProofResponse {
                accepted: false,
                message: "Proof verification failed".to_string(),
                credential: None,
                next_step: NextStep::SubmitProof.into(),
            }));
        }

        // Verify degradation (human finiteness)
        if let Some(ref degradation) = proof.degradation {
            if !self.verify_degradation(degradation) {
                warn!(request_id = %request_id, "AI-like behavior detected in degradation score");
                return Ok(Response::new(SubmitProofResponse {
                    accepted: false,
                    message: "Degradation score indicates non-human behavior".to_string(),
                    credential: None,
                    next_step: NextStep::SubmitProof.into(),
                }));
            }
        }

        // Generate credential
        let credential_id = format!("cred_{}", Uuid::new_v4());
        let now = Self::now();
        let expires = {
            let future = chrono::Utc::now() + chrono::Duration::days(30);
            Some(Timestamp {
                seconds: future.timestamp(),
                nanos: 0,
            })
        };

        // Mock capability score (in production: from capability test)
        let capability = Some(CapabilityScore {
            independence: 0.7,
            detection: 0.65,
            alternatives: 0.6,
            critique: 0.55,
            total: 0.625,
            context: Some(MeasurementContext {
                measured_at: now.clone(),
                environment_id: "chinju-sidecar-tee-001".to_string(),
                attestation: None,
                test_version: "hcal-v1.0".to_string(),
            }),
        });

        let certification_level = self.calculate_certification_level(0.625);

        // Create data to sign (credential identity + timestamp)
        let sign_data = format!(
            "{}:{}:{}",
            credential_id,
            pending
                .subject_id
                .as_ref()
                .map(|id| id.id.as_str())
                .unwrap_or("anonymous"),
            now.as_ref().map(|t| t.seconds).unwrap_or(0)
        );

        // Sign the credential data using hardware-backed signing if available
        let (issuer_signature, attestation) = self.sign_credential_data(sign_data.as_bytes()).await;

        let credential = HumanCredential {
            subject_id: pending.subject_id.clone(),
            issuer_id: Some(Identifier {
                namespace: "issuer".to_string(),
                id: "chinju-sidecar-001".to_string(),
                version: 1,
            }),
            issued_at: now.clone(),
            validity: Some(ValidityPeriod {
                not_before: now.clone(),
                not_after: expires,
            }),
            humanity_proof: Some(proof),
            capability: capability.clone(),
            chain_proof: Some(ChainProof {
                previous_hash: None, // First credential
                chain_length: 1,
                recent_links: vec![],
            }),
            issuer_signature: Some(issuer_signature),
            attestation, // Hardware attestation if available
            version: 1,
        };

        // Store credential
        {
            let mut creds = self.credentials.write().await;
            creds.insert(
                credential_id.clone(),
                StoredCredential {
                    credential: credential.clone(),
                    state: CredentialState::Verified,
                    certification: CertificationStatus {
                        state: CredentialState::Verified.into(),
                        level: certification_level.into(),
                        expires_at: credential.validity.as_ref().and_then(|v| v.not_after.clone()),
                        conditions: vec![],
                    },
                },
            );
        }

        // Remove pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.remove(&request_id);
        }

        info!(
            credential_id = %credential_id,
            certification_level = ?certification_level,
            "Credential issued successfully"
        );

        Ok(Response::new(SubmitProofResponse {
            accepted: true,
            message: "Credential issued successfully".to_string(),
            credential: Some(credential),
            next_step: NextStep::Unspecified.into(),
        }))
    }

    async fn get_credential(
        &self,
        request: Request<GetCredentialRequest>,
    ) -> Result<Response<GetCredentialResponse>, Status> {
        let req = request.into_inner();
        let cred_id = req
            .credential_id
            .as_ref()
            .map(|id| id.id.clone())
            .ok_or_else(|| Status::invalid_argument("Credential ID required"))?;

        let creds = self.credentials.read().await;
        let stored = creds
            .get(&cred_id)
            .ok_or_else(|| Status::not_found("Credential not found"))?;

        Ok(Response::new(GetCredentialResponse {
            credential: Some(stored.credential.clone()),
            state: stored.state.into(),
            certification: Some(stored.certification.clone()),
        }))
    }

    async fn verify_credential(
        &self,
        request: Request<VerifyCredentialRequest>,
    ) -> Result<Response<VerifyCredentialResponse>, Status> {
        let req = request.into_inner();
        let credential = req
            .credential
            .ok_or_else(|| Status::invalid_argument("Credential required"))?;
        let options = req.options.unwrap_or_default();

        let response = self.verify_credential_internal(&credential, &options);
        Ok(Response::new(response))
    }

    async fn renew_credential(
        &self,
        request: Request<RenewCredentialRequest>,
    ) -> Result<Response<RenewCredentialResponse>, Status> {
        let req = request.into_inner();
        let cred_id = req
            .credential_id
            .as_ref()
            .map(|id| id.id.clone())
            .ok_or_else(|| Status::invalid_argument("Credential ID required"))?;

        info!(credential_id = %cred_id, "Renewing credential");

        // Get existing credential
        let existing = {
            let creds = self.credentials.read().await;
            creds.get(&cred_id).cloned()
        };

        let existing =
            existing.ok_or_else(|| Status::not_found("Credential not found for renewal"))?;

        // Verify new proof if provided
        if let Some(ref new_proof) = req.new_proof {
            if !self.verify_zkp_proof(new_proof) {
                return Err(Status::invalid_argument("New proof verification failed"));
            }
        }

        // Create renewed credential with extended validity
        let now = Self::now();
        let expires = {
            let future = chrono::Utc::now() + chrono::Duration::days(30);
            Some(Timestamp {
                seconds: future.timestamp(),
                nanos: 0,
            })
        };

        let mut renewed = existing.credential.clone();
        renewed.issued_at = now.clone();
        renewed.validity = Some(ValidityPeriod {
            not_before: now.clone(),
            not_after: expires,
        });
        renewed.version += 1;

        // Update chain
        if let Some(ref mut chain) = renewed.chain_proof {
            // Add previous credential hash to chain
            let prev_hash = Hash {
                algorithm: HashAlgorithm::Sha3256.into(),
                value: vec![0u8; 32], // Mock hash
            };
            chain.previous_hash = Some(prev_hash.clone());
            chain.chain_length += 1;
            chain.recent_links.push(ChainLink {
                credential_hash: Some(prev_hash),
                issued_at: existing.credential.issued_at,
                signature: existing.credential.issuer_signature,
            });
            // Keep only last 10 links
            if chain.recent_links.len() > 10 {
                chain.recent_links.remove(0);
            }
        }

        // Store renewed credential
        let new_id = format!("cred_{}", Uuid::new_v4());
        {
            let mut creds = self.credentials.write().await;
            creds.insert(
                new_id.clone(),
                StoredCredential {
                    credential: renewed.clone(),
                    state: CredentialState::Verified,
                    certification: existing.certification.clone(),
                },
            );
        }

        info!(
            old_id = %cred_id,
            new_id = %new_id,
            chain_length = ?renewed.chain_proof.as_ref().map(|c| c.chain_length),
            "Credential renewed"
        );

        Ok(Response::new(RenewCredentialResponse {
            renewed_credential: Some(renewed),
        }))
    }

    async fn revoke_credential(
        &self,
        request: Request<RevokeCredentialRequest>,
    ) -> Result<Response<RevokeCredentialResponse>, Status> {
        let req = request.into_inner();
        let cred_id = req
            .credential_id
            .as_ref()
            .map(|id| id.id.clone())
            .ok_or_else(|| Status::invalid_argument("Credential ID required"))?;

        warn!(
            credential_id = %cred_id,
            reason = %req.reason,
            "IRREVERSIBLE: Revoking credential"
        );

        // Verify threshold signature (in production)
        if req.authorization.is_none() {
            return Err(Status::permission_denied(
                "Threshold signature required for revocation",
            ));
        }

        let now = Self::now();

        // Remove from active credentials
        {
            let mut creds = self.credentials.write().await;
            creds.remove(&cred_id);
        }

        // Add to revocation list
        let revoked_record = RevokedCredential {
            credential_id: req.credential_id,
            revoked_at: now.clone(),
            reason: RevocationReason::RevocationManual.into(),
            details: req.reason,
            hardware_proof: Some(Hash {
                algorithm: HashAlgorithm::Sha3256.into(),
                value: vec![0u8; 32], // Mock eFuse burn proof
            }),
        };

        {
            let mut revoked = self.revoked.write().await;
            revoked.push(revoked_record);
        }

        Ok(Response::new(RevokeCredentialResponse {
            success: true,
            revoked_at: now,
            hardware_proof: Some(Hash {
                algorithm: HashAlgorithm::Sha3256.into(),
                value: vec![0u8; 32],
            }),
        }))
    }

    type StartCapabilityTestStream =
        tokio_stream::wrappers::ReceiverStream<Result<CapabilityTestChallenge, Status>>;

    async fn start_capability_test(
        &self,
        request: Request<StartCapabilityTestRequest>,
    ) -> Result<Response<Self::StartCapabilityTestStream>, Status> {
        let req = request.into_inner();
        let subject_id = req
            .subject_id
            .as_ref()
            .map(|id| id.id.clone())
            .unwrap_or_else(|| format!("anon_{}", Uuid::new_v4()));

        info!(subject = %subject_id, test_type = ?req.test_type(), "Starting HCAL capability test");

        // Create a new test session using the capability test manager
        let session = self.test_manager.start_session(&subject_id).await;
        let session_id = session.session_id.clone();

        // Store the session mapping
        {
            let mut active = self.active_tests.write().await;
            active.insert(session_id.clone(), subject_id.clone());
        }

        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let generator = ChallengeGenerator::new();

        // Get challenges from session
        let challenges = session.challenges;
        let total = challenges.len() as i32;

        // Spawn task to send challenges
        tokio::spawn(async move {
            for (seq, (challenge_id, challenge_type, challenge_data)) in challenges.into_iter().enumerate() {
                let proto_challenge = generator.to_proto_challenge(
                    &challenge_id,
                    challenge_type,
                    &challenge_data,
                    (seq + 1) as i32,
                    total,
                );

                debug!(
                    challenge_id = %proto_challenge.challenge_id,
                    challenge_type = ?challenge_type,
                    sequence = seq + 1,
                    "Sending challenge"
                );

                if tx.send(Ok(proto_challenge)).await.is_err() {
                    warn!("Client disconnected during capability test");
                    break;
                }

                // Small delay between challenges
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            info!(session_id = %session_id, "All challenges sent");
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    async fn submit_test_response(
        &self,
        request: Request<tonic::Streaming<TestResponse>>,
    ) -> Result<Response<TestResult>, Status> {
        let mut stream = request.into_inner();
        let mut responses: Vec<TestResponse> = Vec::new();
        let mut session_id: Option<String> = None;

        // Collect all responses
        while let Some(response) = stream.message().await? {
            debug!(
                challenge_id = %response.challenge_id,
                response_time_ms = response.response_time_ms,
                "Received test response"
            );

            // Try to find session from challenge_id
            if session_id.is_none() {
                // Look up session by iterating active tests
                let active = self.active_tests.read().await;
                for (sid, _) in active.iter() {
                    if let Some(sess) = self.test_manager.get_session(sid).await {
                        if sess.challenges.iter().any(|(cid, _, _)| *cid == response.challenge_id) {
                            session_id = Some(sid.clone());
                            break;
                        }
                    }
                }
            }

            // Record the response if we have a session
            if let Some(ref sid) = session_id {
                self.test_manager
                    .record_response(sid, &response.challenge_id, &response)
                    .await;
            }

            responses.push(response);
        }

        info!(
            response_count = responses.len(),
            session_id = ?session_id,
            "Processing HCAL test responses"
        );

        // Complete the session and get result
        let result = if let Some(ref sid) = session_id {
            // Clean up active test mapping
            {
                let mut active = self.active_tests.write().await;
                active.remove(sid);
            }

            self.test_manager.complete_session(sid).await
        } else {
            None
        };

        // Return the result or generate a fallback
        match result {
            Some(test_result) => {
                info!(
                    passed = test_result.passed,
                    score = ?test_result.score.as_ref().map(|s| s.total),
                    "HCAL test completed"
                );
                Ok(Response::new(test_result))
            }
            None => {
                // Fallback: calculate basic scores from responses without session
                warn!("No session found, calculating fallback scores");

                let total_time_ms: u32 = responses.iter().map(|r| r.response_time_ms).sum();
                let response_count = responses.len() as f64;
                let avg_time = if response_count > 0.0 {
                    total_time_ms as f64 / response_count
                } else {
                    0.0
                };

                // Human-like response time variance contributes to score
                let time_variance_bonus = if avg_time > 1000.0 && avg_time < 30000.0 {
                    0.1
                } else {
                    0.0
                };

                let score = CapabilityScore {
                    independence: 0.5 + time_variance_bonus,
                    detection: 0.5,
                    alternatives: 0.5,
                    critique: 0.5,
                    total: 0.5 + time_variance_bonus,
                    context: Some(MeasurementContext {
                        measured_at: Self::now(),
                        environment_id: "chinju-hcal-fallback".to_string(),
                        attestation: None,
                        test_version: "hcal-v1.0".to_string(),
                    }),
                };

                let level = self.calculate_certification_level(score.total);
                let passed = score.total >= MIN_CAPABILITY_SCORE;

                Ok(Response::new(TestResult {
                    score: Some(score),
                    passed,
                    feedback: if passed {
                        format!("Test passed (fallback evaluation). Certification level: {:?}", level)
                    } else {
                        "Test failed. Score below minimum threshold.".to_string()
                    },
                    achieved_level: level.into(),
                }))
            }
        }
    }

    async fn get_revocation_list(
        &self,
        request: Request<GetRevocationListRequest>,
    ) -> Result<Response<GetRevocationListResponse>, Status> {
        let req = request.into_inner();
        let revoked = self.revoked.read().await;

        let filtered: Vec<_> = if let Some(since) = req.since {
            revoked
                .iter()
                .filter(|r| {
                    r.revoked_at
                        .as_ref()
                        .map(|t| t.seconds >= since.seconds)
                        .unwrap_or(false)
                })
                .take(req.limit as usize)
                .cloned()
                .collect()
        } else {
            revoked.iter().take(req.limit as usize).cloned().collect()
        };

        Ok(Response::new(GetRevocationListResponse {
            revoked: filtered,
            as_of: Self::now(),
            next_page_token: String::new(),
        }))
    }

    type WatchCredentialStateStream =
        tokio_stream::wrappers::ReceiverStream<Result<CredentialStateTransition, Status>>;

    async fn watch_credential_state(
        &self,
        request: Request<WatchCredentialStateRequest>,
    ) -> Result<Response<Self::WatchCredentialStateStream>, Status> {
        let _req = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(10);

        // In production: watch for state changes
        // For now: just return empty stream
        drop(tx);

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }
}
