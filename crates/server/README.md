![ontrack](../../assets/ontrack.png)

[![Crates.io](https://img.shields.io/crates/v/ontrack.svg)](https://crates.io/crates/ontrack)
[![Documentation](https://docs.rs/ontrack/badge.svg)](https://docs.rs/ontrack)
[![License](https://img.shields.io/crates/l/ontrack.svg)](LICENSE)

Server built using the Ontrack library allowing anyone to integrate Ontrack into there own stack.

> [!NOTE]
> This project is early in development, if you like the idea and want to help improve it, please open an issue.

## Installation

### Docker

#### docker run
Spin up a Ontrack instance quickly with docker run
```bash
docker run ontrack:vincentbrodin@latest
```

#### docker compose
```bash
docker run ontrack:vincentbrodin@latest
```

### Build from source
**Prerequisite**
- cargo

```bash
git clone https://github.com/VincentBrodin/ontrack.git
cd ontrack
cargo build -r -p server
```
