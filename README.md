# sentry-tunnel-standalone

Request proxy for outgoing Sentry payloads from the JS SDK so ad blockers
and tracking protection don't block them.

## Config (env vars)

- `TUNNEL_BIND` (default: `0.0.0.0:8008`)
- `TUNNEL_PATH` (default: `/tunnel`)
- `TUNNEL_ORIGINS` (must be set, comma-separated list)
- `SENTRY_HOST` (must be set, take it from the DSN)
- `SENTRY_PROJECT_IDS` (must be set, comma-separated list)
