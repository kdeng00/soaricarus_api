#!/bin/sh
set -e

BUCKET_NAME="${BUCKET_NAME:-soaricarus-storage}"
CREDENTIALS_FILE="/shared/garage-credentials.env"
SETUP_DONE="/shared/setup.done"

# Function to write credentials to shared volume
write_credentials() {
    local access_key="$1"
    local secret_key="$2"
    mkdir -p "$(dirname "$CREDENTIALS_FILE")"
    cat > "$CREDENTIALS_FILE" << EOF
GARAGE_ACCESS_KEY_ID=$access_key
GARAGE_SECRET_ACCESS_KEY=$secret_key
GARAGE_BUCKET_NAME=$BUCKET_NAME
GARAGE_ENDPOINT=http://maze:3900
EOF
    touch "$SETUP_DONE"
    echo "✅ Credentials saved to $CREDENTIALS_FILE"
}

# Check if setup already done
if [ -f "$SETUP_DONE" ]; then
    echo "ℹ️  Setup already completed. Starting Garage..."
    # Source credentials if needed for the main Garage process
    # shellcheck source=/dev/null
    [ -f "$CREDENTIALS_FILE" ] && . "$CREDENTIALS_FILE"
    exec /usr/local/bin/garage "$@"
fi

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
if ! /usr/local/bin/garage layout show 2>/dev/null | grep -q "Current cluster layout"; then
    echo "📦 Applying cluster layout..."
    NODE_ID=$(/usr/local/bin/garage node id | head -n1)
    /usr/local/bin/garage layout assign --version 1 "$NODE_ID"
    /usr/local/bin/garage layout apply --version 1
    echo "   ✅ Layout applied"
    sleep 3
else
    echo "ℹ️  Layout already applied"
fi

echo "Laid out"

# Create bucket if it doesn't exist
if ! /usr/local/bin/garage bucket info "$BUCKET_NAME" 2>/dev/null; then
    echo "📦 Creating bucket: $BUCKET_NAME"
    /usr/local/bin/garage bucket create "$BUCKET_NAME"
else
    echo "ℹ️  Bucket $BUCKET_NAME already exists"
fi

# Create or import key
# We'll use the credentials from .env if provided, otherwise generate new ones
if [ -n "$AWS_ACCESS_KEY_ID" ] && [ -n "$AWS_SECRET_ACCESS_KEY" ]; then
    echo "🔑 Using pre-defined AWS credentials from .env"
    if ! /usr/local/bin/garage key info "$AWS_ACCESS_KEY_ID" 2>/dev/null; then
        # Key doesn't exist, import it
        /usr/local/bin/garage key import "$AWS_ACCESS_KEY_ID" "$AWS_SECRET_ACCESS_KEY" "app-key"
        echo "   ✅ Key imported: $AWS_ACCESS_KEY_ID"
    else
        echo "   ℹ️  Key already exists: $AWS_ACCESS_KEY_ID"
    fi
    ACCESS_KEY="$AWS_ACCESS_KEY_ID"
    SECRET_KEY="$AWS_SECRET_ACCESS_KEY"
else
    echo "🔑 Generating new key..."
    KEY_OUTPUT=$(/usr/local/bin/garage key create --name "app-key")
    echo "$KEY_OUTPUT"
    ACCESS_KEY=$(echo "$KEY_OUTPUT" | grep 'Key ID:' | awk '{print $3}')
    SECRET_KEY=$(echo "$KEY_OUTPUT" | grep 'Secret key:' | awk '{print $3}')
    echo "   ✅ Key generated: $ACCESS_KEY"
fi

# Grant permissions
/usr/local/bin/garage bucket allow --read --write --owner "$BUCKET_NAME" --key "$ACCESS_KEY"
echo "   ✅ Permissions granted"

# Write credentials to shared volume
write_credentials "$ACCESS_KEY" "$SECRET_KEY"

echo "🎉 Setup complete!"
echo "   Bucket: $BUCKET_NAME"
echo "   Access Key: $ACCESS_KEY"

# Stop the background Garage process
echo "⏹️  Stopping background Garage process..."
kill "$GARAGE_PID"
wait "$GARAGE_PID" 2>/dev/null || true

echo "🚀 Starting Garage in foreground..."
exec /usr/local/bin/garage "$@"

