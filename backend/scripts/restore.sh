#!/bin/bash

# Load environment variables from .env
if [ -f .env ]; then
  export $(cat .env | grep -v '#' | xargs)
else
  echo "Error: .env file not found"
  exit 1
fi

# Check if archive path is provided
if [ -z "$1" ]; then
  echo "Usage: ./restore.sh <path-to-backup-archive.tar.gz> [collection1] [collection2] ..."
  echo "Example: ./restore.sh ~/Downloads/ktt_prod_20260513_143022.tar.gz"
  echo "Example: ./restore.sh ~/Downloads/ktt_prod_20260513_143022.tar.gz users posts"
  exit 1
fi

ARCHIVE_PATH="$1"
shift  # Remove first argument, rest are collection names
RESTORE_COLLECTIONS="$@"

# Check if archive exists
if [ ! -f "$ARCHIVE_PATH" ]; then
  echo "Error: Archive file not found: $ARCHIVE_PATH"
  exit 1
fi

# Extract database name from archive filename
# Expected format: database_name_YYYYMMDD_HHMMSS.tar.gz
ARCHIVE_NAME=$(basename "$ARCHIVE_PATH" .tar.gz)
DB_NAME=$(echo "$ARCHIVE_NAME" | sed 's/_[0-9]\{8\}_[0-9]\{6\}$//')

if [ -z "$DB_NAME" ]; then
  echo "Error: Could not extract database name from archive filename"
  exit 1
fi

# Extract database name from MONGODB_URI for comparison
URI_DB_NAME=$(echo "$MONGODB_URI" | sed 's|.*://[^/]*/||')

echo "=========================================="
echo "MongoDB Restore Script"
echo "=========================================="
echo "Archive: $ARCHIVE_PATH"
echo "Database to restore: $DB_NAME"
echo "Target database: $URI_DB_NAME"
echo "MongoDB URI: $MONGODB_URI"
echo ""

# Check if MongoDB is running
echo "Checking MongoDB connection..."
if ! mongosh "$MONGODB_URI" --eval "db.adminCommand('ping')" > /dev/null 2>&1; then
  echo "✗ Error: Cannot connect to MongoDB at $MONGODB_URI"
  echo "Make sure MongoDB is running and accessible."
  exit 1
fi
echo "✓ MongoDB is running and accessible"
echo ""

# Create temporary directory for extraction
TEMP_DIR=$(mktemp -d)
echo "Extracting archive to temporary directory..."

# Extract archive
tar -xzf "$ARCHIVE_PATH" -C "$TEMP_DIR"

if [ $? -ne 0 ]; then
  echo "✗ Error: Failed to extract archive"
  rm -rf "$TEMP_DIR"
  exit 1
fi

echo "✓ Archive extracted"

# Find the dump directory - it could be nested differently
# The archive has structure: timestamp_folder/database_name/
# mongorestore needs the parent directory of the database folder

DUMP_PARENT=""

# Look for any directory containing .bson files at depth 2
BSON_FILE=$(find "$TEMP_DIR" -type f -name "*.bson" | head -1)

if [ -z "$BSON_FILE" ]; then
  echo "✗ Error: Could not find database dump in archive"
  echo "Archive structure:"
  find "$TEMP_DIR" -type f | head -10
  rm -rf "$TEMP_DIR"
  exit 1
fi

# Get the parent directory of the .bson file (the database directory)
DB_DIR=$(dirname "$BSON_FILE")
# Get the parent of that (the dump root)
DUMP_PARENT=$(dirname "$DB_DIR")

# Extract the actual database name from the directory structure
ACTUAL_DB_NAME=$(basename "$DB_DIR")

echo ""

# Determine restore mode
if [ -z "$RESTORE_COLLECTIONS" ]; then
  echo "No collections specified. Restoring ALL collections."
  RESTORE_MODE="all"
else
  echo "Restoring specific collections: $RESTORE_COLLECTIONS"
  RESTORE_MODE="selective"
fi

echo ""

# Ask about merge vs replace
read -p "Merge (m) or Replace (r)? [m]: " merge_choice

merge_choice=${merge_choice:-m}  # Default to 'm' if empty

if [ "$merge_choice" = "r" ]; then
  # Confirm replace
  read -p "⚠️  WARNING: This will OVERWRITE the selected collections. Type 'yes' to confirm: " confirmation
  
  if [ "$confirmation" != "yes" ]; then
    echo "Restore cancelled."
    rm -rf "$TEMP_DIR"
    exit 0
  fi
  
  REPLACE_MODE=true
elif [ "$merge_choice" = "m" ]; then
  REPLACE_MODE=false
else
  echo "Invalid choice. Using merge mode (default)."
  REPLACE_MODE=false
fi

echo ""
echo "Restoring database..."

# Restore with appropriate flags
if [ "$REPLACE_MODE" = true ]; then
  # Replace mode: use --drop
  if [ "$RESTORE_MODE" = "all" ]; then
    mongorestore --uri="$MONGODB_URI" --drop "$DB_DIR"
  else
    mongorestore --uri="$MONGODB_URI" --drop "$DB_DIR" $(for col in $RESTORE_COLLECTIONS; do echo "--nsInclude=$ACTUAL_DB_NAME.$col"; done)
  fi
else
  # Merge mode: no --drop
  if [ "$RESTORE_MODE" = "all" ]; then
    mongorestore --uri="$MONGODB_URI" "$DB_DIR"
  else
    mongorestore --uri="$MONGODB_URI" "$DB_DIR" $(for col in $RESTORE_COLLECTIONS; do echo "--nsInclude=$ACTUAL_DB_NAME.$col"; done)
  fi
fi

if [ $? -ne 0 ]; then
  echo "✗ Error: Restore failed"
  rm -rf "$TEMP_DIR"
  exit 1
fi

echo "✓ Database restored successfully"

# Clean up temporary directory
rm -rf "$TEMP_DIR"

# Verify restore by querying the database
echo ""
echo "Verifying restore..."
echo ""

echo "Collections in database '$URI_DB_NAME':"
mongosh "$MONGODB_URI" --eval "db.getCollectionNames().forEach(col => print('  - ' + col))" --quiet

echo ""
echo "Sample document counts:"
mongosh "$MONGODB_URI" --eval "
db.getCollectionNames().forEach(col => {
  const count = db[col].countDocuments();
  print('  ' + col + ': ' + count + ' documents');
});
" --quiet

echo ""
echo "=========================================="
echo "✓ Restore completed and verified!"
echo "=========================================="
