# rproxy

A minimal async HTTP reverse proxy using Tokio.

## Overview

`rproxy` listens for HTTP connections, matches requests by `Host` header and path prefix, and forwards traffic to a configured upstream using
bidirectional TCP streaming.  

In addition to routing, `rproxy` can optionally enforce access control and authorization, allowing the proxy to operate as a private reverse proxy when required.
## Features

- Async, non-blocking I/O (Tokio)
- Dynamic routing via configuration file (default: `config.toml`)
- Supports **TOML** and **YAML** (`.yaml` / `.yml`) configs - examples in `data` directory
- Routing by `Host` header + path prefix
- Per-route upstream timeouts
- Protection against stalled or unresponsive backends
- Graceful shutdown with active connection tracking
- Access control and authorization
  - IP whitelist
  - HTTP Basic authentication
  - API key authentication
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
cargo run -r -- -d -c data/config.toml
    Finished `release` profile [optimized] target(s) in 0.03s
     Running `target/release/rproxy -d -c data/config.toml`
2026-01-15T23:13:47.134917Z  INFO proxy-request{client=[::1]:47754 request_id="9a7ea93a-736a-4fee-832e-8798c528e022"}: rproxy::connection: Processing new request
2026-01-15T23:13:47.134933Z DEBUG proxy-request{client=[::1]:47754 request_id="9a7ea93a-736a-4fee-832e-8798c528e022"}: rproxy::http: Parsing request headers
2026-01-15T23:13:47.135005Z DEBUG proxy-request{client=[::1]:47754 request_id="9a7ea93a-736a-4fee-832e-8798c528e022"}: rproxy::connection: No authorization method used
2026-01-15T23:14:30.407349Z  INFO proxy-request{client=[::1]:33838 request_id="93cb474f-6000-4cc6-aee1-b76937deabe9"}: rproxy::connection: Processing new request
2026-01-15T23:14:30.407365Z DEBUG proxy-request{client=[::1]:33838 request_id="93cb474f-6000-4cc6-aee1-b76937deabe9"}: rproxy::http: Parsing request headers
2026-01-15T23:14:30.407414Z DEBUG proxy-request{client=[::1]:33838 request_id="93cb474f-6000-4cc6-aee1-b76937deabe9"}: rproxy::connection: Invalid API Key
2026-01-15T23:14:48.390522Z  INFO proxy-request{client=[::1]:41644 request_id="66dee5f3-b429-4eaf-8dd4-b10316155ba6"}: rproxy::connection: Processing new request
2026-01-15T23:14:48.390540Z DEBUG proxy-request{client=[::1]:41644 request_id="66dee5f3-b429-4eaf-8dd4-b10316155ba6"}: rproxy::http: Parsing request headers
2026-01-15T23:15:18.518185Z  INFO proxy-request{client=[::1]:33330 request_id="81227206-dc38-4666-a687-bc7ab64ce3b7"}: rproxy::connection: Processing new request
2026-01-15T23:15:18.518202Z DEBUG proxy-request{client=[::1]:33330 request_id="81227206-dc38-4666-a687-bc7ab64ce3b7"}: rproxy::http: Parsing request headers
2026-01-15T23:15:18.518242Z DEBUG proxy-request{client=[::1]:33330 request_id="81227206-dc38-4666-a687-bc7ab64ce3b7"}: rproxy::connection: Failed to select upstream server
^C2026-01-15T23:16:04.107351Z  WARN rproxy: Shutdown signal received...
2026-01-15T23:16:04.107369Z  INFO rproxy: Waiting for active connections to finish...
2026-01-15T23:16:04.107376Z  INFO rproxy: Shutdown complete!
```

