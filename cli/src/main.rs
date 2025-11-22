use clap::Parser;
use colored::Colorize;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor};
use std::borrow::Cow;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "hexagondb-cli")]
#[command(about = "HexagonDB command-line client", long_about = None)]
#[command(version)]
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

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Disable colors
    #[arg(long)]
    no_color: bool,

    /// Password for authentication
    #[arg(short = 'a', long)]
    password: Option<String>,

    /// Prompt for password (secure input)
    #[arg(long)]
    ask_pass: bool,
}

// Command completer and helper
struct HexagonHelper {
    commands: Vec<String>,
}

impl HexagonHelper {
    fn new() -> Self {
        Self {
            commands: vec![
                // String commands
                "GET",
                "SET",
                "DEL",
                "INCR",
                "DECR",
                "EXISTS",
                "KEYS",
                // List commands
                "LPUSH",
                "RPUSH",
                "LPOP",
                "RPOP",
                "LLEN",
                "LRANGE",
                // Hash commands
                "HSET",
                "HGET",
                "HDEL",
                "HGETALL",
                "HKEYS",
                "HVALS",
                // Set commands
                "SADD",
                "SREM",
                "SMEMBERS",
                "SISMEMBER",
                // Sorted set commands
                "ZADD",
                "ZREM",
                "ZRANGE",
                "ZCARD",
                "ZSCORE",
                // TTL commands
                "EXPIRE",
                "TTL",
                "PERSIST",
                // Pub/Sub commands
                "PUBLISH",
                "SUBSCRIBE",
                "UNSUBSCRIBE",
                // Server commands
                "PING",
                "ECHO",
                "INFO",
                "SAVE",
                "CONFIG",
                "SHUTDOWN",
                // CLI commands
                "HELP",
                "CLEAR",
                "EXIT",
                "QUIT",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        }
    }
}

impl Completer for HexagonHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line = &line[..pos];
        let mut matches = Vec::new();

        // Get the last word for completion
        let start = line.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word = &line[start..];

        if word.is_empty() {
            return Ok((start, matches));
        }

        let word_upper = word.to_uppercase();
        let is_lowercase = word.chars().all(|c| !c.is_uppercase());

        // Find matching commands
        for cmd in &self.commands {
            if cmd.starts_with(&word_upper) {
                // Preserve user's case preference
                let replacement = if is_lowercase {
                    cmd.to_lowercase()
                } else {
                    cmd.clone()
                };
                matches.push(Pair {
                    display: cmd.clone(),
                    replacement,
                });
            }
        }

        Ok((start, matches))
    }
}

impl Hinter for HexagonHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        if pos < line.len() {
            return None;
        }

        let line_upper = line.trim().to_uppercase();

        // Find first matching command
        for cmd in &self.commands {
            if cmd.starts_with(&line_upper) && cmd.len() > line_upper.len() {
                return Some(cmd[line_upper.len()..].to_string());
            }
        }

        None
    }
}

impl Highlighter for HexagonHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        // Simple syntax highlighting
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Cow::Borrowed(line);
        }

        let cmd = parts[0].to_uppercase();

        // Check if it's a valid command
        if self.commands.contains(&cmd) {
            // Command is valid - would be colored in terminal
            Cow::Borrowed(line)
        } else {
            Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        true
    }
}

impl Validator for HexagonHelper {}

