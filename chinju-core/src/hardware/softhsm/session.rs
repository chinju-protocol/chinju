//! PKCS#11 session management

use cryptoki::context::{CInitializeArgs, Pkcs11};
use cryptoki::session::{Session, UserType};
use cryptoki::slot::Slot;
use cryptoki::types::AuthPin;
use std::sync::Arc;

use crate::hardware::traits::HardwareError;

/// PKCS#11 session wrapper
pub struct Pkcs11Session {
    pkcs11: Arc<Pkcs11>,
    slot: Slot,
    session: Session,
}

impl Pkcs11Session {
    /// Create a new PKCS#11 session
    pub fn new(module_path: &str, slot_id: u64, pin: &str) -> Result<Self, HardwareError> {
        // Load the PKCS#11 module
        let pkcs11 = Pkcs11::new(module_path).map_err(|e| {
            HardwareError::CommunicationError(format!("Failed to load PKCS#11 module: {}", e))
        })?;

        // Initialize the library
        pkcs11.initialize(CInitializeArgs::OsThreads).map_err(|e| {
            HardwareError::CommunicationError(format!("Failed to initialize PKCS#11: {}", e))
        })?;

        let pkcs11 = Arc::new(pkcs11);

        // Get the slot
        let slots = pkcs11.get_slots_with_token().map_err(|e| {
            HardwareError::CommunicationError(format!("Failed to get slots: {}", e))
        })?;

        let slot = slots
            .into_iter()
            .find(|s| s.id() == slot_id)
            .ok_or_else(|| HardwareError::SlotNotFound(format!("Slot {} not found", slot_id)))?;

        // Open a session
        let session = pkcs11.open_rw_session(slot).map_err(|e| {
            HardwareError::CommunicationError(format!("Failed to open session: {}", e))
        })?;

        // Login
        let auth_pin = AuthPin::new(pin.into());
        session
            .login(UserType::User, Some(&auth_pin))
            .map_err(|e| HardwareError::CommunicationError(format!("Failed to login: {}", e)))?;

        Ok(Self {
            pkcs11,
            slot,
            session,
        })
    }

    /// Get a reference to the session
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get the slot
    pub fn slot(&self) -> Slot {
        self.slot
    }

    /// Get the PKCS#11 context
    pub fn pkcs11(&self) -> &Arc<Pkcs11> {
        &self.pkcs11
    }

    /// Get slot info
    pub fn slot_info(&self) -> Result<String, HardwareError> {
        let info = self.pkcs11.get_slot_info(self.slot).map_err(|e| {
            HardwareError::CommunicationError(format!("Failed to get slot info: {}", e))
        })?;
        Ok(format!(
            "Slot: {:?}, Manufacturer: {:?}",
            info.slot_description(),
            info.manufacturer_id()
        ))
    }

    /// Get token info
    pub fn token_info(&self) -> Result<String, HardwareError> {
        let info = self.pkcs11.get_token_info(self.slot).map_err(|e| {
            HardwareError::CommunicationError(format!("Failed to get token info: {}", e))
        })?;
        Ok(format!(
            "Token: {:?}, Model: {:?}",
            info.label(),
            info.model()
        ))
    }
}

impl Drop for Pkcs11Session {
    fn drop(&mut self) {
        // Session will be closed automatically when dropped
        // Logout is also handled by the session drop
        tracing::debug!("PKCS#11 session closed");
    }
}

#[cfg(test)]
mod tests {
    // Tests require SoftHSM to be installed and configured
    // Run with: cargo test --features softhsm -- --ignored

    #[test]
    #[ignore]
    fn test_session_creation() {
        use super::*;

        let module_path = std::env::var("PKCS11_MODULE")
            .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".into());
        let slot = std::env::var("PKCS11_SLOT")
            .unwrap_or_else(|_| "0".into())
            .parse()
            .unwrap();
        let pin = std::env::var("PKCS11_PIN").unwrap_or_else(|_| "1234".into());

        let session = Pkcs11Session::new(&module_path, slot, &pin).unwrap();
        let slot_info = session.slot_info().unwrap();
        println!("Slot info: {}", slot_info);

        let token_info = session.token_info().unwrap();
        println!("Token info: {}", token_info);
    }
}
