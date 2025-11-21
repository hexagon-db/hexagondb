use clap::Parser;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info};
use tracing_subscriber;

use hexagondb::{
    commands, config::Config, db::DB, network::connection, persistence::aof::Aof,
    server_info::ServerInfo,
};

/// HexagonDB - in-memory database written in Rust
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "hexagondb.toml")]
    config: String,

    /// Override bind address
    #[arg(long)]
    bind: Option<String>,

    /// Override port
    #[arg(short, long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Load configuration
    let mut config = Config::from_file(&args.config).unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config file: {}", e);
        eprintln!("Using default configuration");
        Config::default()
    });

    // Override with CLI arguments
    if let Some(bind) = args.bind {
        config.server.bind_address = bind;
    }
    if let Some(port) = args.port {
        config.server.port = port;
    }

    // Wrap config in Arc<RwLock> for hot reload
    let config = Arc::new(RwLock::new(config));

    // Initialize logging with configured level.
    // Note: Dynamic log level change requires tracing-subscriber reload layer.
    // Currently, config reload does not update the log level.
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .compact()
        .init();

    // Initialize metrics
    hexagondb::observability::metrics::init_metrics();

    {
        let cfg = config.read().await;
        info!("Starting HexagonDB server...");
        info!("Configuration loaded from: {}", args.config);
        info!("HexagonDB server listening on {} 🚀", cfg.server_address());
        info!("Max connections: {}", cfg.server.max_connections);
    }

    // Create database
    let db: DB = DB::new();
    let db = Arc::new(RwLock::new(db));

    // Initialize AOF
    let aof = Aof::new("database.aof")?;
    if let Err(e) = Aof::load("database.aof", &db).await {
        error!("Error loading AOF: {}", e);
    }
    let aof = Arc::new(RwLock::new(aof));

    // Initialize server info
    let server_info = Arc::new(ServerInfo::new());

    // Start TCP server
    let addr = config.read().await.server_address();
    let listener = TcpListener::bind(&addr).await?;

    // Limit max concurrent connections
    let max_conn = config.read().await.server.max_connections;
    let connection_limit = Arc::new(tokio::sync::Semaphore::new(max_conn));

    // Initialize PubSub
    let pubsub = Arc::new(hexagondb::db::pubsub::PubSub::new());

    // Spawn signal handler for SIGHUP
    let config_clone = Arc::clone(&config);
    let config_path = args.config.clone();
    tokio::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};
        let mut stream = signal(SignalKind::hangup()).unwrap();
        loop {
            stream.recv().await;
            info!("Received SIGHUP. Reloading configuration...");
            match Config::from_file(&config_path) {
                Ok(new_config) => {
                    let mut cfg = config_clone.write().await;
                    cfg.persistence = new_config.persistence;
                    cfg.logging = new_config.logging;
                    cfg.memory = new_config.memory;
                    info!("Configuration reloaded successfully");
                }
                Err(e) => error!("Failed to reload configuration: {}", e),
            }
        }
    });

    // Spawn automatic RDB save task
    let db_clone = Arc::clone(&db);
    let config_clone = Arc::clone(&config);
    tokio::spawn(async move {
        let mut last_save_time = std::time::Instant::now();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            let cfg = config_clone.read().await;
            if !cfg.persistence.rdb_enabled {
                continue;
            }

            let save_interval = cfg.persistence.rdb_save_interval;
            drop(cfg);

            if last_save_time.elapsed().as_secs() < save_interval {
                continue;
            }

            let changes = {
                let db_guard = db_clone.read().await;
                db_guard.get_changes()
            };

            if changes > 0 {
                info!("Auto-save triggered: {} changes since last save", changes);

                match hexagondb::persistence::snapshot::save("dump.rdb", &db_clone).await {
                    Ok(_) => {
                        let db_guard = db_clone.read().await;
                        db_guard.reset_changes();
                        drop(db_guard);
                        last_save_time = std::time::Instant::now();
                        info!("Auto-save completed successfully");
                    }
                    Err(e) => {
                        error!("Auto-save failed: {}", e);
                    }
                }
            }
        }
    });

    // Accept incoming connections
    loop {
        // Acquire permit before accepting (or immediately after accepting to not block accept loop?)
        // Better to acquire after accept but before spawning heavy task, or use acquire_owned for the task.
        // If we acquire before accept, we block the accept loop which is fine for backpressure.

        // However, if we block accept, the OS backlog fills up.
        // Let's accept first, then try to acquire. If full, drop connection or wait?
        // Standard pattern: Acquire permit, then accept? No, accept then acquire.

        match listener.accept().await {
            Ok((stream, addr)) => {
                let db_clone = Arc::clone(&db);
                let aof_clone = Arc::clone(&aof);
                let info_clone = Arc::clone(&server_info);
                let config_clone = Arc::clone(&config);
                let pubsub_clone = Arc::clone(&pubsub);
                let limit_clone = Arc::clone(&connection_limit);

                // Try to acquire permit
                // We use acquire_owned so the permit moves into the task and is dropped when task finishes
                match limit_clone.clone().try_acquire_owned() {
                    Ok(permit) => {
                        tokio::spawn(async move {
                            // permit is held until this block exits
                            let _permit = permit;
                            info!("New client connected: {}", addr);
                            let mut client = commands::Interpreter::new(
                                db_clone,
                                aof_clone,
                                info_clone,
                                config_clone,
                                pubsub_clone,
                            );
                            connection::handle_client(stream, &mut client).await;
                            info!("Client disconnected: {}", addr);
                        });
                    }
                    Err(_) => {
                        error!("Max connections reached. Rejecting client: {}", addr);
                        // Optional: Send error message to client before closing?
                        // stream.write_all(b"-ERR max number of clients reached\r\n").await.ok();
                    }
                }
            }
            Err(e) => error!("Connection error: {}", e),
        }
    }
}
