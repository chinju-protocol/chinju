//! CHINJU CLI Client
//!
//! Command-line tool for interacting with the CHINJU Sidecar.
//!
//! ## Usage
//!
//! ```bash
//! # Get AI status
//! cargo run --bin chinju-cli -- status
//!
//! # Send a request
//! cargo run --bin chinju-cli -- ask "What is CHINJU?"
//!
//! # Check health (HTTP)
//! cargo run --bin chinju-cli -- health
//!
//! # View metrics (HTTP)
//! cargo run --bin chinju-cli -- metrics
//!
//! # View audit logs
//! cargo run --bin chinju-cli -- audit [count]
//!
//! # Verify audit chain
//! cargo run --bin chinju-cli -- verify-chain
//!
//! # Manage genesis ceremony
//! cargo run --bin chinju-cli -- ceremony status
//! ```

use chinju_sidecar::gen::chinju::api::gateway::ai_gateway_service_client::AiGatewayServiceClient;
use chinju_sidecar::gen::chinju::api::gateway::*;
use chinju_core::hardware::threshold::ceremony::{Ceremony, CeremonyPhase};
use chinju_core::hardware::threshold::evidence::CeremonyEvidence;
use std::env;
use std::path::Path;
use tonic::Request;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];
    let server_addr = "http://[::1]:50051";
    let http_addr = "http://localhost:8080";

    match command.as_str() {
        "status" => {
            get_status(server_addr).await?;
        }
        "ask" => {
            if args.len() < 3 {
                eprintln!("Usage: chinju-cli ask <message>");
                return Ok(());
            }
            let message = args[2..].join(" ");
            send_request(server_addr, &message).await?;
        }
        "stream" => {
            if args.len() < 3 {
                eprintln!("Usage: chinju-cli stream <message>");
                return Ok(());
            }
            let message = args[2..].join(" ");
            stream_request(server_addr, &message).await?;
        }
        "validate" => {
            if args.len() < 3 {
                eprintln!("Usage: chinju-cli validate <message>");
                return Ok(());
            }
            let message = args[2..].join(" ");
            validate_request(server_addr, &message).await?;
        }
        "queue" => {
            get_queue_status(server_addr).await?;
        }
        "health" => {
            check_health(http_addr).await?;
        }
        "metrics" => {
            show_metrics(http_addr).await?;
        }
        "audit" => {
            let count = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            show_audit_logs(count).await?;
        }
        "verify-chain" => {
            verify_audit_chain().await?;
        }
        "ceremony" => {
            handle_ceremony(&args[2..]).await?;
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!(r#"
CHINJU CLI - CHINJU Protocol Command Line Interface

USAGE:
    chinju-cli <COMMAND> [OPTIONS]

COMMANDS:
    status          Get AI system status (tokens, state, health)
    ask             Send a request to the AI
    stream          Send a streaming request
    validate        Validate a request without processing
    queue           Get queue status
    health          Check HTTP health endpoint
    metrics         Show Prometheus metrics
    audit [N]       Show last N audit log entries (default: 10)
    verify-chain    Verify audit log hash chain integrity
    ceremony        Manage genesis ceremony (key generation)
    help            Show this help message

EXAMPLES:
    chinju-cli status
    chinju-cli ask "What is the meaning of life?"
    chinju-cli stream "Tell me a story"
    chinju-cli health
    chinju-cli metrics
    chinju-cli audit 20
    chinju-cli verify-chain
    chinju-cli ceremony status

SERVERS:
    gRPC: http://[::1]:50051
    HTTP: http://localhost:8080
"#);
}

fn print_ceremony_usage() {
    println!(r#"
CHINJU Ceremony CLI - Genesis Key Generation

USAGE:
    chinju-cli ceremony <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    init <t> <n> [id]   Initialize ceremony with threshold t and total n
    register <name>     Register a participant
    run                 Run key generation (Trusted Dealer mode)
    sign <hash>         Sign genesis hash
    status              Show current ceremony status
    reset               Reset ceremony (delete all data)

    # Phase 5: Evidence & Export
    export-shares       Export all key shares (for distribution)
    export-record       Export complete ceremony evidence (for preservation)
    verify-record <f>   Verify evidence file integrity

EXAMPLES:
    chinju-cli ceremony init 3 5 "genesis-v1"
    chinju-cli ceremony register "Alice"
    chinju-cli ceremony run
    chinju-cli ceremony sign "00000000000000000000000000000000"
    chinju-cli ceremony export-record
    chinju-cli ceremony verify-record evidence.json
"#);
}

async fn handle_ceremony(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        print_ceremony_usage();
        return Ok(());
    }

    let command = &args[0];
    let storage_path = "data/ceremony";
    std::fs::create_dir_all(storage_path)?;
    let ceremony_file = format!("{}/ceremony.json", storage_path);
    let key_store_path = format!("{}/keys", storage_path);

        // Helper to load or create ceremony
        let load_ceremony = || -> Result<Ceremony, Box<dyn std::error::Error>> {
            if Path::new(&ceremony_file).exists() {
                let record = chinju_core::hardware::threshold::ceremony::CeremonyRecord::load_from_file(&ceremony_file)?;
                Ok(Ceremony::from_record(record, &key_store_path))
            } else {
                Err("Ceremony not initialized. Run 'init' first.".into())
            }
        };

    match command.as_str() {
        "init" => {
            if args.len() < 3 {
                eprintln!("Usage: chinju-cli ceremony init <threshold> <total> [id]");
                return Ok(());
            }
            let threshold: u16 = args[1].parse()?;
            let total: u16 = args[2].parse()?;
            let id = args.get(3).cloned().unwrap_or_else(|| format!("genesis-{}", Uuid::new_v4()));

            if Path::new(&ceremony_file).exists() {
                eprintln!("Ceremony already exists. Use 'reset' to start over.");
                return Ok(());
            }

            let ceremony = Ceremony::with_storage(&id, threshold, total, &key_store_path)?;
            ceremony.record().save_to_file(&ceremony_file)?;
            println!("Ceremony '{}' initialized with threshold {}/{}", id, threshold, total);
        }
        "register" => {
            if args.len() < 2 {
                eprintln!("Usage: chinju-cli ceremony register <name>");
                return Ok(());
            }
            let name = &args[1];
            let mut ceremony = load_ceremony()?;
            
            if ceremony.phase() == CeremonyPhase::NotStarted {
                ceremony.start_registration()?;
            }
            
            let id = ceremony.register_participant(name)?;
            ceremony.record().save_to_file(&ceremony_file)?;
            println!("Participant '{}' registered with ID {}", name, id);
        }
        "run" => {
            let mut ceremony = load_ceremony()?;
            
            if ceremony.phase() == CeremonyPhase::Registration {
                ceremony.start_key_generation()?;
            }
            
            println!("Running Trusted Dealer Key Generation...");
            ceremony.run_trusted_dealer_keygen()?;
            ceremony.record().save_to_file(&ceremony_file)?;
            println!("Key generation completed successfully.");
            println!("Group Public Key generated.");
        }
            "sign" => {
                if args.len() < 2 {
                    eprintln!("Usage: chinju-cli ceremony sign <genesis_hash_hex>");
                    return Ok(());
                }
                let hash_hex = &args[1];
                let hash = hex::decode(hash_hex)?;
                
                let mut ceremony = load_ceremony()?;
                ceremony.set_genesis_hash(hash)?;
                
                println!("Restoring coordinator from key shares...");
                ceremony.restore_coordinator()?;
                
                println!("Signing genesis hash with threshold signature...");
                let signature = ceremony.sign_genesis()?;
                ceremony.complete()?;
                ceremony.record().save_to_file(&ceremony_file)?;
                
                println!("Genesis hash signed successfully.");
                println!("Signature: {}", hex::encode(signature));
                println!("Ceremony completed!");
            }
        "status" => {
            if !Path::new(&ceremony_file).exists() {
                println!("No ceremony initialized.");
                return Ok(());
            }
            let ceremony = load_ceremony()?;
            let record = ceremony.record();
            
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                   CHINJU Ceremony Status                     ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ ID: {:>48} ║", record.ceremony_id);
            println!("║ Phase: {:>45} ║", record.phase.to_string());
            println!("║ Threshold: {:>41} ║", format!("{}/{}", record.threshold, record.total));
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ Participants: {:>40} ║", record.participants.len());
            for p in &record.participants {
                println!("║   {}. {:<46} ║", p.id, p.name);
            }
            
            if let Some(pk) = &record.group_public_key {
                println!("╠══════════════════════════════════════════════════════════════╣");
                println!("║ Group Public Key:                                            ║");
                let pk_hex = hex::encode(pk);
                println!("║   {}...{} ║", &pk_hex[..20], &pk_hex[pk_hex.len()-20..]);
            }
            
            if let Some(sig) = &record.genesis_signature {
                println!("╠══════════════════════════════════════════════════════════════╣");
                println!("║ Genesis Signature:                                           ║");
                let sig_hex = hex::encode(sig);
                println!("║   {}...{} ║", &sig_hex[..20], &sig_hex[sig_hex.len()-20..]);
            }
            
            println!("╚══════════════════════════════════════════════════════════════╝");
        }
        "reset" => {
            if Path::new(storage_path).exists() {
                std::fs::remove_dir_all(storage_path)?;
                println!("Ceremony data deleted.");
            } else {
                println!("No data to delete.");
            }
        }
        "export-shares" => {
            let ceremony = load_ceremony()?;
            let key_share_ids = ceremony.key_share_ids();

            if key_share_ids.is_empty() {
                println!("No key shares available. Run key generation first.");
                return Ok(());
            }

            let export_dir = format!("{}/export", storage_path);
            std::fs::create_dir_all(&export_dir)?;

            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║               Exporting Key Shares                           ║");
            println!("╠══════════════════════════════════════════════════════════════╣");

            for id in &key_share_ids {
                if let Some(share) = ceremony.export_key_share(*id)? {
                    let filename = format!("{}/keyshare_{}.json", export_dir, id);
                    share.save_to_file(&filename)?;
                    println!("║ Exported key share {} → {}  ", id, filename);
                }
            }

            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ Total: {} key shares exported                                 ", key_share_ids.len());
            println!("║ Location: {}  ", export_dir);
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!("\n⚠️  WARNING: Distribute key shares securely to each participant!");
        }
        "export-record" => {
            let ceremony = load_ceremony()?;
            let evidence = ceremony.create_evidence()?;

            let evidence_file = format!("{}/evidence.json", storage_path);
            evidence.save_to_file(&evidence_file)?;

            let summary = evidence.summary();

            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║              Ceremony Evidence Exported                      ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ Ceremony ID: {:>49} ║", summary.ceremony_id);
            println!("║ Threshold: {:>51} ║", format!("{}/{}", summary.threshold, summary.total));
            println!("║ Phase: {:>55} ║", summary.phase);
            println!("║ Participants: {:>48} ║", summary.participant_count);
            println!("║ Genesis Signature: {:>43} ║", if summary.has_genesis_signature { "Yes" } else { "No" });
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ Record Hash:                                                 ║");
            println!("║   {:60} ║", &summary.record_hash[..60.min(summary.record_hash.len())]);
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ Saved to: {:52} ║", evidence_file);
            println!("╚══════════════════════════════════════════════════════════════╝");
        }
        "verify-record" => {
            if args.len() < 2 {
                eprintln!("Usage: chinju-cli ceremony verify-record <evidence_file>");
                return Ok(());
            }
            let evidence_file = &args[1];

            let evidence = CeremonyEvidence::load_from_file(evidence_file)?;
            let hash_valid = evidence.verify_record_hash()?;
            let summary = evidence.summary();

            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║              Ceremony Evidence Verification                  ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ File: {:56} ║", evidence_file);
            println!("║ Ceremony ID: {:>49} ║", summary.ceremony_id);
            println!("╠══════════════════════════════════════════════════════════════╣");

            if hash_valid {
                println!("║ ✓ Record Hash: VALID                                         ║");
            } else {
                println!("║ ✗ Record Hash: INVALID                                       ║");
            }

            println!("║ Genesis Signature: {:>43} ║", if summary.has_genesis_signature { "Present" } else { "Missing" });
            println!("║ Witnesses: {:>51} ║", summary.witness_count);
            println!("║ Hardware Attestations: {:>39} ║", if summary.has_hardware_attestation { "Present" } else { "None" });
            println!("║ Timestamp Proofs: {:>44} ║", if summary.has_timestamp_proof { "Present" } else { "None" });
            println!("╠══════════════════════════════════════════════════════════════╣");

            let complete = evidence.is_complete();
            if complete {
                println!("║ Overall Status: ✓ COMPLETE                                   ║");
            } else {
                println!("║ Overall Status: ⚠ INCOMPLETE                                 ║");
                println!("║   (Ceremony not completed or missing witnesses)              ║");
            }

            println!("╚══════════════════════════════════════════════════════════════╝");
        }
        _ => {
            print_ceremony_usage();
        }
    }
    
    Ok(())
}

async fn get_status(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to CHINJU Sidecar at {}...\n", addr);
    
    let mut client = AiGatewayServiceClient::connect(addr.to_string()).await?;
    
    let request = Request::new(GetAiStatusRequest {
        model: "mock".to_string(),
    });
    
    let response = client.get_ai_status(request).await?;
    let status = response.into_inner();
    
    println!("╔══════════════════════════════════════╗");
    println!("║       CHINJU AI System Status        ║");
    println!("╠══════════════════════════════════════╣");
    
    // Operating state
    let state_name = match status.state {
        0 => "UNSPECIFIED",
        1 => "BOOTSTRAPPING",
        2 => "ACTIVE",
        3 => "THROTTLED",
        4 => "SUSPENDED",
        5 => "HALTED",
        6 => "SHUTDOWN",
        _ => "UNKNOWN",
    };
    println!("║ State: {:>27} ║", state_name);
    
    // Token balance
    if let Some(balance) = &status.token_balance {
        let state_str = match balance.state {
            1 => "HEALTHY",
            2 => "LOW",
            3 => "CRITICAL",
            4 => "EXHAUSTED",
            _ => "UNKNOWN",
        };
        println!("╠══════════════════════════════════════╣");
        println!("║ Token Balance: {:>19} ║", balance.current_balance);
        println!("║ Total Consumed: {:>18} ║", balance.total_consumed);
        println!("║ Balance State: {:>19} ║", state_str);
    }
    
    // Health
    if let Some(health) = &status.health {
        println!("╠══════════════════════════════════════╣");
        let health_str = if health.healthy { "HEALTHY" } else { "UNHEALTHY" };
        println!("║ Health: {:>26} ║", health_str);
        if !health.issues.is_empty() {
            for issue in &health.issues {
                println!("║   - {} ║", issue);
            }
        }
    }
    
    // Limits
    if let Some(limits) = &status.limits {
        println!("╠══════════════════════════════════════╣");
        println!("║ Max Requests/sec: {:>16.1} ║", limits.max_requests_per_second);
        println!("║ Max Concurrent: {:>18} ║", limits.max_concurrent);
        println!("║ Streaming: {:>23} ║", if limits.streaming_allowed { "YES" } else { "NO" });
    }
    
    println!("╚══════════════════════════════════════╝");
    
    Ok(())
}

async fn send_request(addr: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to CHINJU Sidecar at {}...\n", addr);
    
    let mut client = AiGatewayServiceClient::connect(addr.to_string()).await?;
    
    let request = Request::new(ProcessRequestRequest {
        request_id: format!("req_{}", Uuid::new_v4()),
        credential: None, // Mock - no credential
        payload: Some(AiRequestPayload {
            model: "mock".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: message.to_string(),
                name: String::new(),
            }],
            parameters: None,
            system_prompt: String::new(),
        }),
        options: Some(RequestOptions {
            skip_audit: false,
            timeout_ms: 30000,
            priority: Priority::Normal.into(),
            force_policy: false,
            debug: true,
        }),
    });
    
    let response = client.process_request(request).await?;
    let resp = response.into_inner();
    
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    CHINJU Response                            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Response ID: {} ║", resp.response_id);
    
    if let Some(payload) = &resp.payload {
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Content:                                                     ║");
        for line in payload.content.lines() {
            println!("║   {}  ", line);
        }
        
        if let Some(usage) = &payload.usage {
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ Token Usage: {} prompt, {} completion, {} total",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
        }
    }
    
    if let Some(meta) = &resp.metadata {
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Processing Time: {}ms", meta.processing_time_ms);
        println!("║ CHINJU Tokens Consumed: {}", meta.chinju_tokens_consumed);
        println!("║ LPT Score: {:.2}", meta.lpt_score);
    }
    
    println!("╚══════════════════════════════════════════════════════════════╝");
    
    Ok(())
}

