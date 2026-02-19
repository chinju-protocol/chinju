//! Nitro Enclave Communication Test
//!
//! This example tests vsock communication with a running Nitro Enclave.
//!
//! # Prerequisites
//!
//! 1. Start the Enclave:
//!    ```bash
//!    nitro-cli run-enclave --eif-path chinju-enclave.eif --memory 512 --cpu-count 2
//!    ```
//!
//! 2. Get the Enclave CID:
//!    ```bash
//!    nitro-cli describe-enclaves | jq '.[0].EnclaveCID'
//!    ```
//!
//! 3. Run this test:
//!    ```bash
//!    CHINJU_NITRO_ENCLAVE_CID=16 cargo run --example nitro_test --features nitro
//!    ```

#[cfg(all(feature = "nitro", target_os = "linux"))]
use chinju_core::hardware::nitro::{NitroEnclaveClient, VsockConfig};

#[cfg(all(feature = "nitro", target_os = "linux"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,chinju_core=debug")
        .init();

    println!("===========================================");
    println!(" CHINJU Nitro Enclave Communication Test");
    println!("===========================================\n");

    // Get CID from environment or use default
    let cid: u32 = std::env::var("CHINJU_NITRO_ENCLAVE_CID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(16);

    let port: u32 = std::env::var("CHINJU_NITRO_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5000);

    println!("Configuration:");
    println!("  Enclave CID: {}", cid);
    println!("  vsock Port:  {}", port);
    println!();

    let config = VsockConfig::new(cid, port).with_timeout(10000);
    let client = NitroEnclaveClient::new(config)?;

    // Test 1: Health Check
    println!("Test 1: Health Check");
    println!("-----------------------------------------");
    match client.health_check() {
        Ok((healthy, version, uptime)) => {
            println!("  ✅ Healthy: {}", healthy);
            println!("  ✅ Version: {}", version);
            println!("  ✅ Uptime:  {} seconds", uptime);
        }
        Err(e) => {
            println!("  ❌ Failed: {}", e);
            return Err(e.into());
        }
    }
    println!();

    // Test 2: Heartbeat
    println!("Test 2: Heartbeat");
    println!("-----------------------------------------");
    match client.heartbeat() {
        Ok(timestamp) => {
            println!("  ✅ Timestamp: {}", timestamp);
        }
        Err(e) => {
            println!("  ❌ Failed: {}", e);
        }
    }
    println!();

    // Test 3: Get Status
    println!("Test 3: Get Status");
    println!("-----------------------------------------");
    match client.get_status() {
        Ok(status) => {
            println!("  ✅ Version:     {}", status.version);
            println!("  ✅ Key Count:   {}", status.key_count);
            println!("  ✅ Memory Used: {} bytes", status.memory_used);
            println!("  ✅ Uptime:      {} seconds", status.uptime_seconds);
        }
        Err(e) => {
            println!("  ❌ Failed: {}", e);
        }
    }
    println!();

    // Test 4: Generate Key Pair
    println!("Test 4: Generate Key Pair (Ed25519)");
    println!("-----------------------------------------");
    let key_id = match client.generate_key_pair("Ed25519", "test-key") {
        Ok((key_id, public_key)) => {
            println!("  ✅ Key ID:     {}", key_id);
            println!("  ✅ Public Key: {} bytes", public_key.len());
            println!("     {:?}", &public_key[..8.min(public_key.len())]);
            Some(key_id)
        }
        Err(e) => {
            println!("  ❌ Failed: {}", e);
            None
        }
    };
    println!();

    // Test 5: Sign Data
    if let Some(ref key_id) = key_id {
        println!("Test 5: Sign Data");
        println!("-----------------------------------------");
        let data = b"Hello, CHINJU Protocol!";
        match client.sign(key_id, data) {
            Ok((signature, public_key)) => {
                println!("  ✅ Signature:  {} bytes", signature.len());
                println!("  ✅ Public Key: {} bytes", public_key.len());
            }
            Err(e) => {
                println!("  ❌ Failed: {}", e);
            }
        }
        println!();
    }

    // Test 6: Get Public Key
    if let Some(ref key_id) = key_id {
        println!("Test 6: Get Public Key");
        println!("-----------------------------------------");
        match client.get_public_key(key_id) {
            Ok(public_key) => {
                println!("  ✅ Public Key: {} bytes", public_key.len());
            }
            Err(e) => {
                println!("  ❌ Failed: {}", e);
            }
        }
        println!();
    }

    // Test 7: Seal/Unseal
    println!("Test 7: Seal/Unseal Data");
    println!("-----------------------------------------");
    let plaintext = b"Secret data to seal";
    match client.seal(plaintext) {
        Ok(sealed_data) => {
            println!(
                "  ✅ Sealed:   {} bytes -> {} bytes",
                plaintext.len(),
                sealed_data.len()
            );

            match client.unseal(&sealed_data) {
                Ok(unsealed) => {
                    if unsealed == plaintext {
                        println!("  ✅ Unsealed: Data matches original");
                    } else {
                        println!("  ❌ Unsealed: Data mismatch!");
                    }
                }
                Err(e) => {
                    println!("  ❌ Unseal Failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("  ❌ Seal Failed: {}", e);
        }
    }
    println!();

    // Test 8: Get Attestation
    println!("Test 8: Get Attestation Document");
    println!("-----------------------------------------");
    let challenge = b"random_challenge_bytes_123456789";
    match client.get_attestation(challenge, Some(b"user_data".to_vec())) {
        Ok(document) => {
            println!("  ✅ Attestation Document: {} bytes", document.len());
            // In production, verify this document
        }
        Err(e) => {
            println!("  ⚠️  Attestation: {} (expected in debug mode)", e);
        }
    }
    println!();

    // Test 9: Delete Key
    if let Some(ref key_id) = key_id {
        println!("Test 9: Delete Key");
        println!("-----------------------------------------");
        match client.delete_key(key_id) {
            Ok(()) => {
                println!("  ✅ Key deleted: {}", key_id);
            }
            Err(e) => {
                println!("  ❌ Failed: {}", e);
            }
        }
        println!();
    }

    println!("===========================================");
    println!(" All tests completed!");
    println!("===========================================");

    Ok(())
}

#[cfg(not(all(feature = "nitro", target_os = "linux")))]
fn main() {
    eprintln!("This example requires the 'nitro' feature and Linux.");
    eprintln!("Run with: cargo run --example nitro_test --features nitro");
    std::process::exit(1);
}
