# wol

Small Rust CLI for Wake-on-LAN with an optional HTTPS control plane.

## Features

- Save devices in a config file under `~/.config/wol/config.toml`
- Send WOL packets directly from the CLI
- Start an HTTPS API with bearer-token authentication
- Run as a Docker Compose service

## Usage

Initialize the server configuration:

```bash
cargo run -- init \
  --bind 0.0.0.0:8443 \
  --cert /path/to/cert.pem \
  --key /path/to/key.pem \
  --token super-secret-token
```

Add a device:

```bash
cargo run -- device add desktop \
  --mac AA:BB:CC:DD:EE:FF \
  --host 192.168.1.255 \
  --port 9
```

List devices:

```bash
cargo run -- device list
```

Wake a device directly:

```bash
cargo run -- wake desktop
```

Start the HTTPS server:

```bash
cargo run -- serve
```

List devices remotely:

```bash
curl -k -X POST \
  -H "Authorization: Bearer super-secret-token" \
  https://localhost:8443/devices
```

Trigger a wake request remotely:

```bash
curl -k -X POST \
  -H "Authorization: Bearer super-secret-token" \
  https://localhost:8443/wake/desktop
```

Check server health:

```bash
curl -k -X POST \
  -H "Authorization: Bearer super-secret-token" \
  https://localhost:8443/healthz
```

## Docker Compose

Build the image:

```bash
docker compose build
```

Initialize the container-managed config:

```bash
mkdir -p data certs
docker compose run --rm wol init \
  --bind 0.0.0.0:8443 \
  --cert /certs/fullchain.pem \
  --key /certs/privkey.pem \
  --token super-secret-token
```

Add a device:

```bash
docker compose run --rm wol device add desktop \
  --mac AA:BB:CC:DD:EE:FF \
  --host 192.168.1.255 \
  --port 9
```

List configured devices:

```bash
docker compose run --rm wol device list
```

Start the API server:

```bash
docker compose up -d
```

Stop it:

```bash
docker compose down
```

## Notes

- Every HTTP endpoint requires bearer authentication.
- HTTP errors return JSON responses for `401`, `404`, and `405`.
- You need a valid PEM certificate and key for HTTPS.
- The compose file mounts `./data` to `/root/.config/wol` so config persists across runs.
- The compose file mounts `./certs` to `/certs` read-only; place your PEM files there or change the paths.
- WOL broadcast from a container can be limited by Docker networking. On Linux, `network_mode: host` is usually the most reliable option. On Docker Desktop for macOS, LAN broadcast behavior may not match native host networking.