async fn stream_request(addr: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to CHINJU Sidecar at {}...\n", addr);
    
    let mut client = AiGatewayServiceClient::connect(addr.to_string()).await?;
    
    let request = Request::new(ProcessRequestRequest {
        request_id: format!("req_{}", Uuid::new_v4()),
        credential: None,
        payload: Some(AiRequestPayload {
            model: "mock".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: message.to_string(),
                name: String::new(),
            }],
            parameters: None,
            system_prompt: String::new(),
        }),
        options: None,
    });
    
    let response = client.process_request_stream(request).await?;
    let mut stream = response.into_inner();
    
    println!("╔══════════════════════════════════════╗");
    println!("║      CHINJU Streaming Response       ║");
    println!("╠══════════════════════════════════════╣");
    
    use tokio_stream::StreamExt;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => {
                if let Some(content) = c.chunk {
                    match content {
                        process_request_chunk::Chunk::Text(text) => {
                            print!("{}", text);
                        }
                        process_request_chunk::Chunk::FinalResponse(final_resp) => {
                            println!("\n╠══════════════════════════════════════╣");
                            println!("║ Stream Complete                      ║");
                            if let Some(meta) = &final_resp.metadata {
                                println!("║ Tokens Consumed: {:>18} ║", meta.chinju_tokens_consumed);
                            }
                        }
                        process_request_chunk::Chunk::Error(err) => {
                            eprintln!("\nError: {} - {}", err.code, err.message);
                        }
                        process_request_chunk::Chunk::Progress(prog) => {
                            println!("\n[Progress: {} tokens, {}ms]", prog.tokens_generated, prog.elapsed_ms);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Stream error: {}", e);
                break;
            }
        }
    }
    
    println!("╚══════════════════════════════════════╝");
    
    Ok(())
}

