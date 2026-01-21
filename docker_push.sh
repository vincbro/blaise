PROJECT_NAME="blaise"
DOCKER_IMAGE="vincbrod/blaise"
VERSION=$(grep '^version =' Cargo.toml | head -1 | cut -d '"' -f 2)

echo "Pushing $PROJECT_NAME docker image..."
sudo docker push $DOCKER_IMAGE:latest
sudo docker push $DOCKER_IMAGE:$VERSION
echo "$PROJECT_NAME docker image pushed"
