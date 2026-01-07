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
docker run --name blaise-server -p 3000:3000 vincentbrodin/blaise:latest
```

### Docker compose
Recommended for persistent data management. Create a `compose.yaml` and run `docker compose up -d`.

```yaml
services:
  blaise-server:
    image: vincentbrodin/blaise:latest
    container_name: blaise-server
    ports:
      - "3000:3000"
    environment:
      - GTFS_DATA_PATH=/app/data/GTFS.zip
    volumes:
      - ./gtfs_data:/app/data
    restart: unless-stopped
```

**OR**

```bash
mkdir blaise-server
cd bliase-server
wget https://raw.githubusercontent.com/VincentBrodin/blaise/refs/heads/main/compose.yaml
docker compose up -d
```

### Build from source

**Prerequisite**: Rust/Cargo installed.

```bash
git clone https://github.com/VincentBrodin/blaise.git
cd blaise
cargo build -r -p server
```

## Endpoints

### /search
Perform a fuzzy search for transit areas by name.

**Example Request** `GET` `/search?q=S:t Eriksplan`
- `q`: **[REQUIRED]** The search query (e.g., "S:t Eriksplan")..
- `count`: Max results to return (Defaults to 5)..

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


### /near
Find transit areas near a specific geographic coordinate.

**Example Request** `GET` `/near?q=59.330569,18.058913`
- `q`: **[REQUIRED]** Coordinate string in lat,lng format..
- `distance`: Max search radius in meters (Defaults to 500)..

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

A `location` can be a coordinate or a area `id`

**Example Request** `GET` `/routing?from=59.330569, 18.059278&to=740021665`
- `from`: **[REQUIRED]** Starting point (Area ID or lat,lng coordinate)..
- `to`: **[REQUIRED]** Destination (Area ID or lat,lng coordinate)..
- `departure_at`: Departure time in hms format `HH:MM:SS` `16:15:37` (Defaults to current system time).

**Output**
```json
{
  "from": {
    "Coordinate": {
      "latitude": 59.33057,
      "longitude": 18.059278
    }
  },
  "to": {
    "Area": {
      "id": "740021665",
      "name": "S:t Eriksplan T-bana",
      "coordinate": {
        "latitude": 59.34002,
        "longitude": 18.03799
      }
    }
  },
  "legs": [
    {
      "from": {
        "Coordinate": {
          "latitude": 59.33057,
          "longitude": 18.059278
        }
      },
      "to": {
        "Stop": {
          "id": "9022050009825003",
          "name": "T-Centralen",
          "coordinate": {
            "latitude": 59.331524,
            "longitude": 18.06124
          }
        }
      },
      "departue_time": 57600,
      "arrival_time": 57734,
      "stops": [],
      "leg_type": "Walk"
    },
    {
      "from": {
        "Stop": {
          "id": "9022050009825003",
          "name": "T-Centralen",
          "coordinate": {
            "latitude": 59.331524,
            "longitude": 18.06124
          }
        }
      },
      "to": {
        "Stop": {
          "id": "9022050009828001",
          "name": "S:t Eriksplan",
          "coordinate": {
            "latitude": 59.340294,
            "longitude": 18.037416
          }
        }
      },
      "departue_time": 57792,
      "arrival_time": 58152,
      "stops": [
        {
          "location": {
            "Stop": {
              "id": "9022050009825003",
              "name": "T-Centralen",
              "coordinate": {
                "latitude": 59.331524,
                "longitude": 18.06124
              }
            }
          },
          "departure_time": 57792,
          "arrival_time": 57750
        },
        ... shortened for readability
      ],
      "leg_type": "Transit"
    },
    {
      "from": {
        "Stop": {
          "id": "9022050009828001",
          "name": "S:t Eriksplan",
          "coordinate": {
            "latitude": 59.340294,
            "longitude": 18.037416
          }
        }
      },
      "to": {
        "Coordinate": {
          "latitude": 59.34002,
          "longitude": 18.03799
        }
      },
      "departue_time": 58152,
      "arrival_time": 58191,
      "stops": [],
      "leg_type": "Walk"
    }
  ]
}
```



### /gtfs/age

Returns the age of the current GTFS dataset in seconds since it was last modified.

**Example Request** `GET` `/gtfs/age`

**Output**
```
10
```


### /gtfs/fetch-url

Installs or replaces the active GTFS dataset from a remote URL without needing to restart the server.

**Example Request** `GET` `/gtfs/fetch-url?q={HTTPS_URL_TO_ZIP}`

## License

This project is licensed under the MIT License.
