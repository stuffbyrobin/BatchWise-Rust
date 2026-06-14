# BatchWise-Rust

A Rust port of [BatchWise](https://github.com/stuffbyrobin/BatchWise) — a
multi-tenant brewery management SaaS — from its original Go modular-monolith
backend to **axum + sqlx + tokio**, preserving the same HTTP/OpenAPI contract,
database schema, and behaviour.

> **Status: in progress.** The port follows the original phase-by-phase build
> order. See [`PORT-PROGRESS.md`](./PORT-PROGRESS.md) for what's done and
> what's next.

## Why a port (and what stays the same)

The conversion keeps the parts that are language-agnostic and re-implements the
Go code:

| Layer | Original (Go) | Port (Rust) |
|---|---|---|
| HTTP router | chi/v5 | axum 0.8 |
| DB driver / pool | pgx/v5 | sqlx (Postgres) |
| Migrations | golang-migrate | `sqlx::migrate!` (same SQL files) |
| JWT | golang-jwt | jsonwebtoken |
| Password hashing | x/crypto argon2 | argon2 |
| Validation | go-playground/validator | validator |
| Logging | slog | tracing |
| Config | caarlos0/env | hand-rolled env loader |
| Physics (`pkg/`) | pure Go | pure Rust (`src/pkg/`) |

The **database schema** (`migrations/`) carries over essentially unchanged — the
golang-migrate `NNNN_name.up.sql`/`.down.sql` convention is also sqlx's
reversible-migration format. The **React frontend** and the **OpenAPI contract**
are unchanged: the Rust backend is a drop-in replacement behind the same API.

## Project layout

```
src/
├── main.rs            # entry point: config, logger, pool, migrations, axum app
├── lib.rs
├── pkg/               # pure brewing physics (ported, fully tested)
│   ├── gravity.rs  color.rs  bitterness.rs  water.rs
│   ├── energy.rs   duty.rs   nutrition.rs
└── platform/          # cross-cutting infra
    ├── config.rs   context.rs  database.rs
    ├── errors.rs   logger.rs   web.rs
migrations/            # schema migrations (shared SQL) + seed/
```

Domain modules (`auth`, `inventory`, `recipe`, `batch`, …) are added under
`src/` phase by phase.

## Local development

```bash
cp .env.example .env          # then set JWT_SECRET (openssl rand -base64 48)
# start a Postgres 16 (e.g. via podman/docker), point DATABASE_URL at it
cargo run                     # applies migrations, serves on :8080
curl localhost:8080/healthz   # {"status":"ok"}
```

## Tests

```bash
cargo test                    # unit tests (pure logic, no DB)
```

Integration tests that need a real Postgres use the `testcontainers` crate
(the Rust equivalent of `testcontainers-go`) and require a Docker/Podman socket.
