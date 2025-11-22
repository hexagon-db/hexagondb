# HexagonDB CLI

Interactive command-line client for HexagonDB.

## Features

- 🎨 Colorized output
- 📝 Command history
- 🔄 Auto-reconnect
- ⚡ RESP protocol support
- 🎯 Tab completion (coming soon)

## Usage

```bash
# Connect to default server (localhost:6379)
hexagondb-cli

# Connect to custom server
hexagondb-cli -h 192.168.1.100 -p 6379

# Execute single command
hexagondb-cli -c "SET mykey myvalue"
```

## Commands

Type `help` in the CLI for available commands.
