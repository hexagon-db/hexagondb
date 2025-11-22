# HexagonDB Migration Tool

Migrate data from MongoDB or Redis to HexagonDB.

## Installation

```bash
pip install redis pymongo
```

## Usage

### Migrate from Redis

```bash
python tools/migrate.py redis \
  --source-host localhost \
  --source-port 6379 \
  --target-host localhost \
  --target-port 6379
```

### Migrate from MongoDB

```bash
python tools/migrate.py mongodb \
  --source-uri mongodb://localhost:27017 \
  --db mydb \
  --target-host localhost \
  --target-port 6379
```

### With Authentication

```bash
python tools/migrate.py redis \
  --source-host localhost \
  --target-host localhost \
  --password mypassword
```

## Features

- ✅ Migrates all Redis data types (string, list, hash, set, zset)
- ✅ Preserves TTL values
- ✅ MongoDB collections → HexagonDB hashes
- ✅ Progress tracking
- ✅ Error handling

## MongoDB Migration Details

MongoDB documents are converted to HexagonDB hashes:

```
MongoDB: mydb.users.find({_id: "123"})
  → HexagonDB: HGETALL users:123

MongoDB collections are indexed:
  → HexagonDB: SMEMBERS _index:users
```

## Examples

```bash
# Migrate entire Redis database
python tools/migrate.py redis

# Migrate specific MongoDB database
python tools/migrate.py mongodb --db production

# Migrate with custom ports
python tools/migrate.py redis --source-port 6380 --target-port 6379
```
