# Running BatchWise on Supabase

Supabase is managed Postgres. BatchWise talks to it through `sqlx` exactly as it
does to any Postgres, so moving over is a **configuration change, not a code
rewrite**: point `DATABASE_URL` at your Supabase project, let the embedded
migrations run, done. No ORM swap, no query changes.

## 1. Create the project

1. Sign up at <https://supabase.com> and create a project.
2. Choose a **region** close to where the backend runs (latency is per-query).
3. Set a strong **database password** and store it in your secrets manager — it
   goes into `DATABASE_URL`, never into the repo.

## 2. Get the connection string

In the dashboard: **Project Settings → Database → Connection string**. You'll see
several options. Pick based on how you run the app:

| Option | Host / port | Use it for |
| --- | --- | --- |
| **Direct connection** | `db.<ref>.supabase.co:5432` | Migrations and a single long-lived backend at moderate concurrency. Supports prepared statements. **Recommended default.** |
| **Session pooler** | `…pooler.supabase.com:5432` | Drop-in replacement for direct when you need more connections; still supports prepared statements. |
| **Transaction pooler** | `…pooler.supabase.com:6543` | High concurrency / serverless. Does **not** support cached prepared statements — see note below. |

Always keep `?sslmode=require` on the URL — Supabase enforces TLS. The crate is
already built with `tls-rustls`, so no dependency changes are needed.

```bash
# Direct connection (recommended)
DATABASE_URL=postgresql://postgres:YOUR_PASSWORD@db.YOUR_PROJECT_REF.supabase.co:5432/postgres?sslmode=require
```

### IPv4-only networks (use a pooler)

The **direct connection** host (`db.<ref>.supabase.co`) resolves to an
**IPv6-only** address unless you have bought Supabase's IPv4 add-on. On an
IPv4-only network (most home/office Wi-Fi), connecting to it fails with
`Network is unreachable (os error 101)`.

Use a **pooler** endpoint (`*.pooler.supabase.com`) instead, which is reachable
over IPv4. The **Session pooler** (port 5432) is the recommended drop-in for a
single backend plus migrations: it still supports prepared statements, so `sqlx`
works unchanged. The pooler URL differs from the direct URL in three ways:

- **Host** becomes `aws-<N>-<REGION>.pooler.supabase.com`
- **Username** becomes `postgres.YOUR_PROJECT_REF` (the project ref is appended)
- **Port** stays `5432` for the session pooler

```bash
# Session pooler (IPv4-friendly), recommended on IPv4-only networks
DATABASE_URL=postgresql://postgres.YOUR_PROJECT_REF:YOUR_PASSWORD@aws-1-YOUR_REGION.pooler.supabase.com:5432/postgres?sslmode=require
```

The host prefix may be **`aws-0-`** or **`aws-1-`** (newer projects); both
resolve in DNS, so you cannot guess it. **Copy the exact host** from the
dashboard (**Session pooler** tab). A wrong region or prefix still reaches
Supavisor but fails with `(ENOTFOUND) tenant/user postgres.YOUR_PROJECT_REF not found`.

### Transaction pooler note

`sqlx` caches server-side prepared statements by default, which fails on
Supabase's transaction pooler (port 6543). `src/platform/database.rs` detects
port `6543` and disables the statement cache automatically, so the transaction
pooler works without any further config. The direct connection and session
pooler are unaffected.

## 3. Configuring secrets (`DATABASE_URL`, `JWT_SECRET`)

The app reads its config from environment variables (`Config::load()` in
`src/platform/config.rs`). The database password lives inside `DATABASE_URL`, so
treat the whole URL as a secret. **Never commit the real password** — `.env` is
git-ignored and `.env.example` only carries placeholders.

**Local development.** Put the real values in a local `.env` file (or export them
in your shell):

```bash
# .env (git-ignored)
DATABASE_URL=postgresql://postgres:REAL_PASSWORD@db.YOUR_PROJECT_REF.supabase.co:5432/postgres?sslmode=require
JWT_SECRET=...     # openssl rand -base64 48
```

**Claude Code on the web (cloud sessions).** There is no encrypted secrets store
yet — environment variables are kept in the *environment configuration* and are
**visible in plaintext to anyone who can edit that environment**. To add them:
open the environment selector (cloud icon, top of the session), click the gear
icon on your environment, and add variables in `.env` format (one `KEY=value`
per line, **no quotes**). Use this only with a **dev/throwaway** Supabase project.

**Production.** Store `DATABASE_URL` and `JWT_SECRET` in your deploy platform's
secrets manager (Fly/Render/Railway/AWS/etc.), not in the web environment config
or the repo. If a credential ever leaks, **rotate the database password** from
Supabase's **Project Settings → Database** and update the secret everywhere.

## 4. Run migrations

Migrations are embedded (`sqlx::migrate!("./migrations")`) and run on startup
unless `MIGRATIONS_DISABLED=true`. Point the app at the **direct connection** the
first time so all 26 migrations apply in order:

```bash
export DATABASE_URL='postgresql://postgres:YOUR_PASSWORD@db.YOUR_PROJECT_REF.supabase.co:5432/postgres?sslmode=require'
cargo run                 # connects, applies migrations, starts the server
cargo run -- --seed       # optional: load reference data (styles, yeasts, water profiles…)
```

No Supabase CLI or SQL-editor steps are required — the app owns its schema.

## 5. What we deliberately do *not* use

Supabase bundles Auth (GoTrue), PostgREST, Storage, and Realtime. BatchWise has
its own JWT auth, tenant scoping, and Axum API, so we treat Supabase as **just
managed Postgres** and ignore those layers.

## 6. Row Level Security (RLS) — deferred, on purpose

RLS enforces per-row access inside Postgres. Its big payoff is when *untrusted
clients query the database directly* (Supabase's default PostgREST/`supabase-js`
model). BatchWise doesn't do that: only the trusted backend connects, and every
repository query is already tenant-scoped (`WHERE tenant_id = $1`). So RLS would
be a second fence behind a working one, and turning it on usefully would require
connecting as a non-owner role and setting `app.current_tenant` per request.

**Decision: leave RLS off for now.** Revisit it as defense-in-depth if any of
these become true:

- You expose the DB directly to frontends via Supabase Auth/PostgREST.
- You want a backstop against a bug in the query layer leaking cross-tenant rows.
- Third-party tools / BI / read replicas query the DB outside the app.

Supabase will warn that `public` tables have RLS disabled. That warning targets
PostgREST exposure, which we don't use. The safe way to satisfy it without
adopting RLS is to ensure the `anon`/PostgREST role has **no privileges** on
these tables, rather than enabling policies we don't rely on.

## CI / tests are unchanged

Integration tests spin up a throwaway `postgres:16-alpine` via `testcontainers`
(or `TEST_DATABASE_URL`). Keep them as-is — CI should not hit a real Supabase
project. Supabase only matters for staging/production environment config.
