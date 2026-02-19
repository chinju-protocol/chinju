//! C13 Model Containment Checker
//!
//! Handles containment-related checks:
//! - Dead Man's Switch state
//! - Extraction deterrent (rate limiting, pattern detection)
//! - Side-channel protection (timing guard)

use crate::config::ContainmentConfig;
use crate::error::ChinjuError;
use crate::ids::{RequestId, UserId};
use crate::services::extraction_deterrent::{compute_query_hash, ExtractionDeterrent};
use crate::services::side_channel::{SideChannelBlocker, TimingGuard};
use chinju_core::hardware::{DeadMansSwitch, SwitchState};
use std::sync::Arc;
use tonic::Status;
use tracing::{debug, error, warn};

/// Containment checker for C13 model safety
pub struct ContainmentChecker {
    config: ContainmentConfig,
    extraction_deterrent: Arc<ExtractionDeterrent>,
    side_channel_blocker: Arc<SideChannelBlocker>,
    dead_mans_switch: Arc<dyn DeadMansSwitch>,
}

impl ContainmentChecker {
    /// Create a new containment checker
    pub fn new(
        config: ContainmentConfig,
        extraction_deterrent: Arc<ExtractionDeterrent>,
        side_channel_blocker: Arc<SideChannelBlocker>,
        dead_mans_switch: Arc<dyn DeadMansSwitch>,
    ) -> Self {
        Self {
            config,
            extraction_deterrent,
            side_channel_blocker,
            dead_mans_switch,
        }
    }

    /// Check Dead Man's Switch state
    ///
    /// Returns an error if the switch is triggered and service should be unavailable.
    pub fn check_dead_mans_switch(&self, request_id: &RequestId) -> Result<(), Status> {
        if !self.config.enable_dead_mans_switch {
            return Ok(());
        }

        let switch_state = self.dead_mans_switch.state();
        match switch_state {
            SwitchState::Triggered => {
                error!(
                    request_id = %request_id,
                    "C13: Dead Man's Switch triggered - service unavailable"
                );
                Err(Status::unavailable(
                    "Service unavailable: safety mechanism triggered",
                ))
            }
            SwitchState::GracePeriod => {
                warn!(
                    request_id = %request_id,
                    "C13: Dead Man's Switch in grace period"
                );
                Ok(()) // Continue but log warning
            }
            _ => Ok(()),
        }
    }

    /// Send heartbeat to Dead Man's Switch
    pub fn send_heartbeat(&self) {
        if self.config.enable_dead_mans_switch {
            if let Err(e) = self.dead_mans_switch.heartbeat() {
                warn!("Failed to send heartbeat to Dead Man's Switch: {}", e);
            }
        }
    }

    /// Check extraction deterrent (rate limiting, pattern detection)
    ///
    /// Returns an error if the request should be blocked.
    pub fn check_extraction_deterrent(
        &self,
        user_id: &UserId,
        query_content: Option<&str>,
        request_id: &RequestId,
    ) -> Result<(), Status> {
        if !self.config.enable_extraction_deterrent {
            return Ok(());
        }

        let query_hash = query_content
            .map(|content| compute_query_hash(content))
            .unwrap_or(0);

        if let Err(e) = self
            .extraction_deterrent
            .check_query(user_id.as_str(), None, query_hash)
        {
            warn!(
                request_id = %request_id,
                user_id = %user_id,
                error = %e,
                "C13: Extraction deterrent blocked request"
            );
            return Err(Status::resource_exhausted(
                "Request blocked by extraction deterrent",
            ));
        }

        debug!(
            request_id = %request_id,
            user_id = %user_id,
            "C13: Extraction deterrent check passed"
        );
        Ok(())
    }

