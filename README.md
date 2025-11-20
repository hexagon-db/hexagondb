# HexagonDB

HexagonDB is a fast, in-memory key-value database written in Rust.  
It supports basic commands over a TCP connection: `GET`, `SET`, and `DEL`.

>Use with caution: this project is in early development and not yet stable.

## Features

- In-memory key-value storage
- TCP-based client-server communication
- Basic commands only (`GET`, `SET`, `DEL`)

## Usage

Start the server:

```bash
hexagondb-server
````

Connect with a TCP client (e.g., `nc`):

```bash
nc localhost 2112
```

Example commands:

```
SET key1 value1
GET key1
DEL key1
```

The server responds with the value for `GET` and confirmation for `SET`/`DEL`.

LICENSE: MIT

