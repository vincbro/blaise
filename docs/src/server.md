# Using the Server

The **blaise-server** is a ready-to-use HTTP wrapper for the *blaise* transit engine library. It allows you to integrate high-performance, local-first transit routing and searching into any stack without managing a complex Rust integration.

## Quick Start

### Docker

The fastest way to get an instance running is using the official Docker image:

```bash
docker run --name blaise-server -p 3000:3000 vincbrod/blaise:latest
```

### Docker Compose

Recommended for persistent data management. Create a `compose.yaml` and run `docker compose up -d`.

```yaml
services:
  blaise-server:
    image: vincbrod/blaise:latest
    container_name: blaise-server
    ports:
      - "3000:3000"
    environment:
      - GTFS_DATA_PATH=/app/data/GTFS.zip
    volumes:
      - ./gtfs_data:/app/data
    restart: unless-stopped

```

Alternatively, you can fetch the standard configuration directly:

```bash
mkdir blaise-server
cd blaise-server
wget https://raw.githubusercontent.com/vincbro/blaise/refs/heads/main/compose.yaml
docker compose up -d
```

### Build from Source

**Prerequisite**: Rust/Cargo installed.

```bash
git clone https://github.com/vincbro/blaise.git
cd blaise
cargo build -r -p server
```

## Endpoints

### `/search`

Perform a fuzzy search for transit areas by name.

**Example Request**: `GET /search?q=S:t Eriksplan`

- **q**: **[REQUIRED]** The search query (e.g., "S:t Eriksplan").
- **count**: Max results to return (Defaults to 5).

### `/near`

Find transit areas near a specific geographic coordinate.

**Example Request**: `GET /near?q=59.330569,18.058913`

- **q**: **[REQUIRED]** Coordinate string in `lat,lng` format.
- **distance**: Max search radius in meters (Defaults to 500).

### `/routing`

Calculate the optimal path between two points using the RAPTOR algorithm. A location can be a coordinate or an area ID.

**Example Request**: `GET /routing?from=59.330569,18.059278&to=740021665`

- **from**: **[REQUIRED]** Starting point (Area ID or `lat,lng` coordinate).
- **to**: **[REQUIRED]** Destination (Area ID or `lat,lng` coordinate).
- **departure_at**: Departure time in HMS format `HH:MM:SS` (Defaults to current system time).

### `/gtfs/age`

Returns the age of the current GTFS dataset in seconds since it was last modified.

### `/gtfs/fetch-url`

Installs or replaces the active GTFS dataset from a remote URL without needing to restart the server.

**Example Request**: `GET /gtfs/fetch-url?q={HTTPS_URL_TO_ZIP}`
