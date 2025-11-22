use clap::Parser;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "hexagondb-cli")]
#[command(about = "HexagonDB command-line client", long_about = None)]
struct Cli {
    /// Server host
    #[arg(short = 'h', long, default_value = "127.0.0.1")]
    host: String,

    /// Server port
    #[arg(short, long, default_value_t = 6379)]
    port: u16,

    /// Execute command and exit
    #[arg(short, long)]
    command: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let addr = format!("{}:{}", cli.host, cli.port);

    println!("{}", "HexagonDB CLI v0.1.0".bright_cyan().bold());
    println!("Connecting to {}...", addr.bright_yellow());

    let mut stream = TcpStream::connect_timeout(&addr.parse()?, Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    println!("{}\n", "Connected!".bright_green().bold());

    // If command provided, execute and exit
    if let Some(cmd) = cli.command {
        execute_command(&mut stream, &cmd)?;
        return Ok(());
    }

    // Interactive REPL mode
    let mut rl = DefaultEditor::new()?;
    let history_file = dirs::home_dir().map(|mut p| {
        p.push(".hexagondb_history");
        p
    });

    if let Some(ref path) = history_file {
        let _ = rl.load_history(path);
    }

    loop {
        let prompt = format!("{}> ", "hexagondb".bright_cyan());
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line)?;

                // Handle special commands
                if line.eq_ignore_ascii_case("exit") || line.eq_ignore_ascii_case("quit") {
                    println!("{}", "Goodbye!".bright_yellow());
                    break;
                }

                if line.eq_ignore_ascii_case("clear") {
                    print!("\x1B[2J\x1B[1;1H");
                    continue;
                }

                if line.eq_ignore_ascii_case("help") {
                    print_help();
                    continue;
                }

                // Execute command
                if let Err(e) = execute_command(&mut stream, line) {
                    eprintln!("{} {}", "Error:".bright_red().bold(), e);

                    // Try to reconnect
                    println!("{}", "Attempting to reconnect...".bright_yellow());
                    match TcpStream::connect_timeout(&addr.parse()?, Duration::from_secs(5)) {
                        Ok(new_stream) => {
                            stream = new_stream;
                            stream.set_read_timeout(Some(Duration::from_secs(10)))?;
                            stream.set_write_timeout(Some(Duration::from_secs(10)))?;
                            println!("{}", "Reconnected!".bright_green());
                        }
                        Err(e) => {
                            eprintln!("{} {}", "Reconnection failed:".bright_red().bold(), e);
                            break;
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "^C".bright_yellow());
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "Goodbye!".bright_yellow());
                break;
            }
            Err(err) => {
                eprintln!("{} {:?}", "Error:".bright_red().bold(), err);
                break;
            }
        }
    }

    if let Some(ref path) = history_file {
        let _ = rl.save_history(path);
    }

    Ok(())
}

fn execute_command(
    stream: &mut TcpStream,
    command: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse command into RESP format
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(());
    }

    // Build RESP array
    let mut resp = format!("*{}\r\n", parts.len());
    for part in parts {
        resp.push_str(&format!("${}\r\n{}\r\n", part.len(), part));
    }

    // Send command
    stream.write_all(resp.as_bytes())?;
    stream.flush()?;

    // Read response
    let mut buffer = vec![0u8; 8192];
    let n = stream.read(&mut buffer)?;

    if n == 0 {
        return Err("Connection closed by server".into());
    }

    let response = String::from_utf8_lossy(&buffer[..n]);
    print_response(&response);

    Ok(())
}

fn print_response(response: &str) {
    let lines: Vec<&str> = response.lines().collect();

    if lines.is_empty() {
        return;
    }

    let first_char = lines[0].chars().next();

    match first_char {
        Some('+') => {
            // Simple string
            println!("{}", lines[0][1..].bright_green());
        }
        Some('-') => {
            // Error
            println!(
                "{} {}",
                "ERROR:".bright_red().bold(),
                lines[0][1..].bright_red()
            );
        }
        Some(':') => {
            // Integer
            println!("{}", lines[0][1..].bright_cyan());
        }
        Some('$') => {
            // Bulk string
            if lines[0] == "$-1" {
                println!("{}", "(nil)".bright_black());
            } else if lines.len() > 1 {
                println!("{}", lines[1].bright_white());
            }
        }
        Some('*') => {
            // Array
            let count = lines[0][1..].parse::<i32>().unwrap_or(0);
            if count == -1 {
                println!("{}", "(nil)".bright_black());
            } else if count == 0 {
                println!("{}", "(empty array)".bright_black());
            } else {
                let mut i = 1;
                let mut index = 1;
                while i < lines.len() && index <= count {
                    if lines[i].starts_with('$') {
                        if lines[i] == "$-1" {
                            println!("{}) {}", index, "(nil)".bright_black());
                        } else if i + 1 < lines.len() {
                            println!("{}) {}", index, lines[i + 1].bright_white());
                            i += 1;
                        }
                        index += 1;
                    } else if lines[i].starts_with(':') {
                        println!("{}) {}", index, lines[i][1..].bright_cyan());
                        index += 1;
                    }
                    i += 1;
                }
            }
        }
        _ => {
            println!("{}", response);
        }
    }
}

fn print_help() {
    println!("\n{}", "HexagonDB CLI Commands:".bright_cyan().bold());
    println!("  {}  - Exit the CLI", "exit/quit".bright_yellow());
    println!("  {}       - Clear the screen", "clear".bright_yellow());
    println!("  {}        - Show this help", "help".bright_yellow());
    println!("\n{}", "HexagonDB Commands:".bright_cyan().bold());
    println!("  String: SET, GET, DEL, INCR, DECR, EXISTS, KEYS");
    println!("  List:   LPUSH, RPUSH, LPOP, RPOP, LLEN, LRANGE");
    println!("  Set:    SADD, SREM, SMEMBERS, SISMEMBER");
    println!("  Hash:   HSET, HGET, HDEL, HGETALL, HKEYS, HVALS");
    println!("  Sorted: ZADD, ZREM, ZRANGE, ZCARD, ZSCORE");
    println!("  TTL:    EXPIRE, TTL, PERSIST");
    println!("  Pub/Sub: PUBLISH, SUBSCRIBE, UNSUBSCRIBE");
    println!("  Server: PING, INFO, SAVE, CONFIG, SHUTDOWN");
    println!();
}
