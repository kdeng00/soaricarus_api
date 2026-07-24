#!/garage
# This script runs as a garage command
# It will check if setup is needed and run initialization

# Check if we should run setup (first run)
SETUP_MARKER="/var/lib/garage/.setup_done"
BUCKET_NAME="${BUCKET_NAME:-soaricarus-storage}"
CREDENTIALS_FILE="/shared/garage-credentials.env"

# Function to check if Garage API is ready
is_garage_ready() {
    # Try to get status, if it works, Garage is ready
    /garage status 2>/dev/null | grep -q "Health: ok"
    return $?
}

# Function to wait for Garage to be ready
wait_for_garage() {
    echo "⏳ Waiting for Garage to be ready..."
    local max_attempts=30
    local attempt=0
    while [ $attempt -lt $max_attempts ]; do
        if is_garage_ready; then
            echo "✅ Garage is ready!"
            return 0
        fi
        attempt=$((attempt + 1))
        echo "   Waiting... (attempt $attempt/$max_attempts)"
        sleep 2
    done
    echo "❌ Garage failed to become ready"
    return 1
}

# Function to run setup
run_setup() {
    echo "🔧 Running initial setup..."
    
    # Get node ID
    NODE_ID=$(/garage node id | head -n1)
    echo "   Node ID: $NODE_ID"
    
    # Apply layout
    /garage layout assign --version 1 "$NODE_ID"
    /garage layout apply --version 1
    echo "   ✅ Layout applied"
    sleep 3
    
    # Create bucket
    if ! /garage bucket info "$BUCKET_NAME" 2>/dev/null; then
        /garage bucket create "$BUCKET_NAME"
        echo "   ✅ Bucket created: $BUCKET_NAME"
    else
        echo "   ℹ️  Bucket already exists"
    fi
    
    # Handle key
    if [ -n "$AWS_ACCESS_KEY_ID" ] && [ -n "$AWS_SECRET_ACCESS_KEY" ]; then
        # Import existing key
        if ! /garage key info "$AWS_ACCESS_KEY_ID" 2>/dev/null; then
            /garage key import "$AWS_ACCESS_KEY_ID" "$AWS_SECRET_ACCESS_KEY" "app-key"
            echo "   ✅ Key imported: $AWS_ACCESS_KEY_ID"
        else
            echo "   ℹ️  Key already exists: $AWS_ACCESS_KEY_ID"
        fi
        ACCESS_KEY="$AWS_ACCESS_KEY_ID"
        SECRET_KEY="$AWS_SECRET_ACCESS_KEY"
    else
        # Generate new key
        KEY_OUTPUT=$(/garage key create --name "app-key")
        echo "$KEY_OUTPUT"
        ACCESS_KEY=$(echo "$KEY_OUTPUT" | grep 'Key ID:' | awk '{print $3}')
        SECRET_KEY=$(echo "$KEY_OUTPUT" | grep 'Secret key:' | awk '{print $3}')
        echo "   ✅ Key generated: $ACCESS_KEY"
    fi
    
    # Grant permissions
    /garage bucket allow --read --write --owner "$BUCKET_NAME" --key "$ACCESS_KEY"
    echo "   ✅ Permissions granted"
    
    # Write credentials to shared volume
    mkdir -p "$(dirname "$CREDENTIALS_FILE")"
    cat > "$CREDENTIALS_FILE" << EOF
GARAGE_ACCESS_KEY_ID=$ACCESS_KEY
GARAGE_SECRET_ACCESS_KEY=$SECRET_KEY
GARAGE_BUCKET_NAME=$BUCKET_NAME
GARAGE_ENDPOINT=http://maze:3900
EOF
    echo "   ✅ Credentials saved to $CREDENTIALS_FILE"
    
    # Mark setup as done
    touch "$SETUP_MARKER"
    echo "🎉 Setup complete!"
}

# Main execution

# Check if we need to run setup
if [ ! -f "$SETUP_MARKER" ]; then
    echo "📦 First run detected. Starting Garage for setup..."
    
    # Start Garage in background
    /garage "$@" &
    GARAGE_PID=$!
    
    # Wait for it to be ready
    if wait_for_garage; then
        # Run the setup
        run_setup
    else
        echo "❌ Setup failed - Garage didn't become ready"
        kill "$GARAGE_PID" 2>/dev/null
        exit 1
    fi
    
    # Kill the background Garage
    echo "⏹️  Stopping background Garage..."
    kill "$GARAGE_PID"
    wait "$GARAGE_PID" 2>/dev/null || true
    echo "🔄 Restarting Garage in foreground..."
else
    echo "ℹ️  Setup already completed. Starting Garage..."
fi

# Start Garage in foreground
exec /garage "$@"
