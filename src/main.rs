use std::net::TcpListener;

use hexagondb::{database::DB, interpreter,connection};
fn main() -> std::io::Result<()>  {
    let mut db: DB = DB::new();
    let mut client:interpreter::Interpreter = interpreter::Interpreter::new(db);

    let listener = TcpListener::bind("127.0.0.1:2112")?;
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New client connected");
                connection::handle_client(stream,&mut client);
            }
            Err(e) => println!("Connection failed: {}", e),
        }
    }

    Ok(())
}
