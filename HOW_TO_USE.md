# HexagonDB - Complete User Guide

## Table of Contents
1. [Quick Start](#quick-start)
2. [Installation](#installation)
3. [Basic Usage](#basic-usage)
4. [Security](#security)
5. [Data Types \u0026 Commands](#data-types--commands)
6. [Production Deployment](#production-deployment)
7. [Web Integration](#web-integration)
8. [Migration](#migration)
9. [Troubleshooting](#troubleshooting)

---

## Quick Start

### For Developers
```bash
# Build and install
make install-global
source ~/.zshrc

# Start server
hexagondb

# Connect with CLI
hexagondb-cli
```

### For Users
```bash
# Connect to HexagonDB
hexagondb-cli -h localhost -p 6379

# With password
hexagondb-cli --ask-pass

# Execute single command
hexagondb-cli -c "SET mykey myvalue"
```

---

## Installation

### From Source
```bash
git clone https://github.com/ibrahmsql/hexagondb.git
cd hexagondb
make build-all
sudo make install-global
```

### Manual Build
```bash
# Build server
cargo build --release

# Build CLI
cd cli && cargo build --release

# Add to PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Verify Installation
```bash
hexagondb --version
hexagondb-cli --help
```

---

## Basic Usage

### Starting the Server
```bash
# Default (port 6379)
hexagondb

# Custom port (edit source)
# Port is hardcoded to 6379 for Redis compatibility
```

### Using the CLI

#### Interactive Mode
```bash
hexagondb-cli

hexagondb> SET user:1 "John Doe"
OK
hexagondb> GET user:1
John Doe
hexagondb> DEL user:1
OK
```

#### Single Command Mode
```bash
hexagondb-cli -c "PING"
hexagondb-cli -c "SET key value"
hexagondb-cli -c "GET key"
```

#### With Authentication
```bash
# Prompt for password (secure)
hexagondb-cli --ask-pass

# Direct password (less secure)
hexagondb-cli -a mypassword

# In session
hexagondb> AUTH mypassword
OK
```

---

## Security

### Password Authentication

**Setup:**
Currently, password is validated against hardcoded AUTH command. For production, implement proper user management.

**Usage:**
```bash
# Secure input
hexagondb-cli --ask-pass
Password: ********

# Command line (visible in history - not recommended)
hexagondb-cli -a mypass
```

### Brute Force Protection

**Built-in Protection:**
- ✅ **IMPLEMENTED** - Password attempts limited to 3 tries
- ✅ Auto-disconnect after failed attempts
- ✅ Attempt counter displayed (1/3, 2/3, 3/3)
- ⚠️ Temporary IP blocking (future feature)

**How It Works:**
```bash
hexagondb-cli --ask-pass
Password: ********
✗ Authentication failed: ERR invalid password (1/3)
Password: ********
✗ Authentication failed: ERR invalid password (2/3)
Password: ********
✗ Maximum authentication attempts reached! Disconnecting...
```

**Current Protection:**
- Maximum 3 password attempts per connection
- Automatic disconnect on failure
- User must reconnect to retry
- Prevents automated brute force attacks

**Recommended Protection (Manual):**
```bash
# Use fail2ban
sudo apt-get install fail2ban

# Configure for HexagonDB (create /etc/fail2ban/jail.d/hexagondb.conf)
[hexagondb]
enabled = true
port = 6379
filter = hexagondb
logpath = /var/log/hexagondb.log
maxretry = 3
bantime = 3600
```

### Network Security

**Firewall Rules:**
```bash
# Allow only localhost
sudo ufw allow from 127.0.0.1 to any port 6379

# Allow specific IP
sudo ufw allow from 192.168.1.100 to any port 6379

# Allow subnet
sudo ufw allow from 192.168.1.0/24 to any port 6379
```

**Bind to Localhost Only:**
Currently binds to `127.0.0.1:6379`. For network access, modify source code.

---

## Data Types \u0026 Commands

### String Operations
```bash
# Set and get
SET mykey "hello world"
GET mykey

# Multiple keys
SET user:1 "Alice"
SET user:2 "Bob"
SET user:3 "Charlie"

# Pattern matching
KEYS user:*
# Returns: user:1, user:2, user:3

# All keys
KEYS *

# Increment/Decrement
SET counter 10
INCR counter    # 11
DECR counter    # 10

# Check existence
EXISTS mykey    # 1 (exists)
EXISTS nokey    # 0 (doesn't exist)

# Delete
DEL mykey
```

### List Operations
```bash
# Push to list
LPUSH mylist "first"
RPUSH mylist "last"

# Pop from list
LPOP mylist
RPOP mylist

# Get length
LLEN mylist

# Get range
LRANGE mylist 0 -1    # All elements
LRANGE mylist 0 10    # First 10
```

### Hash Operations
```bash
# Set hash fields
HSET user:1 name "John"
HSET user:1 email "john@example.com"
HSET user:1 age "30"

# Get specific field
HGET user:1 name

# Get all fields and values
HGETALL user:1

# Get all keys
HKEYS user:1

# Get all values
HVALS user:1

# Delete field
HDEL user:1 age
```

### Set Operations
```bash
# Add to set
SADD myset "apple"
SADD myset "banana"
SADD myset "cherry"

# Get all members
SMEMBERS myset

# Check membership
SISMEMBER myset "apple"    # 1 (exists)
SISMEMBER myset "grape"    # 0 (doesn't exist)

# Remove from set
SREM myset "banana"
```

### Sorted Set Operations
```bash
# Add with score
ZADD leaderboard 100 "player1"
ZADD leaderboard 95 "player2"
ZADD leaderboard 110 "player3"

# Get range (by rank)
ZRANGE leaderboard 0 -1

# Get score
ZSCORE leaderboard "player1"

# Get count
ZCARD leaderboard

# Remove member
ZREM leaderboard "player2"
```

### TTL (Time-To-Live)
```bash
# Set expiration (seconds)
SET session:abc "user123"
EXPIRE session:abc 3600    # Expires in 1 hour

# Check TTL
TTL session:abc            # Returns remaining seconds

# Remove expiration
PERSIST session:abc        # Never expires

# Set with expiration
SET temp:data "value"
EXPIRE temp:data 60       # Expires in 60 seconds
```

### Server Commands
```bash
# Ping
PING
# Returns: PONG

# Echo
ECHO "Hello"
# Returns: Hello

# Server info
INFO
# Returns: Server version, uptime, etc.
```

---

## Production Deployment

### System Requirements
```
- OS: Linux (Ubuntu 20.04+, CentOS 7+)
- RAM: Minimum 1GB, Recommended 4GB+
- CPU: 2+ cores recommended
- Disk: SSD recommended for AOF persistence
```

### Production Setup

#### 1. Build Optimized Binary
```bash
cargo build --release --target x86_64-unknown-linux-gnu
strip target/release/hexagondb  # Reduce binary size
```

#### 2. Create System User
```bash
sudo useradd -r -s /bin/false hexagondb
sudo mkdir /var/lib/hexagondb
sudo chown hexagondb:hexagondb /var/lib/hexagondb
```

#### 3. Create Systemd Service
Create `/etc/systemd/system/hexagondb.service`:
```ini
[Unit]
Description=HexagonDB Server
After=network.target

[Service]
Type=simple
User=hexagondb
Group=hexagondb
WorkingDirectory=/var/lib/hexagondb
ExecStart=/usr/local/bin/hexagondb
Restart=always
RestartSec=10
StandardOutput=syslog
StandardError=syslog
SyslogIdentifier=hexagondb

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/hexagondb

[Install]
WantedBy=multi-user.target
```

#### 4. Start Service
```bash
sudo systemctl daemon-reload
sudo systemctl enable hexagondb
sudo systemctl start hexagondb
sudo systemctl status hexagondb
```

#### 5. Log Rotation
Create `/etc/logrotate.d/hexagondb`:
```
/var/log/hexagondb.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0640 hexagondb hexagondb
    postrotate
        systemctl reload hexagondb > /dev/null 2>&1 || true
    endscript
}
```

### Monitoring

#### Health Check
```bash
# Simple check
hexagondb-cli -c PING

# With monitoring
while true; do
  hexagondb-cli -c PING || echo "Server down!"
  sleep 60
done
```

#### Metrics (Future)
```bash
# Get server stats
hexagondb-cli -c INFO

# Watch key count
watch -n 1 'hexagondb-cli -c "KEYS *" | wc -l'
```

### Backup

#### AOF Persistence
```bash
# AOF file location
/var/lib/hexagondb/database.aof

# Backup AOF
cp /var/lib/hexagondb/database.aof /backup/hexagondb-$(date +%Y%m%d).aof

# Automated backup
0 2 * * * cp /var/lib/hexagondb/database.aof /backup/hexagondb-$(date +\%Y\%m\%d).aof
```

### High Availability

#### Load Balancing (Redis Proxy)
```bash
# Use HAProxy or Twemproxy
# Example HAProxy config:
frontend hexagondb_front
    bind *:6380
    default_backend hexagondb_back

backend hexagondb_back
    balance roundrobin
    server hex1 127.0.0.1:6379 check
    server hex2 127.0.0.1:6380 check
```

#### Replication (Future)
```
⚠️ Master-slave replication coming soon!
```

---

## Web Integration

### Node.js
```javascript
const redis = require('redis');

// Connect
const client = redis.createClient({
  host: 'localhost',
  port: 6379,
  password: 'yourpassword' // if auth enabled
});

// Basic operations
await client.connect();

// Set
await client.set('user:1', 'John Doe');

// Get
const value = await client.get('user:1');
console.log(value); // John Doe

// Hash
await client.hSet('user:2', {
  name: 'Jane',
  email: 'jane@example.com'
});

// Get all hash
const user = await client.hGetAll('user:2');
console.log(user); // { name: 'Jane', email: 'jane@example.com' }

// Close
await client.disconnect();
```

### Python
```python
import redis

# Connect
r = redis.Redis(
    host='localhost',
    port=6379,
    password='yourpassword',  # if auth enabled
    decode_responses=True
)

# Basic operations
r.set('user:1', 'John Doe')
value = r.get('user:1')
print(value)  # John Doe

# Hash
r.hset('user:2', mapping={
    'name': 'Jane',
    'email': 'jane@example.com'
})

user = r.hgetall('user:2')
print(user)  # {'name': 'Jane', 'email': 'jane@example.com'}

# Pattern matching
keys = r.keys('user:*')
print(keys)  # ['user:1', 'user:2']
```

### PHP
```php
<?php
$redis = new Redis();
$redis->connect('127.0.0.1', 6379);

// Auth if needed
// $redis->auth('yourpassword');

// Basic operations
$redis->set('user:1', 'John Doe');
$value = $redis->get('user:1');
echo $value; // John Doe

// Hash
$redis->hMSet('user:2', [
    'name' => 'Jane',
    'email' => 'jane@example.com'
]);

$user = $redis->hGetAll('user:2');
print_r($user);

// Pattern matching
$keys = $redis->keys('user:*');
print_r($keys);
?>
```

### Go
```go
package main

import (
    "context"
    "fmt"
    "github.com/go-redis/redis/v8"
)

func main() {
    ctx := context.Background()

    rdb := redis.NewClient(&redis.Options{
        Addr:     "localhost:6379",
        Password: "yourpassword", // if auth enabled
        DB:       0,
    })

    // Set
    err := rdb.Set(ctx, "user:1", "John Doe", 0).Err()
    if err != nil {
        panic(err)
    }

    // Get
    val, err := rdb.Get(ctx, "user:1").Result()
    if err != nil {
        panic(err)
    }
    fmt.Println(val) // John Doe

    // Pattern matching
    keys, err := rdb.Keys(ctx, "user:*").Result()
    if err != nil {
        panic(err)
    }
    fmt.Println(keys)
}
```

### Session Store (Express.js)
```javascript
const session = require('express-session');
const RedisStore = require('connect-redis')(session);
const { createClient } = require('redis');

const redisClient = createClient({
  host: 'localhost',
  port: 6379,
  legacyMode: true
});

redisClient.connect().catch(console.error);

app.use(session({
  store: new RedisStore({ client: redisClient }),
  secret: 'your-secret-key',
  resave: false,
  saveUninitialized: false,
  cookie: {
    secure: false, // true in production with HTTPS
    httpOnly: true,
    maxAge: 1000 * 60 * 60 * 24 // 24 hours
  }
}));
```

---

## Migration

### From MongoDB
```bash
python tools/migrate.py mongodb \
  --source-uri mongodb://localhost:27017 \
  --db mydb \
  --target-host localhost \
  --target-port 6379
```

### From Redis
```bash
python tools/migrate.py redis \
  --source-host localhost \
  --source-port 6379 \
  --target-host localhost \
  --target-port 6380
```

### Manual Migration
```bash
# Export from Redis
redis-cli --rdb dump.rdb

# Import to HexagonDB (use migration tool)
python tools/migrate.py redis
```

---

## Troubleshooting

### Server Won't Start
```bash
# Check if port is in use
lsof -i :6379

# Kill existing process
sudo kill -9 $(lsof -t -i:6379)

# Check logs
journalctl -u hexagondb -f
```

### Connection Refused
```bash
# Check if server is running
ps aux | grep hexagondb

# Check firewall
sudo ufw status

# Test connection
telnet localhost 6379
```

### Slow Performance
```bash
# Check memory usage
free -h

# Check CPU
top -p $(pgrep hexagondb)

# Optimize AOF
# Currently no BGREWRITEAOF command
# Restart server to compact AOF
```

### Data Loss
```bash
# Check AOF file
ls -lh /var/lib/hexagondb/database.aof

# Restore from backup
cp /backup/hexagondb-20240101.aof /var/lib/hexagondb/database.aof
sudo systemctl restart hexagondb
```

### Authentication Issues
```bash
# Test auth
hexagondb-cli
hexagondb> AUTH wrongpassword
ERR invalid password

hexagondb> AUTH correctpassword
OK
```

---

## Advanced Features

### Hidden Vim Commands (Easter Egg)
```bash
# In CLI, try these:
:q          # Quit
:w          # Save history
:help       # Show help
:clear      # Clear screen

# These are NOT announced in welcome message!
```

### Tab Completion
```bash
# Type partial command and press TAB
hexagondb> get<TAB>
# Completes to: get (lowercase)

hexagondb> GET<TAB>
# Completes to: GET (uppercase)

# Works for all commands!
```

### Command History
```bash
# Use arrow keys
↑  # Previous command
↓  # Next command

# History is saved to ~/.hexagondb_history
```

---

## Performance Tips

1. **Use Hashes for Objects**
   ```bash
   # Instead of multiple keys:
   SET user:1:name "John"
   SET user:1:email "john@example.com"

   # Use hash:
   HSET user:1 name "John" email "john@example.com"
   ```

2. **Use Pipelining** (in code)
   ```python
   pipe = r.pipeline()
   pipe.set('key1', 'value1')
   pipe.set('key2', 'value2')
   pipe.execute()
   ```

3. **Set Appropriate TTLs**
   ```bash
   # Cache for 1 hour
   SET cache:data "value"
   EXPIRE cache:data 3600
   ```

4. **Use Pattern Matching Wisely**
   ```bash
   # Specific pattern
   KEYS user:*    # Good

   # Too broad
   KEYS *         # Use sparingly
   ```

---

## Support

- GitHub Issues: https://github.com/ibrahmsql/hexagondb/issues
- Documentation: This file!
- Migration Tool: `tools/README.md`

---

**Made with ❤️ by ibrahmsql**
