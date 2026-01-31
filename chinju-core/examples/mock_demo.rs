//! Mock HSM Demo
//!
//! Demonstrates the CHINJU Protocol mock hardware components:
//! - MockHsm: Key generation, signing, verification
//! - MockOtp: Immutable storage (OTP/eFuse simulation)
//! - MockRandom: Random number generation
//!
//! Run with: cargo run --example mock_demo

use chinju_core::hardware::mock::{MockHsm, MockOtp, MockRandom};
use chinju_core::hardware::traits::*;

fn main() {
    println!("=== CHINJU Protocol Mock Demo ===\n");

    // Initialize tracing for better output
    tracing_subscriber_init();

    demo_hsm();
    demo_otp();
    demo_random();

    println!("\n=== Demo Complete ===");
}

fn tracing_subscriber_init() {
    // Simple init - in production would use tracing-subscriber
    // For now, just prints warnings from the mock modules
}

fn demo_hsm() {
    println!("--- MockHsm Demo ---");

    let hsm = MockHsm::new();

    // Check attestation
    let attestation = hsm.get_attestation().unwrap();
    println!("Trust Level: {:?}", attestation.trust_level);
    println!("Hardware Type: {}", attestation.hardware_type);
    println!("Hardware Backed: {}", hsm.is_hardware_backed());

    // Generate key pair
    let key_handle = hsm
        .generate_key_pair(KeyAlgorithm::Ed25519, "demo-key")
        .unwrap();
    println!("\nGenerated key: {:?}", key_handle);

    // Sign data
    let message = b"Hello, CHINJU Protocol!";
    let signature = hsm.sign(&key_handle, message).unwrap();
    println!("Signed message: {:?}", String::from_utf8_lossy(message));
    println!("Signature (first 16 bytes): {:02x?}", &signature.signature[..16]);

    // Verify signature
    let valid = hsm.verify(&key_handle, message, &signature).unwrap();
    println!("Signature valid: {}", valid);

    // Verify tampered message fails
    let tampered = b"Hello, HACKED Protocol!";
    let invalid = hsm.verify(&key_handle, tampered, &signature).unwrap();
    println!("Tampered message valid: {} (expected: false)", invalid);

    // Get public key
    let pubkey = hsm.get_public_key(&key_handle).unwrap();
    println!("Public key length: {} bytes", pubkey.len());

    // Secure erase
    hsm.secure_erase(&key_handle).unwrap();
    println!("Key securely erased");
    println!("Key still exists: {}", hsm.key_exists(&key_handle));

    println!();
}

fn demo_otp() {
    println!("--- MockOtp Demo ---");

    let otp = MockOtp::new();

    // Check attestation
    let attestation = otp.get_attestation().unwrap();
    println!("Trust Level: {:?}", attestation.trust_level);
    println!("Available slots: {}", otp.available_slots().unwrap());
    println!("Slot size: {} bytes", otp.slot_size());

    // Burn genesis hash to OTP
    let slot = SlotId::new("genesis-hash");
    let genesis_data = b"CHINJU_GENESIS_BLOCK_HASH_v1.0.0";

    println!("\nBurning genesis hash to slot '{}'", slot.0);
    otp.burn(&slot, genesis_data).unwrap();

    // Read back
    let read_data = otp.read(&slot).unwrap();
    println!(
        "Read back: {:?}",
        String::from_utf8_lossy(&read_data)
    );

    // Verify burned
    println!("Slot is burned: {}", otp.is_burned(&slot).unwrap());

    // Try to burn again (should fail)
    let result = otp.burn(&slot, b"different-data");
    println!(
        "Re-burn attempt: {}",
        if result.is_err() {
            "Failed (expected)"
        } else {
            "Succeeded (unexpected!)"
        }
    );

    // Demo fuse
    let fuse = FuseId::new("user-revoke-001");
    println!("\nBurning fuse '{}'", fuse.0);
    otp.burn_fuse(&fuse).unwrap();
    println!("Fuse blown: {}", otp.is_fuse_blown(&fuse).unwrap());

    println!();
}

fn demo_random() {
    println!("--- MockRandom Demo ---");

    let rng = MockRandom::new();

    // Check attestation
    let attestation = rng.get_attestation().unwrap();
    println!("Trust Level: {:?}", attestation.trust_level);
    println!("Hardware Backed: {}", rng.is_hardware_backed());

    // Generate random bytes
    let random1 = rng.generate(32).unwrap();
    let random2 = rng.generate(32).unwrap();

    println!("\nRandom 1 (first 8 bytes): {:02x?}", &random1[..8]);
    println!("Random 2 (first 8 bytes): {:02x?}", &random2[..8]);
    println!("Are they different: {}", random1 != random2);

    // Check entropy quality
    let quality = rng.entropy_quality().unwrap();
    println!("\nEntropy source: {}", quality.source_type);
    println!("Bits per sample: {}", quality.bits_per_sample);
    println!("NIST compliant: {}", quality.nist_compliant);

    // Health check
    println!("Health check passed: {}", rng.health_check().unwrap());

    println!();
}