async fn validate_request(addr: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to CHINJU Sidecar at {}...\n", addr);
    
    let mut client = AiGatewayServiceClient::connect(addr.to_string()).await?;
    
    let request = Request::new(ValidateRequestRequest {
        payload: Some(AiRequestPayload {
            model: "mock".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: message.to_string(),
                name: String::new(),
            }],
            parameters: None,
            system_prompt: String::new(),
        }),
        credential: None,
    });
    
    let response = client.validate_request(request).await?;
    let resp = response.into_inner();
    
    println!("╔══════════════════════════════════════╗");
    println!("║      CHINJU Validation Result        ║");
    println!("╠══════════════════════════════════════╣");
    println!("║ Valid: {:>27} ║", if resp.valid { "YES" } else { "NO" });
    println!("║ Estimated Token Cost: {:>13} ║", resp.estimated_token_cost);
    println!("║ Estimated LPT Score: {:>14.2} ║", resp.estimated_lpt_score);
    
    if !resp.errors.is_empty() {
        println!("╠══════════════════════════════════════╣");
        println!("║ Validation Errors:                   ║");
        for err in &resp.errors {
            println!("║   - {}: {}  ", err.field, err.message);
        }
    }
    
    println!("╚══════════════════════════════════════╝");
    
    Ok(())
}

