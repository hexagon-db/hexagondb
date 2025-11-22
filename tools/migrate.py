#!/usr/bin/env python3
"""
HexagonDB Migration Tool
Migrate data from MongoDB or Redis to HexagonDB
"""

import argparse
import json
import sys

try:
    import redis
    REDIS_AVAILABLE = True
except ImportError:
    REDIS_AVAILABLE = False

try:
    from pymongo import MongoClient
    MONGO_AVAILABLE = True
except ImportError:
    MONGO_AVAILABLE = False


def migrate_from_redis(source_host, source_port, target_host, target_port, password=None):
    """Migrate all keys from Redis to HexagonDB"""
    if not REDIS_AVAILABLE:
        print("❌ Error: redis-py not installed. Run: pip install redis")
        return False
    
    print(f"🔄 Migrating from Redis ({source_host}:{source_port}) to HexagonDB ({target_host}:{target_port})")
    
    try:
        # Connect to source Redis
        source = redis.Redis(host=source_host, port=source_port, decode_responses=True)
        source.ping()
        print("✓ Connected to source Redis")
        
        # Connect to target HexagonDB
        target = redis.Redis(host=target_host, port=target_port, decode_responses=True)
        if password:
            target.auth(password)
        target.ping()
        print("✓ Connected to target HexagonDB")
        
        # Get all keys
        keys = source.keys('*')
        total = len(keys)
        print(f"\n📊 Found {total} keys to migrate")
        
        migrated = 0
        for i, key in enumerate(keys, 1):
            try:
                # Get key type
                key_type = source.type(key)
                
                if key_type == 'string':
                    value = source.get(key)
                    target.set(key, value)
                    
                    # Handle TTL
                    ttl = source.ttl(key)
                    if ttl > 0:
                        target.expire(key, ttl)
                
                elif key_type == 'list':
                    values = source.lrange(key, 0, -1)
                    if values:
                        target.delete(key)
                        target.rpush(key, *values)
                
                elif key_type == 'hash':
                    hash_data = source.hgetall(key)
                    if hash_data:
                        target.delete(key)
                        target.hset(key, mapping=hash_data)
                
                elif key_type == 'set':
                    members = source.smembers(key)
                    if members:
                        target.delete(key)
                        target.sadd(key, *members)
                
                elif key_type == 'zset':
                    members = source.zrange(key, 0, -1, withscores=True)
                    if members:
                        target.delete(key)
                        target.zadd(key, dict(members))
                
                migrated += 1
                if i % 100 == 0:
                    print(f"  Progress: {i}/{total} ({(i/total*100):.1f}%)")
                    
            except Exception as e:
                print(f"  ⚠️  Error migrating key '{key}': {e}")
        
        print(f"\n✅ Migration complete! {migrated}/{total} keys migrated")
        return True
        
    except Exception as e:
        print(f"❌ Migration failed: {e}")
        return False


def migrate_from_mongodb(source_uri, db_name, target_host, target_port, password=None):
    """Migrate collections from MongoDB to HexagonDB"""
    if not MONGO_AVAILABLE:
        print("❌ Error: pymongo not installed. Run: pip install pymongo")
        return False
    
    print(f"🔄 Migrating from MongoDB ({db_name}) to HexagonDB ({target_host}:{target_port})")
    
    try:
        # Connect to source MongoDB
        source_client = MongoClient(source_uri)
        source_db = source_client[db_name]
        print(f"✓ Connected to MongoDB database '{db_name}'")
        
        # Connect to target HexagonDB
        target = redis.Redis(host=target_host, port=target_port, decode_responses=True)
        if password:
            target.auth(password)
        target.ping()
        print("✓ Connected to target HexagonDB")
        
        # Get all collections
        collections = source_db.list_collection_names()
        print(f"\n📊 Found {len(collections)} collections")
        
        total_docs = 0
        for collection_name in collections:
            collection = source_db[collection_name]
            docs = list(collection.find())
            
            print(f"\n  Collection: {collection_name} ({len(docs)} documents)")
            
            for doc in docs:
                # Convert MongoDB document to HexagonDB hash
                doc_id = str(doc.get('_id', ''))
                key = f"{collection_name}:{doc_id}"
                
                # Remove _id from hash fields
                doc_copy = {k: json.dumps(v) if isinstance(v, (dict, list)) else str(v) 
                           for k, v in doc.items() if k != '_id'}
                
                if doc_copy:
                    target.hset(key, mapping=doc_copy)
                    total_docs += 1
            
            # Create index key for collection
            index_key = f"_index:{collection_name}"
            doc_ids = [f"{collection_name}:{doc.get('_id', '')}" for doc in docs]
            if doc_ids:
                target.delete(index_key)
                target.sadd(index_key, *doc_ids)
        
        print(f"\n✅ Migration complete! {total_docs} documents migrated from {len(collections)} collections")
        print(f"\n💡 Query examples:")
        print(f"   HGETALL {collections[0] if collections else 'collection'}:<id>")
        print(f"   SMEMBERS _index:{collections[0] if collections else 'collection'}")
        return True
        
    except Exception as e:
        print(f"❌ Migration failed: {e}")
        return False


def main():
    parser = argparse.ArgumentParser(
        description='Migrate data from MongoDB or Redis to HexagonDB',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Examples:
  # Migrate from Redis
  %(prog)s redis --source-host localhost --source-port 6379 --target-host localhost --target-port 6379

  # Migrate from MongoDB
  %(prog)s mongodb --source-uri mongodb://localhost:27017 --db mydb --target-host localhost --target-port 6379

  # With authentication
  %(prog)s redis --source-host localhost --target-host localhost --password mypass
        '''
    )
    
    subparsers = parser.add_subparsers(dest='source', help='Source database type')
    
    # Redis migration
    redis_parser = subparsers.add_parser('redis', help='Migrate from Redis')
    redis_parser.add_argument('--source-host', default='localhost', help='Source Redis host')
    redis_parser.add_argument('--source-port', type=int, default=6379, help='Source Redis port')
    redis_parser.add_argument('--target-host', default='localhost', help='Target HexagonDB host')
    redis_parser.add_argument('--target-port', type=int, default=6379, help='Target HexagonDB port')
    redis_parser.add_argument('--password', '-a', help='HexagonDB password')
    
    # MongoDB migration
    mongo_parser = subparsers.add_parser('mongodb', help='Migrate from MongoDB')
    mongo_parser.add_argument('--source-uri', default='mongodb://localhost:27017', help='MongoDB connection URI')
    mongo_parser.add_argument('--db', required=True, help='MongoDB database name')
    mongo_parser.add_argument('--target-host', default='localhost', help='Target HexagonDB host')
    mongo_parser.add_argument('--target-port', type=int, default=6379, help='Target HexagonDB port')
    mongo_parser.add_argument('--password', '-a', help='HexagonDB password')
    
    args = parser.parse_args()
    
    if not args.source:
        parser.print_help()
        return 1
    
    print("╔════════════════════════════════════════╗")
    print("║   HexagonDB Migration Tool             ║")
    print("╚════════════════════════════════════════╝\n")
    
    if args.source == 'redis':
        success = migrate_from_redis(
            args.source_host, args.source_port,
            args.target_host, args.target_port,
            args.password
        )
    elif args.source == 'mongodb':
        success = migrate_from_mongodb(
            args.source_uri, args.db,
            args.target_host, args.target_port,
            args.password
        )
    else:
        print(f"❌ Unknown source: {args.source}")
        return 1
    
    return 0 if success else 1


if __name__ == '__main__':
    sys.exit(main())
