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
| FeatureGate / TierGate | [ ] |

## Domain modules (mirrors original phases)

| # | Module | Status |
|---|---|---|
| 01 | auth + tenant | [x] argon2 + JWT + refresh rotation, integration-tested |
| 02 | library + inventory (FIFO) | [x] seed runner + FIFO/overdraft, integration-tested |
| 03 | recipe | [x] nested children + physics calc + BeerXML/Brewfather import, integration-tested |
| 04 | batch + calendar + yeastkinetics | [ ] |
| 05 | tracking + reporting (tier-gated) | [ ] |
| 06 | dashboard + OpenAPI serving | [ ] |
| 09 | sales | [ ] |
| 10 | batch cost / profitability | [ ] |
| 11 | water chemistry | [ ] |
| 12 | beer duty & excise records | [ ] |
| 13 | allergen & label compliance | [ ] |
| 14 | packaging / distribution / traceability | [ ] |
| 15 | trading standards audit | [ ] |
| 16 | procurement | [ ] |
| 17 | yeast banking | [ ] |
| 18 | fermentation tracking | [ ] |
| 19 | label & print design | [ ] |
| 20 | equipment maintenance | [ ] |

## Frontend

Decision deferred (per project owner). Options: keep the existing React 19
frontend unchanged (it already speaks the same API), or rewrite in a Rust/WASM
framework (Leptos/Yew). Revisit once the backend port is functional.

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
