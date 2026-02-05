PROJECT_NAME="blaise"
DOCKER_IMAGE="vincbrod/blaise"
VERSION=$(grep '^version =' Cargo.toml | head -1 | cut -d '"' -f 2)

echo "Building $PROJECT_NAME docker image..."
sudo docker build -t $DOCKER_IMAGE:latest -t $DOCKER_IMAGE:$VERSION .
echo "$PROJECT_NAME docker image done"
