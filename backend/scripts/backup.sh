#!/bin/bash

# Load environment variables from .env
if [ -f .env ]; then
  export $(cat .env | grep -v '#' | xargs)
else
  echo "Error: .env file not found"
  exit 1
fi

# Extract database name from MONGODB_URI
# Format: mongodb://host:port/database_name
DB_NAME=$(echo "$MONGODB_URI" | sed 's|.*://[^/]*/||')

if [ -z "$DB_NAME" ]; then
  echo "Error: Could not extract database name from MONGODB_URI"
  exit 1
fi

# Check if MongoDB is running
if ! mongosh "$MONGODB_URI" --eval "db.adminCommand('ping')" > /dev/null 2>&1; then
  echo "✗ Error: Cannot connect to MongoDB at $MONGODB_URI"
  echo "Make sure MongoDB is running and accessible."
  exit 1
fi

echo "=========================================="
echo "MongoDB Backup Script"
echo "=========================================="
echo "Database: $DB_NAME"
echo ""

# Get all collections
echo "Available collections:"
COLLECTIONS=$(mongosh "$MONGODB_URI" --eval "db.getCollectionNames().forEach(col => print(col))" --quiet)
mongosh "$MONGODB_URI" --eval "db.getCollectionNames().forEach(col => print('  - ' + col))" --quiet

echo ""

# Parse arguments for specific collections
if [ $# -eq 0 ]; then
  echo "No collections specified. Backing up ALL collections."
  BACKUP_COLLECTIONS=""
else
  echo "Backing up specific collections: $@"
  BACKUP_COLLECTIONS="$@"
fi

echo ""

# Create backup directory with timestamp
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="./backups/${DB_NAME}_${TIMESTAMP}"
BACKUP_ARCHIVE="${BACKUP_DIR}.tar.gz"

echo "Backup location: $BACKUP_ARCHIVE"
echo ""

# Run mongodump with optional collection filter
if [ -z "$BACKUP_COLLECTIONS" ]; then
  # Backup all collections
  mongodump --uri="$MONGODB_URI" --out="$BACKUP_DIR"
else
  # Backup specific collections
  mongodump --uri="$MONGODB_URI" --out="$BACKUP_DIR" $(for col in $BACKUP_COLLECTIONS; do echo "--collection=$col"; done)
fi

if [ $? -eq 0 ]; then
  # Compress the backup
  tar -czf "$BACKUP_ARCHIVE" -C ./backups "${DB_NAME}_${TIMESTAMP}"
  
  if [ $? -eq 0 ]; then
    # Remove the uncompressed directory
    rm -rf "$BACKUP_DIR"
    echo "✓ Backup completed successfully"
    echo "Database backed up to: $BACKUP_ARCHIVE"
  else
    echo "✗ Compression failed"
    exit 1
  fi
else
  echo "✗ Backup failed"
  exit 1
fi
