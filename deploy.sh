#!/usr/bin/env bash

set -e

PROJECT_NAME="blaise"
DOCKER_IMAGE="vincbrod/blaise"
VERSION=$(grep '^version =' Cargo.toml | head -1 | cut -d '"' -f 2)

echo "Starting deployment for $PROJECT_NAME v$VERSION..."

echo "📦 Publishing to Crates.io..."
# cargo publish --allow-dirty


echo "🐳 Building Docker image..."
sudo docker build -t $DOCKER_IMAGE:latest -t $DOCKER_IMAGE:$VERSION .

echo "📤 Pushing Docker image..."
sudo docker push $DOCKER_IMAGE:latest
sudo docker push $DOCKER_IMAGE:$VERSION
echo "✅ Deployment complete!"
