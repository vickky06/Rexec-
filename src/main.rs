mod cleanup_service;
mod config_service;
mod docker;
mod language_executor;
mod ports_service;
mod proto;
mod executor_service;
mod session_management_service;
mod utils;
mod validation_service;
mod websocket_server;
mod models;

use models::{
    cleanup_models::{ActivityType, CleanupService},
    config_models::Config,
    executor_models::ExecutorService,
    session_management_models::SessionManagementService as ssm,
};
use crate::config_service::{ GLOBAL_CONFIG};
use proto::executor::code_executor_server::CodeExecutorServer;
use session_management_service::SessionManagement;
use websocket_server::run_websocket_server;

use std::env;
use std::net::SocketAddr;
use tokio::signal;
use tokio::time::Duration;
use tonic::transport::Server;
use tonic_reflection::server::Builder;
use uuid::Uuid;
use crate::models::port_models::PortsService as ps;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <run|grpcui>", args[0]);
        return Ok(());
    }

    let command = &args[1];
    println!("Command: {}", command);

    let mut config = Config::new();

    let server_pod_id = Uuid::new_v4(); // Replace with actual server pod ID
    config.session_management_service =ssm::default();
        // session_management_service::SessionManagementService::default(); // second call
    config.build.service_name = format!("{} {}", config.build.service_name, server_pod_id);
    GLOBAL_CONFIG.set(config).expect("Config already set");
    let ports_service = ps::new(); //ports_service::PortsService::new();
    let address = ports_service.get_grpc_server_address();
    println!("gRPC server address: {}", address); // Add this line
    let grpc_addr: SocketAddr = address
        .parse()
        .expect("Failed to parse gRPC server address");

    let websocket_addr = ports_service.get_websocket_address();

    let service = ExecutorService::default();

    let reflection_service = Builder::configure()
        .register_encoded_file_descriptor_set(proto::executor::FILE_DESCRIPTOR_SET)
        .build()?;

    let ws_handle = tokio::spawn(async move {
        if let Err(e) = run_websocket_server(&websocket_addr).await {
            eprintln!("WebSocket server error: {}", e);
        }
    });

    match command.as_str() {
        cmd if cmd.contains("run") => {
            println!(
                "Initiating port {} for {} service",
                grpc_addr,
                &GLOBAL_CONFIG.get().unwrap().constants.service_name
            );

            let session_management_service = GLOBAL_CONFIG
                .get()
                .unwrap()
                .session_management_service
                .clone();

            tokio::spawn(async move {
                let cleanup_interval = Duration::from_secs(
                    GLOBAL_CONFIG
                        .get()
                        .unwrap()
                        .session_configs
                        .session_cleanup_interval,
                );
                loop {
                    if session_management_service.get_last_cleanup().await + cleanup_interval
                        <= std::time::Instant::now()
                    {
                        println!("Skipping cleanup, last cleanup was recent.");
                        tokio::time::sleep(cleanup_interval).await;
                        continue;
                    }
                    tokio::time::sleep(cleanup_interval).await;
                    let _ = session_management_service.cleanup_expired_sessions();
                    println!("Periodic session cleanup completed.");
                }
            });

            // Create a shutdown signal future
            let shutdown_signal = async {
                signal::ctrl_c()
                    .await
                    .expect("Failed to install Ctrl+C handler");
                println!("Shutdown signal received. Cleaning up...");
            };

            let grpc_server = Server::builder()
                .add_service(CodeExecutorServer::new(service))
                .add_service(reflection_service)
                .serve_with_shutdown(grpc_addr, shutdown_signal);
            // Run the server and listen for shutdown signal
            tokio::select! {
                     _ = grpc_server => {
                println!("gRPC server exited");
            }
            _ = ws_handle => {
                println!("WebSocket server exited");
            }
                }
            let container = cleanup_service::CLEANUP_ACTIVITY_CONTAINER;
            let all_tars = cleanup_service::CLEANUP_ACTIVITY_ALL_TARS;
            println!("Cleaning up resources...");
            println!("Container: {}", container);
            println!("All Tars: {}", all_tars);
            // Perform cleanup operations
            // Cleanup logic here
            let cleanup_service = CleanupService {};
            let activity = ActivityType::new(
                Some(container.to_string()),
                None,
                Some(all_tars.to_string()),
                None,
                Some(ports_service.get_all_ports()),
            );
            cleanup_service.cleanup(activity).await?;

            println!("Server exited cleanly.");
        }
        cmd if cmd.contains("grpcui") => {
            println!(
                "GRPC server starting on {}",
                ports_service.get_grpc_server_address()
            );
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Usage: {} <run|grpcui>", args[0]);
        }
    }

    Ok(())
}
