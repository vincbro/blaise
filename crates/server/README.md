![blaise](../../assets/blaise.png)

[![Crates.io](https://img.shields.io/crates/v/blaise.svg)](https://crates.io/crates/blaise)
[![Documentation](https://docs.rs/blaise/badge.svg)](https://docs.rs/blaise)
[![License](https://img.shields.io/crates/l/blaise.svg)](LICENSE)

Server built using the *blaise* library allowing anyone to integrate *blaise* into there own stack.

> [!NOTE]
> This project is early in development, if you like the idea and want to help improve it, please open an issue.

## Quick start

### Docker

#### docker run
```bash
docker run --name blaise-server -p 3000:3000 vincentbrodin/blaise:latest
```

#### docker compose
```bash
mkdir blaise-server
cd bliase-server
wget https://raw.githubusercontent.com/VincentBrodin/blaise/refs/heads/main/compose.yaml
docker compose up -d
```

### Build from source
```bash
git clone https://github.com/VincentBrodin/blaise.git
cd blaise
cargo build -r -p server
```

## Endpoints

### /search
Search for transit areas by name.

**Example Request** `GET` `/near?q=S:t Eriksplan`
- `q`: **[REQUIRED]** Search query.
- `count`: Amount of areas to return (Defaults to 5).

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
Search for transit areas by distance from given coordinate.

**Example Request** `GET` `/near?q=59.330569,18.058913`
- `q`: **[REQUIRED]** Coordinate.
- `distance`: Max search distance (Defaults to 500).

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
Find the shortest route from the given `location`

A `location` can be a coordinate or a area `id`

**Example Request** `GET` `/routing?from=59.330569, 18.059278&to=740021665`
- `from`: **[REQUIRED]** Coordinate or id.
- `to`: **[REQUIRED]** Coordinate or id.
- `departure_at`: Time to start the departure in hms format `HH:MM:SS` `16:15:37` (Defaults to current system time).

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
Returns the age of the GTFS data set in seconds since last modified.

**Example Request** `GET` `/gtfs/age`

**Output**
```
10
```


### /gtfs/fetch-url
Allows you to install and replace a GTFS data set with a new without downtime from a url.

**Example Request** `GET` `/gtfs/fetch-url?q=URL_HERE`

