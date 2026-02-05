![blaise](../../assets/blaise.png)

[![Crates.io](https://img.shields.io/crates/v/blaise.svg)](https://crates.io/crates/blaise)
[![Documentation](https://docs.rs/blaise/badge.svg)](https://docs.rs/blaise)
[![License](https://img.shields.io/crates/l/blaise.svg)](LICENSE)

The blaise-server is a ready-to-use HTTP wrapper for the *blaise* transit engine library.
It allows you to integrate high-performance, local-first transit routing and searching into any stack without managing a complex Rust integration.

> [!NOTE]
> This project is early in development, if you like the idea and want to help improve it, please open an issue.

## Quick start

### Docker
The fastest way to get an instance running.
```bash
docker run --name blaise-server -p 3000:3000 vincbrod/blaise:latest
```

### Docker compose
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
      - ALLOCATOR_COUNT=32
      - LOG_LEVEL=info
    volumes:
      - ./gtfs_data:/app/data
    restart: unless-stopped
```

**OR**

```bash
mkdir blaise-server
cd bliase-server
wget https://raw.githubusercontent.com/vincbro/blaise/refs/heads/main/compose.yaml
docker compose up -d
```

### Build from source

**Prerequisite**: Rust/Cargo installed.

```bash
git clone https://github.com/vincbro/blaise.git
cd blaise
cargo build -r -p server
```

## Enviroment variables
### GTFS_DATA_PATH
This is where *blaise* will look for and store the GTFS data.

### ALLOCATOR_COUNT
To improve performance *blaise* will pre allocate most of the memory needed for the raptor algorithm to run.


**BE AWARE**, setting this number too high will use large amounts of memory. Start low and see how far you can get.

### LOG_LEVEL
Sets the maximum log level that will be displayed.
Can be: `error` `warn` `info` `debug` `trace`


## Endpoints

### /search/area
Perform a fuzzy search for transit areas by name.

**Example Request** `GET` `/search/area?q=S:t Eriksplan&count=5`

**Parameters:**
- `q`: **[REQUIRED]** The search query (e.g., "S:t Eriksplan")
- `count`: Max results to return (Defaults to 5)

**Output**
```json
[
  {
    "id": "740021665",
    "name": "S:t Eriksplan T-bana",
    "coordinate": {
      "latitude": 59.34002,
      "longitude": 18.03799
    }
  },
  ... shortened for readability
]
```

### /search/stop
Perform a fuzzy search for transit stops by name.

**Example Request** `GET` `/search/stop?q=S:t Eriksplan&count=5`

**Parameters:**
- `q`: **[REQUIRED]** The search query (e.g., "S:t Eriksplan")
- `count`: Max results to return (Defaults to 5)

**Output**
```json
[
  {
    "id": "740021665",
    "name": "S:t Eriksplan T-bana",
    "coordinate": {
      "latitude": 59.34002,
      "longitude": 18.03799
    }
  },
  ... shortened for readability
]
```


### /near/area
Find transit areas near a specific geographic coordinate.

**Example Request** `GET` `/near/area?q=59.330569,18.058913&distance=500`

**Parameters:**
- `q`: **[REQUIRED]** Coordinate string in lat,lng format
- `distance`: Max search radius in meters (Defaults to 500)

**Output**
```json
[
  {
    "id": "740021665",
    "name": "S:t Eriksplan T-bana",
    "coordinate": {
      "latitude": 59.34002,
      "longitude": 18.03799
    }
  },
  ... shortened for readability
]
```


### /near/stop
Find transit stops near a specific geographic coordinate.

**Example Request** `GET` `/near/stop?q=59.330569,18.058913&distance=500`

**Parameters:**
- `q`: **[REQUIRED]** Coordinate string in lat,lng format
- `distance`: Max search radius in meters (Defaults to 500)

**Output**
```json
[
  {
    "id": "740021665",
    "name": "S:t Eriksplan T-bana",
    "coordinate": {
      "latitude": 59.34002,
      "longitude": 18.03799
    }
  },
  ... shortened for readability
]
```



### /routing
Calculate the optimal path between two points using the RAPTOR algorithm.

A `location` can be a coordinate or a area/stop `id`

**Example Request** `GET` `/routing?from=59.330569,18.059278&to=740021665&departure_at=16:15:37&shapes=true&allow_walks=true`

**Parameters:**
- `from`: **[REQUIRED]** Starting point (Area ID, Stop ID or lat,lng coordinate)
- `to`: **[REQUIRED]** Destination (Area ID, Stop ID or lat,lng coordinate)
- `departure_at`: Departure time in hms format `HH:MM:SS` (Defaults to current system time)
- `arrive_at`: Arrival time in hms format `HH:MM:SS`
- `shapes`: Set to `true` if you want the shape for the leg (Defaults to `false`)
- `allow_walk`: Set to `false` if you want to ignore possible walkable routes (Defaults to `true`)

**Output**
```json
{
  "from": {
    "type": "coordinate",
    "latitude": 59.33057,
    "longitude": 18.059278
  },
  "to": {
    "type": "area",
    "id": "740021665",
    "name": "S:t Eriksplan T-bana",
    "coordinate": {
      "latitude": 59.339966,
      "longitude": 18.03757
    }
  },
  "legs": [
    {
      "from": {
        "type": "stop",
        "id": "9022050009825003",
        "name": "T-Centralen",
        "coordinate": {
          "latitude": 59.331524,
          "longitude": 18.06124
        }
      },
      "to": {
        "type": "stop",
        "id": "9022050009828001",
        "name": "S:t Eriksplan",
        "coordinate": {
          "latitude": 59.340294,
          "longitude": 18.037416
        }
      },
      "departue_time": 25428,
      "arrival_time": 25788,
      "stops": [
        {
          "location": {
            "type": "stop",
            "id": "9022050009825003",
            "name": "T-Centralen",
            "coordinate": {
              "latitude": 59.331524,
              "longitude": 18.06124
            }
          },
          "departure_time": 25428,
          "arrival_time": 25386,
          "distance_traveled": 11963.58
        },
        ... shortened for readability
      ],
      "mode": "Subway",
      "head_sign": null,
      "long_name": "Gröna linjen",
      "short_name": "18",
      "shapes": null
    }
  ]
}
```

**Shapes**

When you set `shapes=true` in your query, you'll receive a detailed geographical path showing the complete route the vehicle travels.

Each shape point contains:

```json
{
  "location": {
    "type": "coordinate",
    "latitude": 59.235462,
    "longitude": 18.101217
  },
  "sequence": 1,
  "distance_traveled": 0.0
}
```

**Key Fields:**
- `location`: GPS coordinates of the point
- `sequence`: Order of points along the route (starts at 1)
- `distance_traveled`: Cumulative distance from the trip start

**Important:** Shape data covers the entire vehicle trip, not just your specific journey segment. To filter shapes for only your journey portion:

```
min_distance_traveled < shape.distance_traveled && shape.distance_traveled < max_distance_traveled
```

Where:
- `min_distance_traveled`: Distance at your journey's first stop
- `max_distance_traveled`: Distance at your journey's last stop


### /gtfs/age

Returns the age of the current GTFS dataset in seconds since it was last modified.

**Example Request** `GET` `/gtfs/age`

**Output**
```
10
```


### /gtfs/fetch-url

Installs or replaces the active GTFS dataset from a remote URL without needing to restart the server.

**Example Request** `GET` `/gtfs/fetch-url?q=https://example.com/gtfs-data.zip`

**Parameters:**
- `q`: **[REQUIRED]** HTTPS URL to a ZIP file containing GTFS data

## License

This project is licensed under the MIT License.
