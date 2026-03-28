# ALICE-LB-SaaS

L4/L7 load balancer control-plane API — part of the ALICE Eco-System.

## Overview

ALICE-LB-SaaS provides a programmatic control plane for L4/L7 load balancing. Register backend pools, configure routing rules, drain nodes for maintenance, and retrieve real-time health and statistics.

## Services

- **core-engine** — Backend registry, routing engine, health checks (port 8126)
- **api-gateway** — JWT auth, rate limiting, reverse proxy

## Quick Start

```bash
cd services/core-engine
cargo run

curl http://localhost:8126/api/v1/lb/health
```

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | /api/v1/lb/backends | Register backend pool |
| POST | /api/v1/lb/route | Configure routing rule |
| GET  | /api/v1/lb/health | Backend health status |
| POST | /api/v1/lb/drain | Drain a backend node |
| GET  | /api/v1/lb/stats | Service statistics |

## License

AGPL-3.0-or-later
