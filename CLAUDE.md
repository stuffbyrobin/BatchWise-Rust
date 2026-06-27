# CLAUDE.md

Guidance for working in this repo. BatchWise-Rust is a Rust/Axum backend
(`src/`) with a React + Vite frontend (`frontend/`), backed by Postgres
(`migrations/`, embedded via `sqlx::migrate!`).

## Before committing — run what CI runs

CI (`.github/workflows/backend.yml`, `frontend.yml`) gates on more than a build.
`cargo build` does **not** check formatting, so a clean build can still fail CI.
Run these locally before committing the matching code:

**Backend (Rust) — the `check` job:**
```bash
cargo fmt --all -- --check      # formatting (CI fails on diffs; cargo build won't catch this)
cargo clippy --all-targets -- -D warnings   # lints as errors; --all-targets includes examples/ & tests
cargo test --lib                # unit tests
```
Use `cargo fmt --all` (no `--check`) to auto-fix formatting. The separate
`integration` job runs `cargo test --tests` against a Postgres service — these
also run locally if Docker/Podman is available (testcontainers) or with
`TEST_DATABASE_URL` set.

**Frontend (`cd frontend`) — uses pnpm, not npm:**
```bash
pnpm install --frozen-lockfile
pnpm exec tsc --noEmit          # type check
pnpm test                       # vitest
pnpm build
```

## Toolchain notes

- **Frontend is pnpm.** The committed lockfile is `pnpm-lock.yaml`; CI runs
  `pnpm install --frozen-lockfile`. Do not commit a `package-lock.json` (npm).
- There are two pre-existing TS errors in test files (`@testing-library/react`
  `screen` export) unrelated to app code — don't treat them as new failures.

## Running the app locally

- Backend: `cargo run` (applies migrations on startup, serves `:8080`). Reads
  config from env / `.env` (`DATABASE_URL`, `JWT_SECRET`). `cargo run -- --seed`
  loads reference data. See `docs/supabase.md` for connecting to Supabase — on
  an IPv4-only network use the **session pooler** host, not the direct
  (IPv6-only) connection.
- Frontend: `cd frontend && pnpm dev` (`:5173`); Vite proxies `/api` → `:8080`.
- Health check: `GET /healthz`. API is under `/api/v1`; docs at `/api/v1/docs`.

## Conventions

- **List endpoints sort server-side** via `?sort=col` (leading `-` = DESC), with
  a per-resource allow-list (`build_sort` / `*_order_by` / `parse_sort`). Only
  real scalar DB columns may be added. The frontend uses the shared
  `components/ui/SortableHeader` for clickable headers.
- **Feature gating:** routes are gated by `check_feature` against the tenant's
  `feature_flags` (tiers: `home` < `pro`/`enterprise`, see
  `src/tenant/presets.rs`). The tenant API can't self-upgrade tier; to give a
  test user full access run `cargo run --example grant_all_features -- <email>`.

## Database migrations

- **Filename padding is inconsistent — match the highest existing file, don't
  assume.** `sqlx::migrate!` derives the integer *version* from the leading
  digits of the filename, ignoring zero-padding. Migrations `0001`–`0010` use
  4-digit padding, but **from 11 onward they switch to 6 digits**
  (`000011_…` … `000028_…`). So `0011_foo` and `000011_bar` both parse to
  version **11** and collide — sqlx then fails with `duplicate key … version=11`
  / `VersionMismatch(11)` against any DB that already has the real 11 applied.
  `ls migrations | tail` is misleading here: `000011…` sorts *before* `0001…`.
  Find the true latest version with
  `ls migrations/*.up.sql | sed -E 's#.*/0*([0-9]+)_.*#\1#' | sort -n | tail -1`
  (or check the live `_sqlx_migrations` table), then add the **next 6-digit
  number** with both `.up.sql` and `.down.sql`.
- Migrations are applied on startup (`database::migrate`) and embedded via
  `sqlx::migrate!("./migrations")`; they must be idempotent-safe to re-run.

## Git

Commit/push only when asked; branch off `main` first. Open PRs with `gh` and
wait for CI to go green before merging.
