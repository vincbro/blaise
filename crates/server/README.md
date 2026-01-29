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
This is were *blaise* will look for and store the GTFS data.

### ALLOCATOR_COUNT
To improve performance *blaise* will pre allocate most of the memory needed for the raptor algorithm to run.


**BE AWARE**, setting this number to high will use large amounts of memory. Start low and see how far you can get.

### LOG_LEVEL
Sets all the the maximum log level that will be displayed.
Can be: `error` `warn` `info` `debug` `trace`


## Endpoints

### /search/area
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

### /search/stop
Perform a fuzzy search for transit stops by name.

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


### /near/area
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


### /near/stop
Find transit stops near a specific geographic coordinate.

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

A `location` can be a coordinate or a area/stop `id`

**Example Request** `GET` `/routing?from=59.330569, 18.059278&to=740021665`
- `from`: **[REQUIRED]** Starting point (Area ID, Stop ID or lat,lng coordinate)..
- `to`: **[REQUIRED]** Destination (Area ID, Stop ID or lat,lng coordinate)..
- `departure_at`: Departure time in hms format `HH:MM:SS` `16:15:37` (Defaults to current system time).
- `arrive_at`: Arrival time in hms format `HH:MM:SS` `16:15:37`.
- `shapes`: Set to `true` if you want the shape for the leg (Defaults to `false`).

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
        {
          "location": {
            "type": "stop",
            "id": "9022050010775001",
            "name": "Hötorget",
            "coordinate": {
              "latitude": 59.335606,
              "longitude": 18.062963
            }
          },
          "departure_time": 25518,
          "arrival_time": 25482,
          "distance_traveled": 12524.78
        },
        {
          "location": {
            "type": "stop",
            "id": "9022050009826001",
            "name": "Rådmansgatan",
            "coordinate": {
              "latitude": 59.340885,
              "longitude": 18.0579
            }
          },
          "departure_time": 25620,
          "arrival_time": 25584,
          "distance_traveled": 13181.62
        },
        {
          "location": {
            "type": "stop",
            "id": "9022050009827001",
            "name": "Odenplan",
            "coordinate": {
              "latitude": 59.34276,
              "longitude": 18.0486
            }
          },
          "departure_time": 25722,
          "arrival_time": 25686,
          "distance_traveled": 13844.33
        },
        {
          "location": {
            "type": "stop",
            "id": "9022050009828001",
            "name": "S:t Eriksplan",
            "coordinate": {
              "latitude": 59.340294,
              "longitude": 18.037416
            }
          },
          "departure_time": 25824,
          "arrival_time": 25788,
          "distance_traveled": 14651.16
        }
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
The shapes object (`shapes` query is set to `true`) includes a list of shapes that define a detailed geographical path the vehicle takes.

Here is an example shape object:
```json
{
  "location": {
    "type": "coordinate",
    "latitude": 59.235462,
    "longitude": 18.101217
  },
  "sequence": 1,
  "distance_traveled": 0.0
},  
```

Important thing to note that the shapes that are returned explain the whole trip, not just the part in your journey. This is to allow you to show the path of each vehicle, however if you only wish to show the part of the trip that is in your journey you can use this logic:

```
  min_distance_traveled < shape.distance_traveled && shape.distance_traveled < max_distance_traveled
```

`min_distance_traveled` will be the distance traveled value of distance traveled in the first stop in a leg, and `max_distance_traveled` will be the distance traveled at the last stop in a leg.


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
