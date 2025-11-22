use hexagondb::aof::Aof;
use hexagondb::connection;
use hexagondb::database::DB;
use hexagondb::interpreter;
use parking_lot::Mutex;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use tracing::{error, info};
use tracing_subscriber;

fn main() -> std::io::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    info!("Initializing HexagonDB...");

    let db = Arc::new(Mutex::new(DB::new()));

    let aof = Aof::new("database.aof").unwrap_or_else(|e| {
        error!("Failed to create AOF: {}", e);
        std::process::exit(1);
    });
    Aof::load("database.aof", &db).ok();
    let aof = Arc::new(Mutex::new(aof));

    // Bind to Redis-compatible port (6379)
    let addr = "127.0.0.1:6379";
    let listener = TcpListener::bind(addr)?;
    info!(
        "HexagonDB server listening on {} (Redis-compatible port)",
        addr
    );

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let db_clone = Arc::clone(&db);
                let aof_clone = Arc::clone(&aof);

                // Spawn a new thread for each client connection
                thread::spawn(move || {
                    info!("New client connected");
                    let mut client = interpreter::Interpreter::new(db_clone, aof_clone);
                    connection::handle_client(stream, &mut client);
                    info!("Client disconnected");
                });
            }
            Err(e) => error!("Connection failed: {}", e),
        }
    }

    Ok(())
}