    /// Create timing guard for side-channel protection
    ///
    /// Returns a guard that normalizes response timing when dropped.
    pub fn create_timing_guard(&self) -> Option<TimingGuard<'_>> {
        if self.config.enable_side_channel_blocking {
            Some(TimingGuard::new(&self.side_channel_blocker))
        } else {
            None
        }
    }

    /// Get Dead Man's Switch state
    pub fn dead_mans_switch_state(&self) -> SwitchState {
        self.dead_mans_switch.state()
    }

    /// Check if Dead Man's Switch is healthy
    pub fn is_dead_mans_switch_healthy(&self) -> bool {
        self.dead_mans_switch.is_healthy()
    }

    /// Check if any containment feature is enabled
    pub fn any_enabled(&self) -> bool {
        self.config.any_enabled()
    }

    /// Get containment config
    pub fn config(&self) -> &ContainmentConfig {
        &self.config
    }

    /// Get extraction deterrent
    pub fn extraction_deterrent(&self) -> Arc<ExtractionDeterrent> {
        Arc::clone(&self.extraction_deterrent)
    }

    /// Get side channel blocker
    pub fn side_channel_blocker(&self) -> Arc<SideChannelBlocker> {
        Arc::clone(&self.side_channel_blocker)
    }

    /// Get Dead Man's Switch
    pub fn dead_mans_switch(&self) -> Arc<dyn DeadMansSwitch> {
        Arc::clone(&self.dead_mans_switch)
    }
}

/// Result of containment pre-flight checks
pub struct PreFlightCheckResult<'a> {
    /// Timing guard for response normalization
    pub timing_guard: Option<TimingGuard<'a>>,
}

impl ContainmentChecker {
    /// Perform all pre-flight containment checks
    ///
    /// This should be called at the start of request processing.
    pub fn pre_flight_check<'a>(
        &'a self,
        request_id: &str,
        user_id: &str,
        query_content: Option<&str>,
    ) -> Result<PreFlightCheckResult<'a>, Status> {
        let request_id = RequestId::new(request_id.to_string())
            .map_err(|e| Status::from(ChinjuError::from(e)))?;
        let user_id =
            UserId::new(user_id.to_string()).map_err(|e| Status::from(ChinjuError::from(e)))?;

        // Check Dead Man's Switch
        self.check_dead_mans_switch(&request_id)?;

        // Send heartbeat
        self.send_heartbeat();

        // Check extraction deterrent
        self.check_extraction_deterrent(&user_id, query_content, &request_id)?;

        // Create timing guard
        let timing_guard = self.create_timing_guard();

        Ok(PreFlightCheckResult { timing_guard })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ContainmentConfig;
    use crate::services::extraction_deterrent::ExtractionDeterrentConfig;
    use crate::services::side_channel::SideChannelConfig;
    use chinju_core::hardware::SoftDeadMansSwitch;

    fn create_test_checker() -> ContainmentChecker {
        let config = ContainmentConfig::disabled();
        let extraction_deterrent = Arc::new(ExtractionDeterrent::with_config(
            ExtractionDeterrentConfig::default(),
        ));
        let side_channel_blocker =
            Arc::new(SideChannelBlocker::with_config(SideChannelConfig::default()));
        let dead_mans_switch = Arc::new(SoftDeadMansSwitch::default());

        ContainmentChecker::new(
            config,
            extraction_deterrent,
            side_channel_blocker,
            dead_mans_switch,
        )
    }

    #[test]
    fn test_disabled_containment_passes() {
        let checker = create_test_checker();
        assert!(checker
            .check_dead_mans_switch(&RequestId::new("test-1").unwrap())
            .is_ok());
        assert!(checker
            .check_extraction_deterrent(
                &UserId::new("user-1").unwrap(),
                None,
                &RequestId::new("test-1").unwrap(),
            )
            .is_ok());
    }

    #[test]
    fn test_timing_guard_none_when_disabled() {
        let checker = create_test_checker();
        assert!(checker.create_timing_guard().is_none());
    }

    #[test]
    fn test_any_enabled() {
        let checker = create_test_checker();
        assert!(!checker.any_enabled());
    }
}
