#!/bin/sh
set -e

BUCKET_NAME="${BUCKET_NAME:-soaricarus-storage}"
CREDENTIALS_FILE="/shared/garage-credentials.env"
SETUP_DONE="/shared/setup.done"

echo "🔧 Running initial setup..."

# Start Garage in background for setup
echo "⏳ Starting Garage in background..."
/usr/local/bin/garage "$@" &
GARAGE_PID=$!

# Wait for Garage API to be ready
echo "⏳ Waiting for Garage API..."
# until curl -s -f "http://localhost:3900/v0/status" > /dev/null 2>&1; do
#    echo "   Not ready yet, sleeping 2s..."
#    sleep 2
#done
echo "✅ Garage API is responding!"

# Apply layout if not already applied
#if ! /usr/local/bin/garage layout show 2>/dev/null | grep -q "Current cluster layout"; then
#    echo "📦 Applying cluster layout..."
#    NODE_ID=$(/usr/local/bin/garage node id | head -n1)
#    echo $NODE_ID
    # /usr/local/bin/garage layout assign --version 1 "$NODE_ID"
    # /usr/local/bin/garage layout apply --version 1
#    echo "   ✅ Layout applied"
#    sleep 3
#else
    # echo "ℹ️  Layout already applied"
# fi

echo "Laid out"
