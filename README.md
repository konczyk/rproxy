# rproxy

A minimal async HTTP reverse proxy using Tokio.

## Overview

`rproxy` listens for HTTP connections, matches requests by `Host` header and path prefix, and forwards traffic to a configured upstream using
bidirectional TCP streaming.

## Features

- Async, non-blocking I/O (Tokio)
- Static routing by `Host` + path prefix
- HTTP/1.1 request parsing
- Zero-copy bidirectional tunneling
- Optional debug logging (`--debug`)

## Running

```bash
cargo run
```

Options:
```shell
cargo run -- -h
```

