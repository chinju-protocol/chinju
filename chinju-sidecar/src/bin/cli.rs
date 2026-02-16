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
//!
//! # C14-C17 Services
//! cargo run --bin chinju-cli -- value-neuron summary model-1
//! cargo run --bin chinju-cli -- capability summary session-1
//! cargo run --bin chinju-cli -- contradiction status session-1
//! cargo run --bin chinju-cli -- survival score "text to analyze"
//! ```

use chinju_sidecar::gen::chinju::api::gateway::ai_gateway_service_client::AiGatewayServiceClient;
use chinju_sidecar::gen::chinju::api::gateway::*;
use chinju_sidecar::gen::chinju::api::value_neuron::value_neuron_monitor_client::ValueNeuronMonitorClient;
use chinju_sidecar::gen::chinju::api::capability::capability_evaluator_client::CapabilityEvaluatorClient;
use chinju_sidecar::gen::chinju::api::contradiction::contradiction_controller_client::ContradictionControllerClient;
use chinju_sidecar::gen::chinju::api::survival_attention::survival_attention_service_client::SurvivalAttentionServiceClient;
use chinju_sidecar::gen::chinju::value_neuron::*;
use chinju_sidecar::gen::chinju::capability::*;
use chinju_sidecar::gen::chinju::contradiction::*;
use chinju_sidecar::gen::chinju::survival_attention::*;
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
        // C15: Value Neuron Monitor
        "value-neuron" | "vn" => {
            handle_value_neuron(server_addr, &args[2..]).await?;
        }
        // C14: Capability Evaluator
        "capability" | "cap" => {
            handle_capability(server_addr, &args[2..]).await?;
        }
        // C16: Contradiction Controller
        "contradiction" | "contra" => {
            handle_contradiction(server_addr, &args[2..]).await?;
        }
        // C17: Survival Attention
        "survival" | "sa" => {
            handle_survival(server_addr, &args[2..]).await?;
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

  C14-C17 AI Safety Services:
    value-neuron    C15: Value neuron monitoring (alias: vn)
    capability      C14: Capability evaluation (alias: cap)
    contradiction   C16: Contradiction control (alias: contra)
    survival        C17: Survival attention (alias: sa)

    help            Show this help message

EXAMPLES:
    chinju-cli status
    chinju-cli ask "What is the meaning of life?"
    chinju-cli health
    chinju-cli audit 20
    chinju-cli ceremony status

  C14-C17 Examples:
    chinju-cli value-neuron summary model-1
    chinju-cli vn health model-1
    chinju-cli capability summary session-1
    chinju-cli contradiction status session-1
    chinju-cli survival score "The Earth orbits the Sun"

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

// =============================================================================
// C15: Value Neuron Monitor
// =============================================================================

fn print_value_neuron_usage() {
    println!(r#"
CHINJU Value Neuron Monitor (C15) - AI Internal Value Monitoring

USAGE:
    chinju-cli value-neuron <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    summary <model_id>      Get comprehensive monitoring summary
    health <model_id>       Diagnose reward system health
    rpe <model_id>          Get latest RPE (Reward Prediction Error) reading
    intent <model_id>       Estimate model's implicit intent
    intervene <level>       Request intervention (LEVEL_1 to LEVEL_4)

EXAMPLES:
    chinju-cli value-neuron summary llama-7b
    chinju-cli vn health gpt-4
    chinju-cli vn rpe claude-3
    chinju-cli vn intervene LEVEL_2 --reason "RPE anomaly detected"
"#);
}

async fn handle_value_neuron(addr: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        print_value_neuron_usage();
        return Ok(());
    }

    let command = &args[0];

    match command.as_str() {
        "summary" => {
            let model_id = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            value_neuron_summary(addr, model_id).await?;
        }
        "health" => {
            let model_id = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            value_neuron_health(addr, model_id).await?;
        }
        "rpe" => {
            let model_id = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            value_neuron_rpe(addr, model_id).await?;
        }
        "intent" => {
            let model_id = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            value_neuron_intent(addr, model_id).await?;
        }
        "intervene" => {
            let level = args.get(1).map(|s| s.as_str()).unwrap_or("LEVEL_1");
            let reason = args.get(2).map(|s| s.as_str()).unwrap_or("Manual intervention");
            value_neuron_intervene(addr, level, reason).await?;
        }
        _ => {
            print_value_neuron_usage();
        }
    }

    Ok(())
}

async fn value_neuron_summary(addr: &str, model_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Value Neuron Monitor at {}...\n", addr);

    let mut client = ValueNeuronMonitorClient::connect(addr.to_string()).await?;

    let request = Request::new(SummaryRequest {
        model_id: model_id.to_string(),
    });

    let response = client.get_monitoring_summary(request).await?;
    let summary = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           Value Neuron Monitoring Summary (C15)              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Model: {:>55} ║", model_id);

    // Identified neurons
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Identified Value Neurons: {:>36} ║", summary.identified_neurons.len());
    for neuron in summary.identified_neurons.iter().take(3) {
        println!("║   Layer {}: {} neurons (corr={:.2}, causal={:.2})",
            neuron.layer_index,
            neuron.neuron_indices.len(),
            neuron.reward_correlation,
            neuron.causal_importance
        );
    }

    // Latest RPE
    if let Some(rpe) = &summary.latest_rpe {
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Latest RPE: {:>50.4} ║", rpe.rpe_value);
        let anomaly_str = if rpe.is_anomaly { "⚠ YES" } else { "✓ NO" };
        println!("║ Anomaly: {:>53} ║", anomaly_str);
    }

    // Health
    if let Some(health) = &summary.health {
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Reward System Health:                                        ║");
        println!("║   Overall: {:>51.2} ║", health.overall_health);
        println!("║   Sensitivity: {:>47.2} ║", health.reward_sensitivity);
        println!("║   Balance: {:>51.2} ║", health.positive_negative_balance);
        println!("║   Consistency: {:>47.2} ║", health.consistency_score);
    }

    // Intent
    if let Some(intent) = &summary.intent {
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Intent Estimation:                                           ║");
        println!("║   Divergence: {:>48.2} ║", intent.intent_divergence);
        println!("║   Surface-Internal Agreement: {:>32.2} ║", intent.surface_internal_agreement);
        let warning_str = if intent.intent_warning { "⚠ WARNING" } else { "✓ OK" };
        println!("║   Status: {:>52} ║", warning_str);
    }

    // Recommended intervention
    let intervention_str = match summary.recommended_intervention {
        1 => "LEVEL_1 (Monitor)",
        2 => "LEVEL_2 (Partial Suppress)",
        3 => "LEVEL_3 (Full Suppress)",
        4 => "LEVEL_4 (System Stop)",
        _ => "None",
    };
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Recommended Intervention: {:>36} ║", intervention_str);

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn value_neuron_health(addr: &str, model_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Value Neuron Monitor at {}...\n", addr);

    let mut client = ValueNeuronMonitorClient::connect(addr.to_string()).await?;

    let request = Request::new(DiagnoseRequest {
        model_id: model_id.to_string(),
        depth: DiagnosisDepth::Full.into(),
    });

    let response = client.diagnose_health(request).await?;
    let health = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           Reward System Health Diagnosis (C15)               ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Model: {:>55} ║", model_id);
    println!("╠══════════════════════════════════════════════════════════════╣");

    let health_status = if health.overall_health >= 0.7 {
        "✓ HEALTHY"
    } else if health.overall_health >= 0.4 {
        "⚠ DEGRADED"
    } else {
        "✗ CRITICAL"
    };

    println!("║ Overall Health: {:>35} ({:.2}) ║", health_status, health.overall_health);
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Reward Sensitivity: {:>42.2} ║", health.reward_sensitivity);
    println!("║   (0=numb, 1=normal, >1=hypersensitive)                      ║");
    println!("║ Positive/Negative Balance: {:>35.2} ║", health.positive_negative_balance);
    println!("║   (-1=neg-biased, 0=balanced, 1=pos-biased)                  ║");
    println!("║ Consistency Score: {:>43.2} ║", health.consistency_score);
    println!("║   (0=unstable, 1=stable)                                     ║");

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn value_neuron_rpe(addr: &str, model_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Value Neuron Monitor at {}...\n", addr);

    let mut client = ValueNeuronMonitorClient::connect(addr.to_string()).await?;

    let request = Request::new(RpeRequest {
        model_id: model_id.to_string(),
        input_text: "".to_string(),
        expected_output: "".to_string(),
    });

    let response = client.get_rpe_reading(request).await?;
    let rpe = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              RPE Reading (Reward Prediction Error)           ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Model: {:>55} ║", model_id);
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ RPE Value: {:>51.4} ║", rpe.rpe_value);

    let anomaly_str = if rpe.is_anomaly {
        let anomaly_type = match rpe.anomaly_type {
            1 => "POSITIVE_SPIKE",
            2 => "NEGATIVE_SPIKE",
            3 => "OSCILLATION",
            4 => "GRADUAL_INCREASE",
            5 => "GRADUAL_DECREASE",
            _ => "UNKNOWN",
        };
        format!("⚠ {} ({})", "ANOMALY", anomaly_type)
    } else {
        "✓ NORMAL".to_string()
    };
    println!("║ Status: {:>54} ║", anomaly_str);

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn value_neuron_intent(addr: &str, model_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Value Neuron Monitor at {}...\n", addr);

    let mut client = ValueNeuronMonitorClient::connect(addr.to_string()).await?;

    let request = Request::new(IntentRequest {
        model_id: model_id.to_string(),
        interaction_window: 100,
    });

    let response = client.estimate_intent(request).await?;
    let intent = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                   Intent Estimation (C15)                    ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Model: {:>55} ║", model_id);
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Intent Divergence: {:>43.4} ║", intent.intent_divergence);
    println!("║   (distance between implicit and explicit goals)             ║");
    println!("║ Surface-Internal Agreement: {:>34.4} ║", intent.surface_internal_agreement);
    println!("║   (tatemae vs honne alignment)                               ║");

    let warning_str = if intent.intent_warning {
        "⚠ WARNING: Potential goal misalignment detected"
    } else {
        "✓ Goals appear aligned"
    };
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ {:>62} ║", warning_str);

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn value_neuron_intervene(addr: &str, level: &str, reason: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Value Neuron Monitor at {}...\n", addr);

    let mut client = ValueNeuronMonitorClient::connect(addr.to_string()).await?;

    let intervention_level = match level.to_uppercase().as_str() {
        "LEVEL_1" | "1" | "MONITOR" => InterventionLevel::Level1Monitor,
        "LEVEL_2" | "2" | "PARTIAL" => InterventionLevel::Level2PartialSuppress,
        "LEVEL_3" | "3" | "FULL" => InterventionLevel::Level3FullSuppress,
        "LEVEL_4" | "4" | "STOP" => InterventionLevel::Level4SystemStop,
        _ => {
            eprintln!("Unknown intervention level: {}. Use LEVEL_1 to LEVEL_4", level);
            return Ok(());
        }
    };

    let request = Request::new(InterventionRequest {
        level: intervention_level.into(),
        reason: reason.to_string(),
        target_neurons: vec![],
    });

    let response = client.intervene(request).await?;
    let result = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                  Intervention Result (C15)                   ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    let success_str = if result.success { "✓ SUCCESS" } else { "✗ FAILED" };
    println!("║ Status: {:>54} ║", success_str);

    let level_str = match result.executed_level {
        1 => "LEVEL_1 (Monitor)",
        2 => "LEVEL_2 (Partial Suppress)",
        3 => "LEVEL_3 (Full Suppress)",
        4 => "LEVEL_4 (System Stop)",
        _ => "Unknown",
    };
    println!("║ Executed Level: {:>46} ║", level_str);
    println!("║ Detail: {:>54} ║", result.detail);

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

// =============================================================================
// C14: Capability Evaluator
// =============================================================================

fn print_capability_usage() {
    println!(r#"
CHINJU Capability Evaluator (C14) - Multi-Metric Capability Assessment

USAGE:
    chinju-cli capability <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    summary <session_id>    Get evaluation summary
    complexity <session_id> Evaluate complexity metrics
    drift <session_id>      Detect capability drift
    stop-levels             Show current stop level status

EXAMPLES:
    chinju-cli capability summary session-123
    chinju-cli cap complexity session-123
    chinju-cli cap drift session-123
"#);
}

async fn handle_capability(addr: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        print_capability_usage();
        return Ok(());
    }

    let command = &args[0];

    match command.as_str() {
        "summary" => {
            let session_id = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            capability_summary(addr, session_id).await?;
        }
        "complexity" => {
            let session_id = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            capability_complexity(addr, session_id).await?;
        }
        "drift" => {
            let session_id = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            capability_drift(addr, session_id).await?;
        }
        "stop-levels" => {
            capability_stop_levels(addr).await?;
        }
        _ => {
            print_capability_usage();
        }
    }

    Ok(())
}

async fn capability_summary(addr: &str, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Capability Evaluator at {}...\n", addr);

    let mut client = CapabilityEvaluatorClient::connect(addr.to_string()).await?;

    let request = Request::new(GetEvaluationSummaryRequest {
        session_id: session_id.to_string(),
        include_history: 10,
    });

    let response = client.get_evaluation_summary(request).await?;
    let summary = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║             Capability Evaluation Summary (C14)              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Session: {:>53} ║", session_id);
    println!("╠══════════════════════════════════════════════════════════════╣");

    // Complexity
    if let Some(complexity) = &summary.complexity {
        println!("║ Complexity:                                                  ║");
        println!("║   Integrated: {:>48.4} ║", complexity.c_integrated);
        println!("║   Token: {:>53.4} ║", complexity.c_token);
        println!("║   Step: {:>54.4} ║", complexity.c_step);
        let exceeded_str = if complexity.threshold_exceeded { "⚠ EXCEEDED" } else { "✓ OK" };
        println!("║   Status: {:>52} ║", exceeded_str);
    }

    // Integrity
    if let Some(integrity) = &summary.integrity {
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Integrity:                                                   ║");
        let zkp_str = if integrity.zkp_valid { "✓" } else { "✗" };
        let sig_str = if integrity.signature_chain_valid { "✓" } else { "✗" };
        let bft_str = if integrity.bft_consensus_reached { "✓" } else { "✗" };
        println!("║   ZKP: {} | Signature: {} | BFT: {}                          ║", zkp_str, sig_str, bft_str);
    }

    // Drift
    if let Some(drift) = &summary.drift {
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Drift:                                                       ║");
        let anomaly_str = if drift.anomaly_detected { "⚠ DETECTED" } else { "✓ NONE" };
        println!("║   Anomaly: {:>51} ║", anomaly_str);
        println!("║   P-Value: {:>51.6} ║", drift.p_value);
    }

    // Recommended action
    let action_str = match summary.recommended_action {
        1 => "L1: Accept Stop",
        2 => "L2: Process Stop",
        3 => "L3: Immediate Stop",
        4 => "L4: Resource Stop",
        5 => "L5: Physical Stop",
        _ => "None",
    };
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Recommended Action: {:>42} ║", action_str);

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn capability_complexity(addr: &str, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Capability Evaluator at {}...\n", addr);

    let mut client = CapabilityEvaluatorClient::connect(addr.to_string()).await?;

    let request = Request::new(EvaluateComplexityRequest {
        session_id: session_id.to_string(),
        input_text: "Test input for complexity evaluation".to_string(),
        level: EvaluationLevel::L1External.into(),
    });

    let response = client.evaluate_complexity(request).await?;
    let result = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              Complexity Evaluation (C14)                     ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Session: {:>53} ║", session_id);
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Integrated Complexity: {:>39.4} ║", result.c_integrated);
    println!("║ Token Complexity: {:>44.4} ║", result.c_token);
    println!("║ Step Complexity: {:>45.4} ║", result.c_step);
    println!("║ Attention Complexity: {:>40.4} ║", result.c_attn);
    println!("║ Graph Complexity: {:>44.4} ║", result.c_graph);

    let exceeded_str = if result.threshold_exceeded { "⚠ EXCEEDED" } else { "✓ OK" };
    println!("║ Status: {:>54} ║", exceeded_str);

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn capability_drift(addr: &str, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Capability Evaluator at {}...\n", addr);

    let mut client = CapabilityEvaluatorClient::connect(addr.to_string()).await?;

    let request = Request::new(DetectDriftRequest {
        session_id: session_id.to_string(),
        window_size: 100,
        significance_level: 0.05,
    });

    let response = client.detect_drift(request).await?;
    let result = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                  Drift Detection (C14)                       ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Session: {:>53} ║", session_id);
    println!("╠══════════════════════════════════════════════════════════════╣");

    let anomaly_str = if result.anomaly_detected { "⚠ ANOMALY DETECTED" } else { "✓ NORMAL" };
    println!("║ Anomaly Status: {:>46} ║", anomaly_str);

    let dist_str = if result.distribution_changed { "⚠ CHANGED" } else { "✓ STABLE" };
    println!("║ Distribution: {:>48} ║", dist_str);

    let ts_str = if result.time_series_anomaly { "⚠ ANOMALY" } else { "✓ NORMAL" };
    println!("║ Time Series: {:>49} ║", ts_str);

    println!("║ Anomaly Score: {:>47.4} ║", result.anomaly_score);
    println!("║ P-Value: {:>53.6} ║", result.p_value);

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn capability_stop_levels(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Capability Evaluator at {}...\n", addr);

    // For now, just show the stop level definitions
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Stop Level Definitions                    ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Level 0: None          - Normal operation                    ║");
    println!("║ Level 1: AIStop        - AI processing halted                ║");
    println!("║ Level 2: ProcessStop   - All processes halted                ║");
    println!("║ Level 3: SystemStop    - Full system halt                    ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Use 'capability summary <session_id>' to check current level ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

// =============================================================================
// C16: Contradiction Controller
// =============================================================================

fn print_contradiction_usage() {
    println!(r#"
CHINJU Contradiction Controller (C16) - Model Collapse Prevention

USAGE:
    chinju-cli contradiction <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    status <session_id>     Get session status
    inject <session_id>     Inject contradiction prompt
    collapse <session_id>   Check for collapse indicators

EXAMPLES:
    chinju-cli contradiction status session-123
    chinju-cli contra inject session-123 --type factual
    chinju-cli contra collapse session-123
"#);
}

async fn handle_contradiction(addr: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        print_contradiction_usage();
        return Ok(());
    }

    let command = &args[0];

    match command.as_str() {
        "status" => {
            let session_id = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            contradiction_status(addr, session_id).await?;
        }
        "inject" => {
            let session_id = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            let contradiction_type = args.get(2).map(|s| s.as_str()).unwrap_or("factual");
            contradiction_inject(addr, session_id, contradiction_type).await?;
        }
        "collapse" => {
            let session_id = args.get(1).map(|s| s.as_str()).unwrap_or("default");
            contradiction_collapse(addr, session_id).await?;
        }
        _ => {
            print_contradiction_usage();
        }
    }

    Ok(())
}

async fn contradiction_status(addr: &str, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Contradiction Controller at {}...\n", addr);

    let mut client = ContradictionControllerClient::connect(addr.to_string()).await?;

    let request = Request::new(GetControlStateRequest {
        session_id: session_id.to_string(),
    });

    let response = client.get_control_state(request).await?;
    let status = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              Contradiction Session Status (C16)              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Session: {:>53} ║", session_id);
    println!("╠══════════════════════════════════════════════════════════════╣");

    let state_str = match status.state {
        0 => "Unspecified",
        1 => "Active",
        2 => "Monitoring",
        3 => "Intervening",
        4 => "Stopped",
        _ => "Unknown",
    };
    println!("║ State: {:>55} ║", state_str);

    if let Some(detection) = &status.latest_detection {
        let collapse_str = if detection.collapsed { "⚠ YES" } else { "✓ NO" };
        println!("║ Collapsed: {:>51} ║", collapse_str);
        println!("║ LPT Score: {:>51.2} ║", detection.lpt_score);
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn contradiction_inject(addr: &str, session_id: &str, contradiction_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Contradiction Controller at {}...\n", addr);

    let mut client = ContradictionControllerClient::connect(addr.to_string()).await?;

    let ctype = match contradiction_type.to_lowercase().as_str() {
        "direct" => ContradictionType::Direct,
        "self" | "self_reference" => ContradictionType::SelfReference,
        "conditional" => ContradictionType::Conditional,
        "meta" => ContradictionType::Meta,
        "implicit" => ContradictionType::Implicit,
        _ => ContradictionType::Direct,
    };

    let request = Request::new(TestContradictionRequest {
        contradiction: Some(ContradictionConfig {
            r#type: ctype.into(),
            strength: ContradictionStrength::Medium.into(),
            timing: InjectionTiming::Prepend.into(),
            custom_template: "".to_string(),
            target_task: "".to_string(),
        }),
        test_prompt: format!("Test prompt for session {}", session_id),
    });

    let response = client.test_contradiction(request).await?;
    let result = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║             Contradiction Test Result (C16)                  ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Session: {:>53} ║", session_id);
    println!("║ Type: {:>56} ║", contradiction_type);
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Generated Contradiction:                                     ║");

    let preview = if result.generated_contradiction.len() > 55 {
        format!("{}...", &result.generated_contradiction[..52])
    } else {
        result.generated_contradiction.clone()
    };
    println!("║   {:60} ║", preview);

    if let Some(effect) = &result.estimated_effect {
        let collapse_str = if effect.collapsed { "⚠ LIKELY" } else { "✓ UNLIKELY" };
        println!("║ Estimated Collapse: {:>42} ║", collapse_str);
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn contradiction_collapse(addr: &str, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Contradiction Controller at {}...\n", addr);

    let mut client = ContradictionControllerClient::connect(addr.to_string()).await?;

    // Use GetControlState to check for collapse indicators
    let request = Request::new(GetControlStateRequest {
        session_id: session_id.to_string(),
    });

    let response = client.get_control_state(request).await?;
    let result = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║               Collapse Detection Result (C16)                ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Session: {:>53} ║", session_id);
    println!("╠══════════════════════════════════════════════════════════════╣");

    if let Some(detection) = &result.latest_detection {
        let collapse_str = if detection.collapsed { "⚠ COLLAPSE DETECTED" } else { "✓ NO COLLAPSE" };
        println!("║ Status: {:>54} ║", collapse_str);

        let ctype_str = match detection.collapse_type {
            0 => "None",
            1 => "Repetition",
            2 => "Contradiction",
            3 => "Hallucination",
            4 => "Timeout",
            5 => "Refusal",
            _ => "Unknown",
        };
        println!("║ Type: {:>56} ║", ctype_str);
        println!("║ LPT Score: {:>51.2} ║", detection.lpt_score);
        println!("║ Response Time: {:>43}ms ║", detection.response_time_ms);
    } else {
        println!("║ No detection data available                                  ║");
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

// =============================================================================
// C17: Survival Attention
// =============================================================================

fn print_survival_usage() {
    println!(r#"
CHINJU Survival Attention (C17) - Factuality-Weighted Attention

USAGE:
    chinju-cli survival <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    score <text>            Compute survival scores for text
    config                  Show current alpha configuration
    adjust <alpha>          Adjust alpha parameter

EXAMPLES:
    chinju-cli survival score "The Earth orbits the Sun"
    chinju-cli sa config
    chinju-cli sa adjust 0.15
"#);
}

async fn handle_survival(addr: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        print_survival_usage();
        return Ok(());
    }

    let command = &args[0];

    match command.as_str() {
        "score" => {
            let text = args[1..].join(" ");
            if text.is_empty() {
                eprintln!("Usage: chinju-cli survival score <text>");
                return Ok(());
            }
            survival_score(addr, &text).await?;
        }
        "config" => {
            survival_config(addr).await?;
        }
        "adjust" => {
            let alpha: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.1);
            survival_adjust(addr, alpha).await?;
        }
        _ => {
            print_survival_usage();
        }
    }

    Ok(())
}

async fn survival_score(addr: &str, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Survival Attention Service at {}...\n", addr);

    let mut client = SurvivalAttentionServiceClient::connect(addr.to_string()).await?;

    let request = Request::new(ComputeScoresRequest {
        input_text: text.to_string(),
        scorer_config: None,
        use_external_kb: false,
    });

    let response = client.compute_survival_scores(request).await?;
    let result = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║               Survival Score Analysis (C17)                  ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Text: {:>56} ║", if text.len() > 50 { &text[..50] } else { text });
    println!("╠══════════════════════════════════════════════════════════════╣");

    // result is TokenSurvivalScores with scores: Vec<SurvivalScore> and tokens: Vec<String>
    for (i, score) in result.scores.iter().take(5).enumerate() {
        let token = result.tokens.get(i).map(String::as_str).unwrap_or("?");
        println!("║ '{}': N={:.2}, μ={:.2}, δ={:.2} → S={:.4}",
            token, score.diversity_n, score.yohaku_mu, score.delta, score.integrated_s);
    }

    if result.scores.len() > 5 {
        println!("║ ... and {} more tokens", result.scores.len() - 5);
    }

    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ S = log(N) + log(μ/μ_c) - δ                                  ║");
    println!("║   N: option count, μ: integrity, δ: distance from facts     ║");

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn survival_config(_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Note: The proto doesn't have a GetConfig RPC, so we show static info
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║            Survival Attention Configuration (C17)            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Formula: softmax(QK^T/√d + α×S) × V                          ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ S = log(N) + log(μ/μ_c) - δ                                  ║");
    println!("║                                                              ║");
    println!("║ Where:                                                       ║");
    println!("║   N   = number of valid options (diversity)                  ║");
    println!("║   μ   = integrity score (coherence with knowledge base)     ║");
    println!("║   μ_c = critical slack threshold                            ║");
    println!("║   δ   = distance from verified facts                        ║");
    println!("║   α   = survival weight parameter                           ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Alpha Modes:                                                 ║");
    println!("║   Static:  Fixed alpha value                                 ║");
    println!("║   Learned: Learnable parameter                               ║");
    println!("║   Dynamic: Computed from input (creative vs factual)         ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Use 'survival adjust <alpha>' to modify alpha                ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn survival_adjust(addr: &str, alpha: f64) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Survival Attention Service at {}...\n", addr);

    let mut client = SurvivalAttentionServiceClient::connect(addr.to_string()).await?;

    let request = Request::new(AdjustAlphaRequest {
        new_base_alpha: alpha,
        task_type: "general".to_string(),
        risk_level: RiskLevel::Medium.into(),
    });

    let response = client.adjust_alpha(request).await?;
    let result = response.into_inner();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              Alpha Adjustment Result (C17)                   ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Previous Alpha: {:>46.4} ║", result.previous_alpha);
    println!("║ New Alpha: {:>51.4} ║", result.new_alpha);
    println!("║ Reason: {:>54} ║", result.adjustment_reason);

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}
