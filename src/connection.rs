use std::net::{TcpListener, TcpStream};
use std::io::{Read,Write};
use crate::{interpreter};

pub fn handle_client(mut stream: TcpStream, interpreter: &mut interpreter::Interpreter) {
    let mut buffer = [0u8; 1024];

    loop {
        let n = match stream.read(&mut buffer) {
            Ok(0) => return,
            Ok(n) => n,
            Err(_) => return,
        };
        let response: String = interpreter.exec(String::from_utf8_lossy(&buffer[..n]).to_string().trim().to_string());
        
        if let Err(e) = stream.write_all(response.as_bytes()) {
            println!("Error sending response: {}", e);
            return;
        }
        
        let _ = stream.write_all(b"\n");
    }
}