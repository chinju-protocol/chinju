//! SoftHSM2 integration via PKCS#11
//!
//! This module provides L1 (Basic) security level by using SoftHSM2
//! through the PKCS#11 interface.
//!
//! # Setup
//!
//! 1. Install SoftHSM2:
//!    ```bash
//!    # Ubuntu/Debian
//!    sudo apt install softhsm2
//!
//!    # macOS
//!    brew install softhsm
//!    ```
//!
//! 2. Initialize a token:
//!    ```bash
//!    softhsm2-util --init-token --slot 0 --label "chinju" --pin 1234 --so-pin 12345678
//!    ```
//!
//! 3. Set environment variables:
//!    ```bash
//!    export PKCS11_MODULE=/usr/lib/softhsm/libsofthsm2.so
//!    export PKCS11_SLOT=0
//!    export PKCS11_PIN=1234
//!    ```

mod hsm;
mod session;

pub use hsm::SoftHsm;
pub use session::Pkcs11Session;