async fn get_queue_status(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to CHINJU Sidecar at {}...\n", addr);

    let mut client = AiGatewayServiceClient::connect(addr.to_string()).await?;

    let request = Request::new(GetQueueStatusRequest {
        model: "mock".to_string(),
    });

    let response = client.get_queue_status(request).await?;
    let status = response.into_inner();

    println!("╔══════════════════════════════════════╗");
    println!("║        CHINJU Queue Status           ║");
    println!("╠══════════════════════════════════════╣");
    println!("║ Pending Requests: {:>17} ║", status.pending_requests);
    println!("║ Processing: {:>23} ║", status.processing_requests);
    println!("║ Estimated Wait: {:>16}ms ║", status.estimated_wait_ms);
    println!("╚══════════════════════════════════════╝");

    Ok(())
}

async fn check_health(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking health at {}...\n", addr);

    let client = reqwest::Client::new();
    let response = client.get(format!("{}/health", addr)).send().await?;

    let status = response.status();
    let body = response.text().await?;

    println!("╔══════════════════════════════════════╗");
    println!("║         CHINJU Health Check          ║");
    println!("╠══════════════════════════════════════╣");
    println!("║ Status: {:>27} ║", status.as_u16());
    println!("║ Response: {:>25} ║", body.trim());
    println!("╚══════════════════════════════════════╝");

    Ok(())
}

