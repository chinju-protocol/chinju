fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "../protocol/proto";
    
    // Configure tonic-build
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/gen")
        .compile_protos(
            &[
                // Common types
                format!("{}/chinju/common.proto", proto_root),
                format!("{}/chinju/credential.proto", proto_root),
                format!("{}/chinju/token.proto", proto_root),
                format!("{}/chinju/policy.proto", proto_root),
                // API services
                format!("{}/chinju/api/gateway_service.proto", proto_root),
                format!("{}/chinju/api/credential_service.proto", proto_root),
            ],
            &[proto_root],
        )?;

    println!("cargo:rerun-if-changed={}", proto_root);
    Ok(())
}
