# HexagonDB

HexagonDB is a fast, in-memory key-value database written in Rust.  
It supports concurrent client connections and provides Redis-like commands over TCP using the RESP (Redis Serialization Protocol).

> **Use with caution**: this project is in early development and not yet stable.

## Features

- **In-Memory Key-Value Store**: Fast read/write operations.
- **RESP Protocol Support**: Compatible with standard Redis clients (e.g., `redis-cli`).
- **Data Types**:
  - **String**: Basic key-value pairs.
  - **List**: Linked list operations (`LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `LLEN`, `LRANGE`).
  - **Hash**: Field-value maps (`HSET`, `HGET`, `HGETALL`, `HDEL`).
- **TTL & Expiration**: Set expiration times on keys (`EXPIRE`, `TTL`, `PERSIST`).
- **Persistence (AOF)**: Append-Only File persistence ensures data durability across restarts.
- **Concurrency**: Handles multiple clients simultaneously using a thread-per-connection model.
- **Thread Safety**: Uses `Arc<Mutex<DB>>` for safe concurrent access.

## Commands

### String Operations
- `SET key value`: Set the value of a key.
- `GET key`: Get the value of a key.
- `DEL key`: Delete a key.
- `EXISTS key`: Check if a key exists.
- `INCR key`: Increment the integer value of a key.
- `DECR key`: Decrement the integer value of a key.

### List Operations
- `LPUSH key value [value ...]`: Prepend one or multiple values to a list.
- `RPUSH key value [value ...]`: Append one or multiple values to a list.
- `LPOP key`: Remove and get the first element in a list.
- `RPOP key`: Remove and get the last element in a list.
- `LLEN key`: Get the length of a list.
- `LRANGE key start stop`: Get a range of elements from a list.

### Hash Operations
- `HSET key field value`: Set the string value of a hash field.
- `HGET key field`: Get the value of a hash field.
- `HGETALL key`: Get all fields and values in a hash.
- `HDEL key field`: Delete one or more hash fields.

### Key Management
- `KEYS pattern`: Find all keys matching the given pattern.
- `EXPIRE key seconds`: Set a key's time to live in seconds.
- `TTL key`: Get the time to live for a key.
- `PERSIST key`: Remove the expiration from a key.

## Usage

### Starting the Server
```bash
cargo run --release
```
The server listens on `127.0.0.1:2112`.

### Connecting with redis-cli
You can use the standard `redis-cli` tool to connect:
```bash
redis-cli -p 2112
```

### Example Session
```bash
$ redis-cli -p 2112
127.0.0.1:2112> SET mykey "Hello"
OK
127.0.0.1:2112> GET mykey
"Hello"
127.0.0.1:2112> EXPIRE mykey 10
(integer) 1
127.0.0.1:2112> TTL mykey
(integer) 8
127.0.0.1:2112> LPUSH mylist a b c
(integer) 3
127.0.0.1:2112> LRANGE mylist 0 -1
1) "c"
2) "b"
3) "a"
127.0.0.1:2112> HSET myhash field1 "value1"
(integer) 1
127.0.0.1:2112> HGETALL myhash
1) "field1"
2) "value1"
```

## Architecture

- **Thread-per-connection**: Each client runs in a dedicated thread.
- **Shared database access**: Database wrapped in `Arc<Mutex<>>` for thread safety.
- **RESP Protocol**: Implements Redis Serialization Protocol for broad client compatibility.
- **AOF Persistence**: Writes all state-changing commands to `database.aof` for durability.
- **Lazy Expiration**: Keys are checked for expiration on access.

## License

MIT