async fn show_metrics(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching metrics from {}...\n", addr);

    let client = reqwest::Client::new();
    let response = client.get(format!("{}/metrics", addr)).send().await?;
    let body = response.text().await?;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    CHINJU Metrics                            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    for line in body.lines() {
        if line.starts_with('#') {
            continue; // Skip comments
        }
        if !line.is_empty() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                println!("║ {:40} {:>18} ║", parts[0], parts[1]);
            }
        }
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn show_audit_logs(count: usize) -> Result<(), Box<dyn std::error::Error>> {
    let audit_path = Path::new("data/audit/audit.jsonl");

    if !audit_path.exists() {
        eprintln!("Audit log file not found at: {}", audit_path.display());
        eprintln!("Make sure the sidecar has been running and has processed requests.");
        return Ok(());
    }

    println!("Reading audit logs from {}...\n", audit_path.display());

    let content = std::fs::read_to_string(audit_path)?;
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    CHINJU Audit Logs                         ║");
    println!("║ Total Entries: {:>47} ║", lines.len());
    println!("╠══════════════════════════════════════════════════════════════╣");

    // Show last N entries
    let start = if lines.len() > count { lines.len() - count } else { 0 };

    for (_i, line) in lines[start..].iter().enumerate() {
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            let seq = entry.get("sequence").and_then(|v| v.as_u64()).unwrap_or(0);
            let event_type = entry.get("event_type").and_then(|v| v.as_str()).unwrap_or("?");
            let timestamp = entry.get("timestamp").and_then(|v| v.as_str()).unwrap_or("?");
            let success = entry
                .get("result")
                .and_then(|r| r.get("success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            println!(
                "║ {:>4} │ {:12} │ {:>23} │ {} ║",
                seq,
                event_type,
                &timestamp[..timestamp.len().min(23)],
                if success { "✓" } else { "✗" }
            );
        }
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn verify_audit_chain() -> Result<(), Box<dyn std::error::Error>> {
    let audit_path = Path::new("data/audit/audit.jsonl");

    if !audit_path.exists() {
        eprintln!("Audit log file not found at: {}", audit_path.display());
        return Ok(());
    }

    println!("Verifying audit chain from {}...\n", audit_path.display());

    let content = std::fs::read_to_string(audit_path)?;
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║               CHINJU Audit Chain Verification                ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Total Entries: {:>47} ║", lines.len());

    let mut errors = Vec::new();
    let mut prev_hash: Option<String> = None;

    for (i, line) in lines.iter().enumerate() {
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            let seq = entry.get("sequence").and_then(|v| v.as_u64()).unwrap_or(0);
            let entry_prev_hash = entry
                .get("prev_hash")
                .and_then(|v| v.as_str())
                .map(String::from);
            let current_hash = entry
                .get("hash")
                .and_then(|v| v.as_str())
                .map(String::from);

            // Check sequence
            if seq != i as u64 {
                errors.push(format!("Sequence mismatch at entry {}: expected {}, got {}", i, i, seq));
            }

            // Check hash chain (skip first entry)
            if i > 0 {
                if let (Some(ref expected), Some(ref actual)) = (&prev_hash, &entry_prev_hash) {
                    if expected != actual {
                        errors.push(format!(
                            "Hash chain broken at entry {}: prev_hash mismatch",
                            i
                        ));
                    }
                }
            }

            prev_hash = current_hash;
        } else {
            errors.push(format!("Failed to parse entry {}", i));
        }
    }

    if errors.is_empty() {
        println!("║ Status: {:>54} ║", "✓ CHAIN VALID");
        println!("║ All {} entries verified successfully{:>26} ║", lines.len(), "");
    } else {
        println!("║ Status: {:>54} ║", "✗ CHAIN INVALID");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Errors Found: {:>48} ║", errors.len());
        for err in &errors[..errors.len().min(5)] {
            println!("║   - {:58} ║", err);
        }
        if errors.len() > 5 {
            println!("║   ... and {} more errors{:>35} ║", errors.len() - 5, "");
        }
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}
