#!/bin/bash
# Setup script for sqlx offline mode

set -e

# Database path
DB_PATH="$HOME/.config/cryptofolio/database.sqlite"
export DATABASE_URL="sqlite://$DB_PATH"

echo "🔧 Setting up sqlx for offline compilation..."
echo ""
echo "Database: $DB_PATH"
echo "DATABASE_URL: $DATABASE_URL"
echo ""

# Create database directory if it doesn't exist
mkdir -p "$(dirname "$DB_PATH")"

# Create database
if [ ! -f "$DB_PATH" ]; then
    echo "📦 Creating database..."
    cargo sqlx database create
else
    echo "✓ Database already exists"
fi

# Run migrations (using the app's built-in migrations)
echo "🔄 Running migrations..."
# We'll use the app to run migrations instead of sqlx migrate
# since our migrations are in Rust code, not SQL files

# Prepare sqlx offline data
echo "📋 Preparing sqlx offline query data..."
cargo sqlx prepare --workspace

echo ""
echo "✅ Setup complete!"
echo ""
echo "To compile the project, run:"
echo "  cargo build"
echo ""
echo "Or set DATABASE_URL in your shell:"
echo "  export DATABASE_URL='$DATABASE_URL'"
echo "  cargo build"