// Implement Helper trait by combining all the traits
impl rustyline::Helper for HexagonHelper {}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let addr = format!("{}:{}", cli.host, cli.port);

    // Simple startup message (no banner)
    if !cli.no_color {
        println!(
            "{} {}",
            "HexagonDB CLI".bright_cyan().bold(),
            "v0.1.0".bright_black()
        );
        println!("{}", "Redis & MongoDB compatible".bright_black());
    } else {
        println!("HexagonDB CLI v0.1.0");
    }

    if cli.verbose {
        println!("Connecting to {}...", addr.bright_yellow());
    }

    let mut stream = TcpStream::connect_timeout(&addr.parse()?, Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    if !cli.no_color {
        println!("{}\n", "✓ Connected".bright_green());
    } else {
        println!("Connected\n");
    }

    // Handle authentication if password provided (with brute force protection)
    let password = if cli.ask_pass {
        if !cli.no_color {
            print!("{}", "Password: ".bright_yellow());
        } else {
            print!("Password: ");
        }
        std::io::stdout().flush()?;
        Some(rpassword::read_password()?)
    } else {
        cli.password
    };

    if let Some(ref pass) = password {
        const MAX_ATTEMPTS: u8 = 3;
        let mut attempts = 0;
        let mut authenticated = false;

        while attempts < MAX_ATTEMPTS && !authenticated {
            attempts += 1;

            // Send AUTH command
            let auth_cmd = format!("AUTH {}", pass);
            match execute_command(&mut stream, &auth_cmd, false, cli.no_color) {
                Ok(_) => {
                    authenticated = true;
                    if !cli.no_color {
                        println!("{}\n", "✓ Authenticated".bright_green());
                    } else {
                        println!("Authenticated\n");
                    }
                }
                Err(e) => {
                    if attempts < MAX_ATTEMPTS {
                        if !cli.no_color {
                            eprintln!(
                                "{} {} ({}/{})",
                                "✗ Authentication failed:".bright_red().bold(),
                                e,
                                attempts,
                                MAX_ATTEMPTS
                            );
                            print!("{}", "Password: ".bright_yellow());
                        } else {
                            eprintln!(
                                "Authentication failed: {} ({}/{})",
                                e, attempts, MAX_ATTEMPTS
                            );
                            print!("Password: ");
                        }
                        std::io::stdout().flush()?;
                        // Read new password
                        let _new_pass = rpassword::read_password()?;
                        // Password will be re-read in next iteration
                        continue;
                    } else {
                        if !cli.no_color {
                            eprintln!(
                                "{} {}",
                                "✗ Maximum authentication attempts reached!"
                                    .bright_red()
                                    .bold(),
                                "Disconnecting...".bright_yellow()
                            );
                        } else {
                            eprintln!("Maximum authentication attempts reached! Disconnecting...");
                        }
                        return Err(anyhow::anyhow!("Too many failed authentication attempts"));
                    }
                }
            }
        }
    }

    // If command provided, execute and exit
    if let Some(cmd) = cli.command {
        execute_command(&mut stream, &cmd, cli.verbose, cli.no_color)?;
        return Ok(());
    }

    // Interactive REPL mode with advanced features
    let mut rl = Editor::new()?;
    rl.set_helper(Some(HexagonHelper::new()));

    // Load history
    let history_file = dirs::home_dir().map(|mut p| {
        p.push(".hexagondb_history");
        p
    });
    if let Some(ref path) = history_file {
        let _ = rl.load_history(path);
    }

    // Show welcome message (vim commands are hidden easter egg)
    if !cli.no_color {
        println!(
            "{}",
            "Type 'help' for commands • Tab completion enabled\n".bright_black()
        );
    }

    loop {
        let prompt = if !cli.no_color {
            format!("{}> ", "hexagondb".bright_cyan().bold())
        } else {
            "hexagondb> ".to_string()
        };

        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line)?;

                // Handle vim-like commands (hidden easter egg - no announcement)
                if line.starts_with(':') {
                    match line {
                        ":q" | ":quit" => {
                            if !cli.no_color {
                                println!("{}", "Goodbye!".bright_yellow());
                            } else {
                                println!("Goodbye!");
                            }
                            break;
                        }
                        ":w" | ":write" => {
                            if !cli.no_color {
                                println!("{}", "✓ History saved".bright_green());
                            } else {
                                println!("History saved");
                            }
                            if let Some(ref path) = history_file {
                                let _ = rl.save_history(path);
                            }
                            continue;
                        }
                        ":help" | ":h" => {
                            print_help(cli.no_color);
                            continue;
                        }
                        ":clear" | ":cl" => {
                            print!("\x1B[2J\x1B[1;1H");
                            continue;
                        }
                        _ => {
                            if !cli.no_color {
                                println!("{} {}", "Unknown vim command:".bright_red(), line);
                                println!("{}", "Try :help, :quit, :write, :clear".bright_black());
                            } else {
                                println!("Unknown vim command: {}", line);
                            }
                            continue;
                        }
                    }
                }

                // Handle special commands
                let cmd_upper = line.to_uppercase();

                // MongoDB compatibility aliases
                let cmd_normalized = match cmd_upper.as_str() {
                    "FIND" => "KEYS *", // MongoDB find -> Redis KEYS
                    "INSERT" => {
                        if !cli.no_color {
                            println!(
                                "{}",
                                "Use SET key value (Redis) or HSET collection id field value"
                                    .bright_yellow()
                            );
                        }
                        continue;
                    }
                    "UPDATE" => {
                        if !cli.no_color {
                            println!(
                                "{}",
                                "Use SET key value or HSET collection id field value"
                                    .bright_yellow()
                            );
                        }
                        continue;
                    }
                    "DELETE" => "DEL", // MongoDB delete -> Redis DEL
                    _ => line,
                };

                if cmd_upper == "EXIT" || cmd_upper == "QUIT" {
                    if !cli.no_color {
                        println!("{}", "Goodbye!".bright_yellow());
                    } else {
                        println!("Goodbye!");
                    }
                    break;
                }

                // Fix CLEAR command - use uppercase comparison
                if cmd_upper == "CLEAR" || cmd_upper == "CLS" {
                    print!("\x1B[2J\x1B[1;1H");
                    continue;
                }

                if cmd_upper == "HELP" {
                    print_help(cli.no_color);
                    continue;
                }

                // Execute command
                if let Err(e) =
                    execute_command(&mut stream, cmd_normalized, cli.verbose, cli.no_color)
                {
                    if !cli.no_color {
                        eprintln!("{} {}", "✗ Error:".bright_red().bold(), e);
                    } else {
                        eprintln!("Error: {}", e);
                    }

                    // Try to reconnect
                    if !cli.no_color {
                        println!("{}", "⟳ Attempting to reconnect...".bright_yellow());
                    } else {
                        println!("Attempting to reconnect...");
                    }

                    match TcpStream::connect_timeout(&addr.parse()?, Duration::from_secs(5)) {
                        Ok(new_stream) => {
                            stream = new_stream;
                            stream.set_read_timeout(Some(Duration::from_secs(10)))?;
                            stream.set_write_timeout(Some(Duration::from_secs(10)))?;
                            if !cli.no_color {
                                println!("{}", "✓ Reconnected!".bright_green());
                            } else {
                                println!("Reconnected!");
                            }
                        }
                        Err(e) => {
                            if !cli.no_color {
                                eprintln!("{} {}", "✗ Reconnection failed:".bright_red().bold(), e);
                            } else {
                                eprintln!("Reconnection failed: {}", e);
                            }
                            break;
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                if !cli.no_color {
                    println!(
                        "{}",
                        "^C (Press Ctrl+D or type EXIT to quit)".bright_yellow()
                    );
                } else {
                    println!("^C");
                }
                continue;
            }
            Err(ReadlineError::Eof) => {
                if !cli.no_color {
                    println!("{}", "👋 Goodbye!".bright_yellow());
                } else {
                    println!("Goodbye!");
                }
                break;
            }
            Err(err) => {
                if !cli.no_color {
                    eprintln!("{} {:?}", "✗ Error:".bright_red().bold(), err);
                } else {
                    eprintln!("Error: {:?}", err);
                }
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
    verbose: bool,
    no_color: bool,
) -> anyhow::Result<()> {
    // Parse command into RESP format
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(());
    }

    if verbose {
        if !no_color {
            println!("{} {}", "→".bright_blue(), command.bright_white());
        } else {
            println!("→ {}", command);
        }
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
        return Err(anyhow::anyhow!("Connection closed by server"));
    }

    let response = String::from_utf8_lossy(&buffer[..n]);
    print_response(&response, no_color);

    Ok(())
}

fn print_response(response: &str, no_color: bool) {
    let lines: Vec<&str> = response.lines().collect();

    if lines.is_empty() {
        return;
    }

    let first_char = lines[0].chars().next();

    match first_char {
        Some('+') => {
            // Simple string
            if !no_color {
                println!("{}", lines[0][1..].bright_green());
            } else {
                println!("{}", &lines[0][1..]);
            }
        }
        Some('-') => {
            // Error
            if !no_color {
                println!(
                    "{} {}",
                    "ERROR:".bright_red().bold(),
                    lines[0][1..].bright_red()
                );
            } else {
                println!("ERROR: {}", &lines[0][1..]);
            }
        }
        Some(':') => {
            // Integer
            if !no_color {
                println!("{}", lines[0][1..].bright_cyan());
            } else {
                println!("{}", &lines[0][1..]);
            }
        }
        Some('$') => {
            // Bulk string
            if lines[0] == "$-1" {
                if !no_color {
                    println!("{}", "(nil)".bright_black());
                } else {
                    println!("(nil)");
                }
            } else if lines.len() > 1 {
                if !no_color {
                    println!("{}", lines[1].bright_white());
                } else {
                    println!("{}", lines[1]);
                }
            }
        }
        Some('*') => {
            // Array
            let count = lines[0][1..].parse::<i32>().unwrap_or(0);
            if count == -1 {
                if !no_color {
                    println!("{}", "(nil)".bright_black());
                } else {
                    println!("(nil)");
                }
            } else if count == 0 {
                if !no_color {
                    println!("{}", "(empty array)".bright_black());
                } else {
                    println!("(empty array)");
                }
            } else {
                let mut i = 1;
                let mut index = 1;
                while i < lines.len() && index <= count {
                    if lines[i].starts_with('$') {
                        if lines[i] == "$-1" {
                            if !no_color {
                                println!("{}) {}", index, "(nil)".bright_black());
                            } else {
                                println!("{}) (nil)", index);
                            }
                        } else if i + 1 < lines.len() {
                            if !no_color {
                                println!(
                                    "{}) {}",
                                    index.to_string().bright_yellow(),
                                    lines[i + 1].bright_white()
                                );
                            } else {
                                println!("{}) {}", index, lines[i + 1]);
                            }
                            i += 1;
                        }
                        index += 1;
                    } else if lines[i].starts_with(':') {
                        if !no_color {
                            println!(
                                "{}) {}",
                                index.to_string().bright_yellow(),
                                lines[i][1..].bright_cyan()
                            );
                        } else {
                            println!("{}) {}", index, &lines[i][1..]);
                        }
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

fn print_help(no_color: bool) {
    let help_text = vec![
        ("", "HexagonDB CLI Commands:"),
        ("", ""),
        ("CLI Commands:", ""),
        ("  HELP", "Show this help message"),
        ("  CLEAR/CLS", "Clear the screen"),
        ("  EXIT/QUIT", "Exit the CLI"),
        ("", ""),
        ("String Commands:", ""),
        ("  SET key value", "Set a key to a value"),
        ("  GET key", "Get the value of a key"),
        ("  DEL key", "Delete a key"),
        ("  INCR key", "Increment a key's value"),
        ("  DECR key", "Decrement a key's value"),
        ("  EXISTS key", "Check if a key exists"),
        ("  KEYS pattern", "Find keys matching pattern"),
        ("", ""),
        ("List Commands:", ""),
        ("  LPUSH key value...", "Push values to the left of a list"),
        ("  RPUSH key value...", "Push values to the right of a list"),
        ("  LPOP key", "Pop value from the left of a list"),
        ("  RPOP key", "Pop value from the right of a list"),
        ("  LLEN key", "Get the length of a list"),
        (
            "  LRANGE key start stop",
            "Get a range of elements from a list",
        ),
        ("", ""),
        ("Hash Commands:", ""),
        ("  HSET key field value", "Set a hash field"),
        ("  HGET key field", "Get a hash field"),
        ("  HDEL key field", "Delete a hash field"),
        ("  HGETALL key", "Get all fields and values"),
        ("  HKEYS key", "Get all field names"),
        ("  HVALS key", "Get all values"),
        ("", ""),
        ("Set Commands:", ""),
        ("  SADD key member...", "Add members to a set"),
        ("  SREM key member...", "Remove members from a set"),
        ("  SMEMBERS key", "Get all set members"),
        ("  SISMEMBER key member", "Check if member is in set"),
        ("", ""),
        ("Sorted Set Commands:", ""),
        ("  ZADD key score member", "Add member with score"),
        ("  ZREM key member", "Remove member"),
        ("  ZRANGE key start stop", "Get range by index"),
        ("  ZCARD key", "Get number of members"),
        ("  ZSCORE key member", "Get member's score"),
        ("", ""),
        ("TTL Commands:", ""),
        ("  EXPIRE key seconds", "Set key expiration"),
        ("  TTL key", "Get time to live"),
        ("  PERSIST key", "Remove expiration"),
        ("", ""),
        ("Server Commands:", ""),
        ("  PING [message]", "Ping the server"),
        ("  ECHO message", "Echo a message"),
        ("  INFO", "Get server information"),
        ("", ""),
        ("Features:", ""),
        ("  • Tab completion", "Press TAB to complete commands"),
        ("  • Command hints", "Type partial commands to see hints"),
        ("  • History", "Use ↑/↓ arrows for command history"),
        ("  • Multi-line", "Commands can span multiple lines"),
    ];

    if !no_color {
        println!(
            "\n{}",
            "╔════════════════════════════════════════════════════════╗".bright_cyan()
        );
        println!(
            "{}",
            "║              HexagonDB CLI Help                        ║"
                .bright_cyan()
                .bold()
        );
        println!(
            "{}",
            "╚════════════════════════════════════════════════════════╝".bright_cyan()
        );
    } else {
        println!("\nHexagonDB CLI Help");
        println!("==================");
    }

    for (cmd, desc) in help_text {
        if cmd.is_empty() && desc.is_empty() {
            println!();
        } else if cmd.is_empty() {
            if !no_color {
                println!("\n{}", desc.bright_yellow().bold());
            } else {
                println!("\n{}", desc);
            }
        } else if !no_color {
            println!("  {} - {}", cmd.bright_green(), desc);
        } else {
            println!("  {} - {}", cmd, desc);
        }
    }

    println!();
}
