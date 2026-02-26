# Custom Relay Setup

This version of `iroh-ssh` is built on iroh version `0.94`. If you want to use a custom relay server as seen in the examples below, you need to use the matching `iroh-relay` of version `0.94`. See [setup relay server](#setup-relay-server) for more information on how to set up a relay server.

## Usage --relay-url

To connect an `iroh-ssh` client to a server using the `--relay-url <URL>` option, the `iroh-ssh server` must also use the same `--relay-url <URL>` to guarantee connection.

```bash
> iroh-ssh server --relay-url <URL>
```

```bash
> iroh-ssh my-user@110017f0d23788158e4d32c0e213ec38b95cf4e9a0a8cbcb10d6f9c578dd7863 --relay-url <URL>
``` 

## Setup Relay Server

The easiest way is to use `n0-computers` own `iroh-relay` Docker image:

```bash
# Creates a relay server bound to 'http://localhost:3340'
> docker run n0computer/iroh-relay:v0.94.0 --dev
```

The best resource for setting up your own relay server beyond the `--dev` testing mode is the [iroh-relay README](https://github.com/n0-computer/iroh/tree/main/iroh-relay). To run securely with TLS you can pass a config `--config-path <PATH>` option as follows:

```bash
> docker run -d -p 80:80 -p 443:443 -v /path/with/config/:/app n0computer/iroh-relay:v0.94.0 --config-path /app/relay-config.toml
```

and to get you started, here my example `relay-config.toml`:

```toml
# Enable the relay server (default: true)
enable_relay = true

# Bind to standard HTTP port for Let's Encrypt challenges
http_bind_addr = "[::]:80"

# Enable metrics (default: true)
enable_metrics = true
metrics_bind_addr = "[::]:9090"

# TLS Configuration (required for production)
[tls]
# Your domain name
hostname = "iroh-relay.rustonbsd.com"

# Your contact email for Let's Encrypt
contact = "yourletsencrypt@email.com"

# Use Let's Encrypt for automatic certificates
cert_mode = "LetsEncrypt"

# Use production Let's Encrypt server (default: true)
prod_tls = true

# Directory to cache certificates
cert_dir = "/app/certs/"

# HTTPS will bind to port 443 by default
https_bind_addr = "[::]:443"  # Optional: explicitly set

# QUIC will bind to port 7842 by default
quic_bind_addr = "[::]:7842"  # Optional: explicitly set

# Optional: Enable QUIC address discovery
enable_quic_addr_discovery = true

# Optional:  Rate limiting to prevent abuse
[limits]
accept_conn_limit = 100    # connections per second
accept_conn_burst = 200    # burst capacity

[limits.client. rx]
bytes_per_second = 10485760  # 10 MB/s per client
max_burst_bytes = 20971520   # 20 MB burst

# Optional: Access control (default: everyone)
# access = "everyone"  # Allow all endpoints
# Or use an allowlist:
# access.allowlist = ["<endpoint-id-1>", "<endpoint-id-2>"]
# Or use HTTP-based access control:
# access. http.url = "https://auth.yourdomain.com/check-relay-access"
# access.http.bearer_token = "your-secret-token"
```

This automagically creates letsencrypt certificates `cert_mode = "LetsEncrypt"` and stores them at `cert_dir = "/app/certs/"` no need for manual certificate management.