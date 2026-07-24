#!/bin/sh
set -e

BUCKET_NAME="soaricarus-storage"

echo "⏳ Waiting for Garage to be ready..."
# until docker exec maze curl -s -f "http://localhost:3901/v0/status" > /dev/null 2>&1; do
until docker exec maze nc -z "localhost 3900" > /dev/null 2>&1; do
  echo "   Garage not ready yet, retrying in 2s..."
  sleep 2
done
echo "✅ Garage API is responding!"

sleep 3

# Check if layout is already applied
if docker exec maze garage layout show 2>/dev/null | grep -q "Current cluster layout"; then
  echo "ℹ️  Layout already applied, checking bucket..."
else
  echo "📦 Applying cluster layout..."
  
  # Get node ID
  MAX_RETRIES=10
  RETRY_COUNT=0
  NODE_ID=""
  while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    NODE_ID=$(docker exec maze garage node id 2>/dev/null | head -n1)
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
  docker exec maze garage layout assign --version 1 "$NODE_ID"
  docker exec maze garage layout apply --version 1
  echo "   ✅ Layout applied"
  
  sleep 3
fi

# Check if bucket exists
if docker exec maze garage bucket info "${BUCKET_NAME}" 2>/dev/null; then
  echo "ℹ️  Bucket '${BUCKET_NAME}' already exists. Skipping setup."
  exit 0
fi

echo "📦 Creating bucket and configuring permissions..."

# Create the bucket
docker exec maze garage bucket create "${BUCKET_NAME}"
echo "   ✅ Bucket created: ${BUCKET_NAME}"

# Check if key exists
if docker exec maze garage key info "${AWS_ACCESS_KEY_ID}" 2>/dev/null; then
  echo "   ℹ️  Key already exists: ${AWS_ACCESS_KEY_ID}"
else
  docker exec maze garage key create \
    --name "app-key" \
    "${AWS_ACCESS_KEY_ID}" \
    "${AWS_SECRET_ACCESS_KEY}"
  echo "   ✅ Key created: ${AWS_ACCESS_KEY_ID}"
fi

# Grant permissions
docker exec maze garage bucket allow \
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
