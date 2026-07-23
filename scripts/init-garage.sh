#!/bin/bash
set -e

# Use the credentials from your .env
BUCKET_NAME="soaricarus-storage"

echo "⏳ Waiting for Garage to be ready..."
# until curl -s -f "http://localhost:3901/v0/status" > /dev/null 2>&1; do
  # echo "   Garage not ready yet, retrying in 2s..."
  # sleep 2
# done
sleep 20
echo "✅ Garage API is responding!"
echo "... I think..."

# Wait a bit more for Garage to fully initialize
sleep 5

# Check if layout is already applied
if garage layout show 2>/dev/null | grep -q "Current cluster layout"; then
  echo "ℹ️  Layout already applied, checking bucket..."
else
  echo "📦 Applying cluster layout..."
  
  # Get node ID - this might need multiple attempts
  MAX_RETRIES=10
  RETRY_COUNT=0
  while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    NODE_ID=$(garage node id 2>/dev/null | head -n1)
    if [ -n "$NODE_ID" ]; then
      break
    fi
    RETRY_COUNT=$((RETRY_COUNT + 1))
    echo "   Waiting for node ID (attempt $RETRY_COUNT/$MAX_RETRIES)..."
    sleep 2
  done
  
  if [ -z "$NODE_ID" ]; then
    echo "❌ Failed to get node ID after $MAX_RETRIES attempts"
    exit 1
  fi
  
  echo "   Node ID: $NODE_ID"
  garage layout assign --version 1 "$NODE_ID"
  garage layout apply --version 1
  echo "   ✅ Layout applied"
  
  # Wait for layout to stabilize
  sleep 3
fi

# Check if bucket already exists
if garage bucket info "${BUCKET_NAME}" 2>/dev/null; then
  echo "ℹ️  Bucket '${BUCKET_NAME}' already exists. Skipping setup."
  exit 0
fi

echo "📦 Creating bucket and configuring permissions..."

# 1. Create the bucket
garage bucket create "${BUCKET_NAME}"
echo "   ✅ Bucket created: ${BUCKET_NAME}"

# 2. Check if key exists
if garage key info "${AWS_ACCESS_KEY_ID}" 2>/dev/null; then
  echo "   ℹ️  Key already exists: ${AWS_ACCESS_KEY_ID}"
else
  garage key create \
    --name "app-key" \
    "${AWS_ACCESS_KEY_ID}" \
    "${AWS_SECRET_ACCESS_KEY}"
  echo "   ✅ Key created: ${AWS_ACCESS_KEY_ID}"
fi

# 3. Grant full permissions
garage bucket allow \
  --read \
  --write \
  --owner \
  "${BUCKET_NAME}" \
  --key "${AWS_ACCESS_KEY_ID}"
echo "   ✅ Permissions granted"

echo "🎉 Setup complete!"
echo "   Bucket: ${BUCKET_NAME}"
echo "   Access Key: ${AWS_ACCESS_KEY_ID}"
echo "   Endpoint: http://maze:3900"
