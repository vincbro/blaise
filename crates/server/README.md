![blaise](../../assets/blaise.png)

[![Crates.io](https://img.shields.io/crates/v/blaise.svg)](https://crates.io/crates/blaise)
[![Documentation](https://docs.rs/blaise/badge.svg)](https://docs.rs/blaise)
[![License](https://img.shields.io/crates/l/blaise.svg)](LICENSE)

Server built using the *blaise* library allowing anyone to integrate *blaise* into there own stack.

> [!NOTE]
> This project is early in development, if you like the idea and want to help improve it, please open an issue.

## Installation

### Docker

#### docker run
Spin up a *blaise* instance quickly with docker run
```bash
docker run blaise:vincentbrodin@latest
```

#### docker compose
```bash
docker run blaise:vincentbrodin@latest
```

### Build from source
**Prerequisite**
- cargo

```bash
git clone https://github.com/VincentBrodin/blaise.git
cd blaise
cargo build -r -p server
```
