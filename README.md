# rproxy

A minimal async HTTP reverse proxy using Tokio.

## Overview

`rproxy` listens for HTTP connections, matches requests by `Host` header and path prefix, and forwards traffic to a configured upstream using
bidirectional TCP streaming.

## Features

- Async, non-blocking I/O (Tokio)
- Dynamic routing via configuration file (default: `config.toml`)
- Supports **TOML** and **YAML** (`.yaml` / `.yml`) configs - examples in `data` directory
- Routing by `Host` header + path prefix
- Per-route upstream timeouts
- Protection against stalled or unresponsive backends
- Graceful shutdown with active connection tracking
- HTTP/1.1 request header parsing
- Zero-copy bidirectional tunneling
- Optional debug logging (`--debug`)
 
## Testing 

```bash
cargo test 
```

## Running

```bash
cargo run
```

Options:
```shell
cargo run -- -h
```

## Examples

Running with a sample TOML configuration
```shell
cargo run -- -c data/config.yaml
```

