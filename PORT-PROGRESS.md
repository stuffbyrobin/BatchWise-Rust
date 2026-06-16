# Port progress

Tracks the Go → Rust conversion, following the original BatchWise build order.
Each phase is independently compiling and tested before moving on.

Legend: `[x]` done · `[~]` in progress · `[ ]` not started

## Foundation & physics

| Item | Status | Notes |
|---|---|---|
| Cargo project + module layout | [x] | axum 0.8, sqlx 0.8, tokio |
| Migrations ported (`migrations/*.sql`) | [x] | 26 schema migrations + seed/, embedded via `sqlx::migrate!` |
| `platform::config` | [x] | env loader + production validation, tested |
| `platform::context` | [x] | `RequestContext` extractor |
| `platform::errors` | [x] | `ApiError` + JSON rendering, tested |
| `platform::web` | [x] | `ValidatedJson` extractor |
| `platform::database` | [x] | pool + migration runner |
| `platform::logger` | [x] | tracing setup |
| `pkg::gravity` | [x] | tested |
| `pkg::color` | [x] | tested |
| `pkg::bitterness` | [x] | Tinseth/Rager/Garetz, tested |
| `pkg::water` | [x] | tested |
| `pkg::energy` | [x] | tested |
| `pkg::duty` | [x] | UK HMRC beer duty, tested |
| `pkg::nutrition` | [x] | tested |
| `main.rs` + `GET /healthz` | [x] | request-id middleware |
| CI workflow (fmt/clippy/test) | [x] | |

## Platform middleware

| Item | Status |
|---|---|
| Auth (JWT parse) → RequestContext middleware | [x] |
| RateLimit middleware (per-IP sliding window) | [x] |
| Request-id + error-body stamping | [x] |
| CORS / SecurityHeaders | [ ] |
| FeatureGate / TierGate | [x] feature-flag gate (403 with required_feature/current_tier) |

## Domain modules (mirrors original phases)

| # | Module | Status |
|---|---|---|
| 01 | auth + tenant | [x] argon2 + JWT + refresh rotation, integration-tested |
| 02 | library + inventory (FIFO) | [x] seed runner + FIFO/overdraft, integration-tested |
| 03 | recipe | [x] nested children + physics calc + BeerXML/Brewfather import, integration-tested |
| 04 | batch + calendar + yeastkinetics | [x] FSM + snapshot + calendar-gen + deferred deduction, integration-tested |
| 05 | tracking + reporting (tier-gated) | [x] FeatureGate + QR codes + cost reports/duty, integration-tested |
| 06 | dashboard + OpenAPI serving | [x] aggregated stats + /openapi.yaml + /docs, integration-tested |
| 09 | sales | [x] customers + order FSM + line items + duty events (tier-gated), integration-tested |
| 10 | batch cost / profitability | [x] covered by reporting (batch-costs compute) in Phase 5 |
| 11 | water chemistry | [x] profiles (system union) + adjustments + /calculate via pkg::water, integration-tested |
| 12 | beer duty & excise records | [x] duty-returns compile/submit (SPR via pkg::duty), tier-gated, integration-tested |
| 13 | allergen & label compliance | [x] /recipes/{id}/allergens + label-records (auto-populated), tier-gated, integration-tested |
| 14 | packaging / distribution / traceability | [x] packaging-runs + distribution-movements (stock-remaining math, insufficient-stock 422) + forward/backward/recall traceability, both tier-gated, integration-tested |
| 15 | trading standards audit | [x] compliance-audit log (read-only, tenant-scoped, NOT feature-gated) + fire-and-forget audit writes wired into packaging, traceability, labels, duty & allergens, integration-tested |
| 16 | procurement | [x] suppliers + purchase-orders (nested lines, PO-number gen via FOR UPDATE, status FSM, partial/full receive), tier-gated "procurement", integration-tested |
| 17 | yeast banking | [x] yeast-bank entries (computed days_in_storage, harvest → generation bump, discarded terminal guard) + nested propagation events, home-tier "yeast_banking", integration-tested |
| 18 | fermentation tracking | [x] fermentation readings (gravity/temp/pH/stage) under /batches/{id}/fermentation, batch-scoped, home-tier "fermentation", integration-tested |
| 19 | label & print design | [x] brand-assets (BYTEA upload/serve) + brand-profiles + label-designs (kind/size validation) + render model & print-ready PDF (printpdf), tier-gated "label_design", integration-tested |
| 20 | equipment maintenance | [x] equipment register + nested maintenance schedules + events + cross-equipment maintenance-due feed (SQL-computed next-due/overdue/days-until/lifetime-cost), tier-gated "equipment_maintenance", integration-tested |

## Frontend

The React 19 frontend is bundled into this repo under `frontend/` so it is a
standalone full-stack version. It speaks the same `/api/v1` contract; the Vite
dev server proxies `/api` to the Rust backend on `:8080`, so
`cd frontend && pnpm install && pnpm dev` (with the backend running) brings the
whole app up. Builds and type-checks clean (`tsc && vite build`).

A Rust/WASM rewrite (Leptos/Yew) was considered and declined — marginal benefit
for a forms-and-tables SaaS. Instead, `wasm/` is a prototype that compiles the
pure `pkg` physics to WebAssembly (single source of truth via `#[path]`), giving
the frontend the *same* brewing calculations the server runs without a rewrite.
See `wasm/README.md` and `frontend/src/lib/physics/`.

## Notes / deviations

- `pkg::duty`: the Go `CalculateDuty` "fails open" (returns 0 + logs a warning)
  for unknown jurisdictions. The Rust port returns `Err(DutyError)` instead, as
  there is no logger in a pure function. To be reconciled at the reporting-service
  port if the 0-return behaviour is relied upon.
- Physics packages that used Go string-typed enums with runtime "unknown type"
  errors now use real Rust enums, making some error paths unrepresentable
  (and a few `Result` returns became infallible).
- Inventory deduct: a manual deduction (empty `reference_type`) records the
  movement as `"manual"` to satisfy the `stock_movements.reference_type` check
  constraint (the Go service left it empty, which would violate the constraint).
- Seed: `seed::run` is scoped to the Phase 2 files (`001`–`004`). The source
  `006_allergen_lots.sql` inserts 31 system rows all sharing `lot_number =
  'SYSTEM'`, violating `ingredients UNIQUE(tenant_id, lot_number)`; it will be
  fixed (unique lot numbers) when the allergen phase is ported.
- Audit (Phase 15): Go reads the acting user from `context.Context` inside the
  audit service. Rust has no ambient context in the service layer, so the actor
  (`ctx.actor_id`) is threaded explicitly from each handler into the service
  functions that emit audit events. The Phase 5/12 notes about "audit writes
  omitted" no longer apply — duty, labels, allergens, packaging and traceability
  now write their events fire-and-forget via `audit::service::write`.
