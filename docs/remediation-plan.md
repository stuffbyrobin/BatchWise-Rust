# Remediation plan

Source: security and efficiency review, 2026-09-15. Each phase is one PR off
`main` (except Phase 0, which lands on the open branch). Phases are ordered so
that shared infrastructure is built **before** the modules that depend on it,
so nothing gets fixed twice. Backend phases (1–8) and frontend phases (9–13)
are independent of each other and can be interleaved or split between people.

Size key: S = under an hour, M = half a day, L = a day or more.

Before every commit: `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --lib`
and, for frontend changes, `pnpm exec tsc --noEmit && pnpm test && pnpm build`.

---

## Phase 0 — Finish the open branch (`feat/hop-yeast-dropdowns`) · S

Small, self-contained, and blocks nothing. Land it first so later frontend
refactors start from `main`.

- [x] Key custom yeast rows by a stable id, not array index (`customYeastRows` in `RecipeEditorPage.tsx`). Give `Yeast` a client-side `key`/`step_order` like hops and malts, or reuse the `nextId` pattern from `RecipeWaterChemistry.tsx`.
- [x] Wrap the bodies of `useMaltOptions`, `useHopOptions`, `useYeastOptions` in `useMemo` keyed on `[generic.data, stock.data]` and return one memoized object.
- [x] Hoist the three `NUMERIC_*_FIELDS` sets to module scope.
- [x] Fix `openapi.yaml` `Yeast` schema (also added the missing `form`/`strength_pct` mineral-addition fields that PR #42 hand-edited into `generated.ts`) (`lab`/`attenuation_min`/`attenuation_max` → `manufacturer`/`attenuation_min_pct`/`attenuation_max_pct`), run `pnpm types`, delete the `LibYeast` cast in `useYeastOptions.ts`, and fix the same stale names in the library YeastsPage.
- [x] Use `step_order` (or the new id) as the React key on all four ingredient tables instead of `index`.
- [ ] Merge.

---

## Phase 1 — Correctness hotfix: batch costing and traceability · S

Standalone, highest business impact, no dependencies. Wrong numbers are being
produced today.

- [x] `src/batch/service.rs` ~411: loop over **all** `result.allocations`, not just `.first()`.
- [x] Carry real cost: `cost_pence` = lot cost weighted by `amount_deducted` (lot cost is on the `Ingredient` rows returned by `select_for_deduct`; extend `DeductAllocation` to carry it).
- [x] Update the module header comment in `src/batch/service.rs` to describe the accounting.
- [x] Integration test: batch spanning two lots → two `batch_ingredients` rows with non-zero cost; `GET /batch-costs/{id}` reports the sum; `trace_ingredient_lot` on the second lot finds the batch.
- [x] `src/procurement/service.rs` `receive_po`: open a transaction, run every `update_line_received_qty` and the final `update_po` on `&mut *tx`, commit once.
- [x] Validate `received_quantity <= line.quantity` (or make over-receipt an explicit business rule).
- [x] Add `///` docs to every `pub fn` in `procurement/service.rs` and `equipment/service.rs` while in there (currently zero).

---

## Phase 2 — Backend platform foundations · M

Do this **before** Phases 5–8. It deletes ~80 duplicated definitions and fixes
three findings (uncapped page size, offset overflow, 500s on DB constraint
violations) in one place instead of 30.

- [x] `src/platform/pagination.rs`: `PageParams { page, page_size }` with a clamping constructor (default 20, max 100, `audit` may override to 200), `limit()`/`offset()` using saturating math, and one generic `Page<T>`.
- [x] Replace the 18 `clamp_page` copies, 19 `Page<T>` structs, 30 `OFFSET (page - 1) * page_size` sites, Normalise the two `i32` modules (`library`, `water`) to `i64`. (The 34 handler `ListQuery` structs were left as-is: they differ in filter fields and only share `page`/`page_size`/`sort`; a `#[serde(flatten)]` paging struct is a possible later tidy-up, not a defect.)
- [x] `src/platform/sort.rs`: promote `library::repository::parse_sort` to `sort::parse(spec, ALLOWED, default) -> Result<String, ApiError>`; each module declares `const ALLOWED_SORT: &[(&str, &str)]`. Replace all 27 sort resolvers. Decided: unknown columns are rejected with a 400 validation error everywhere (14 resolvers previously fell back silently).
- [x] `src/platform/errors.rs`: in `From<sqlx::Error>`, map SQLSTATE `23505` → 409, `23514` → 422, `23503` → 409/422, `22001` → 400. Delete the 11 `is_unique_violation` copies.
- [x] Make the validator→`ApiError` helper in `platform/web.rs` `pub(crate)` (needed by Phase 6).
- [x] `sqlx` LIKE helper: `platform::sql::like_contains(term)` that escapes `\ % _` and appends `ESCAPE '\'`; swap in at the 15 LIKE sites.
- [x] Tests: `page_size=1000000` is clamped, `page=i64::MAX` does not panic, `?sort=bogus` is rejected on two representative endpoints, a CHECK violation returns 4xx.

---

## Phase 3 — Auth hardening · M

All in `src/auth` and `src/platform/config.rs`; one PR, one test file.

- [x] `config.rs:66`: run `validate_production` for every `app_env != "development"` (and enforce minimum `JWT_SECRET` length unconditionally).
- [x] Refresh rotation: `UPDATE refresh_tokens SET used_at=now() WHERE id=$1 AND used_at IS NULL` and require `rows_affected()==1`; on replay of a used token call `delete_refresh_tokens_for_user` (family revocation).
- [x] Wrap `hash_password` and `verify_password` calls in `tokio::task::spawn_blocking`.
- [x] Login: verify against a fixed dummy PHC hash when the email is unknown (timing oracle). Use `hasher()` in `verify_password` too.
- [x] Register: kept the 409s (usability) and documented the enumeration trade-off in the service; login now hashes against a dummy PHC string on unknown email so timing no longer distinguishes.
- [x] Refresh token: 32 bytes from `OsRng`, base64url, instead of ULID.
- [x] Rate limiter: idle keys swept every 1024 ops; `TRUST_PROXY_HEADERS` (default false) keys by the last `X-Forwarded-For` entry; per-account failed-login limiter (10/min, `AppState::login_failures`).
- [x] Bootstrap registration: kept the system-tenant behaviour (it is the only API path for curating the shared reference library) but documented it in the service, `.env.example`, and a startup warning. Default remains off.
- [x] Manual `Debug` for `Config` that redacts `jwt_secret` and `database_url`.
- [x] Add `#[validate]` to `tenant_name` (length), `country` (exactly 2), `display_name`, and `password` (max 128) on the auth and tenant DTOs.
- [x] Tests: concurrent refresh yields exactly one success; replayed token revokes the family; weak secret rejected when `APP_ENV` is unset.

---

## Phase 4 — HTTP middleware in `src/app.rs` · S

Single file. Do after Phase 3 so the global rate limiter reuses the fixed
limiter.

- [x] `CorsLayer` from `cfg.cors_origin` (allow credentials off, explicit methods/headers). `CORS_ORIGIN` is now a comma-separated list, validated at startup.
- [x] `SetResponseHeaderLayer` for `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, and HSTS when `app_env != development` (same fail-closed `Config::is_development()` as the config checks).
- [x] `TimeoutLayer` (30s, 503) and an explicit `DefaultBodyLimit` (2 MiB; brand-asset upload gets file limit + 64 KiB framing). Over-limit bodies now return 413 `payload_too_large` instead of a 400.
- [x] Global rate limit from `rate_limit_default_per_minute` on `/api/v1` (shared `platform::middleware::rate_limit`, also used by the auth routes).
- [x] `TraceLayer` for request logging (INFO; method, path without query string, request id).
- [x] Swagger UI: pinned `swagger-ui-dist@5.32.15` with SRI `integrity` hashes (docs stay public).
- [x] Update the config doc comments so they describe what is now actually applied.

---

## Phase 5 — Tenant isolation · M

Write the test harness **first**; it then guards every change in this phase
and in Phases 6–7.

- [x] Integration test `tests/tenant_isolation.rs`: create two tenants, create one resource of each type in tenant A, assert tenant B gets 404 on `GET/PATCH/DELETE /{id}` across all ~20 id routes (parameterised table). Covers ~150 probes (GET/PUT/PATCH/DELETE and POST sub-actions on every id route, core and pro tiers); failures are collected so one run lists every leak, and the owner re-checks existence afterwards. All id routes already returned 404.
- [x] `src/traceability/repository.rs:199`: `LEFT JOIN orders o ON o.id = dm.order_id AND o.tenant_id = dm.tenant_id`.
- [x] Add ownership checks (copy the `batch_exists(pool, tenant_id, id)` pattern) for client-supplied foreign keys in: `calendar` (batch_id), `packaging` (batch_id, order_id), `tracking::fill` (batch_id), `yeastbanking` (library_yeast_id, batch_id), `batch::create` (fermenter_id), `recipe` (style_id, equipment_profile_id, mash_profile_id). Implemented once as `platform::refs::ensure_ref` (shared library tables also accept system-tenant rows; a foreign id gets the same 400 as a nonexistent one). The test found more than the review listed, all fixed: batch update `fermenter_id`, label design `batch_id`/`recipe_id`, water adjustment `target_profile_id`/`batch_id`/`recipe_id` (create, PUT, PATCH), recipe child yeast `yeast_id`, recipe PUT/PATCH, yeast-bank propagation `batch_id` (no FK at all). `packaging` `order_id` was already checked.
- [x] Add the redundant `AND tenant_id = $n` to the id-only mutations in `inventory/repository.rs:202`, `procurement/repository.rs:423/440/454` (lines have no tenant column, so they are scoped through their purchase order).
- [ ] Same for `recipe/repository.rs` `update_calculations` and the four child `DELETE`s. Not done: the child tables have no `tenant_id`, and every caller runs them in the same transaction immediately after a tenant-scoped insert/update of the recipe. Low value; fold into Phase 8's recipe child-row rewrite.
- [x] Extend the isolation test to cover the FK cases (tenant B's batch_id in tenant A's calendar event → 404/422). `foreign_references_are_rejected`: 43 cases, each sent with the other tenant's id and with a random id; the foreign id must be rejected with the same status as the random one.

---

## Phase 6 — Input validation · M

Depends on Phase 2 (shared validator helper, 4xx error mapping).

- [x] `recipe::service::import`: call `req.validate()` immediately after parsing. (A 10k-character BeerXML name and a `NaN` amount were both being saved.)
- [x] Reject non-finite floats globally: a custom validator `finite` applied to every `f64` DTO field, or a serde helper. Narrowed: serde_json refuses NaN/Infinity and overflowing numbers, so no JSON body can carry one; they only arrive through the BeerXML/Brewfather importers (text parsing accepts `NaN`) or arithmetic. `finite` is applied to every float in the recipe request/input types the importers produce, and `validator`'s `range` is not relied on (NaN passes it).
- [x] PATCH DTOs: add the same `length`/`range`/enum rules as the matching CREATE DTO in `recipe`, `inventory`, `library` (×3), `tracking`, `sales`, `tenant`, `procurement`, `equipment`. Consider a small macro to share the attribute sets. Found by diffing every Patch/Create pair: inventory (13 fields), recipe (11), library (5 names), label design, supplier, PO, cost rate, customer, container asset, water profile/adjustment, yeast kinetics. Equipment had no gaps. No macro: the attributes are copied verbatim and the test pins them. Three of these gaps were 500s (empty cost-rate `effective_from`, zero adjustment volume, huge labor hours). Error `details.field` now reports raw identifiers by their JSON name (`type`, not `r#type`).
- [x] Attach the dead `validate_event_type` in `sales/models.rs`; add `range(min=1)` to order item `quantity`; validate `ibu_method` as an enum and `next_batch_number`/`next_order_number` as `>= 1`. `validate_event_type` was deleted instead: no request carries an event type (duty events are created by fulfilment). `ibu_method` accepts `tinseth`/`rager`, matching the OpenAPI enum.
- [x] `#[validate(length(max = N))]` on every request `Vec` (recipe children, PO lines, allergens, water additions). Recipe children and batch ingredients 100, mash steps 50, PO receive lines 500, water additions 50, inventory and label allergens 50.
- [x] `ComputeBatchCostRequest`: validate ranges; use `checked_add` in `reporting/service.rs`; reject NaN/inf before `round_half_away`.
- [x] Brand assets: sniff PNG/JPEG magic bytes at upload; decode with `image::io::Reader::with_limits` in `pkg/labelkit/render.rs`; run `render_pdf` in `spawn_blocking`; serve assets with `Content-Disposition: attachment`. Upload also reads only the header to reject anything over 4096×4096 and requires the bytes to match the declared content type.
- [x] Tests: PATCH rejects what POST rejects (one per module), BeerXML with `NaN` amount and 10k-char name is rejected, oversized image is rejected. `tests/input_validation.rs` (6 tests, ~70 cases, each must be a 400 on the named field).

---

## Phase 7 — Transactions and concurrency · M

- [x] `sales::fulfil`: `SELECT … FOR UPDATE` on the order inside the transaction; preload batches with one `= ANY($1)` query before opening the tx. (Before: 8 concurrent fulfils all succeeded and crystallised duty 8 times.)
- [x] `batch::create`: one `select_by_batch_number … FOR UPDATE` on `&mut *tx`, remove the duplicate pool read.
- [x] `packaging` stock check: read `stock_remaining` and insert the movement in one transaction with a row lock on the packaging run. The stock query aggregates, so the lock is a separate `lock_run`; `delete_run` takes it too before its has-movements check. (Before: 20 concurrent sales of 1 unit all succeeded against a run of 10.)
- [x] Audit writes: pass the caller's transaction so a dropped audit row fails the operation (or document why fire-and-forget is acceptable). `audit::service::write` now takes any executor and returns errors. Duty compile/submit, label create/patch/delete and packaging run/movement create/delete write the change and its audit row in one transaction; the read-only allergen computation and recall query write on the pool but no longer ignore failures.
- [x] Tests: double `POST /orders/{id}/fulfill` produces one duty event; concurrent packaging sales cannot oversell. `tests/concurrency.rs` (also: concurrent batch creates with one number leave exactly one batch). Stable across repeated runs.

---

## Phase 8 — Backend efficiency and docs · M

- [x] Recipe child rows: `QueryBuilder::push_values` (or `UNNEST`) instead of per-row INSERT in `recipe/repository.rs` (×4). Also closes the Phase 5 leftover: the child DELETEs only match a recipe of the caller's tenant, and `update_calculations` filters on `tenant_id`.
- [x] Dashboard: `tokio::try_join!` across the independent queries; add `count_*` repository functions instead of `list(page_size=1)`. (`inventory::count_expiring_within`, `recipe::count`.)
- [x] Feature flags: TTL cache in `check_feature` (or fold into JWT claims and re-issue on change); remove the duplicate tenant query in `dashboard.rs`. `platform::features::FeatureCache` (60 s TTL): a cached "enabled" is trusted, a cached "disabled" is always re-read, so enabling a feature applies immediately and only revocation can lag. Invalidated on tenant update; the dashboard uses it too.
- [x] Indexes migration: `(tenant_id, created_at DESC)` on `recipes` and `label_records`; `(tenant_id, name)` on `styles`, `equipment_profiles`, `mash_profiles`, `yeasts`; `pg_trgm` GIN on searched name columns (or switch to prefix match). `000029_list_and_search_indexes`: `yeasts` is already covered by its unique `(tenant_id, name, …)` index, `library_fermentables` added; trigram indexes are on the exact searched expressions (`lower(name)`, recipe `name`, yeast `manufacturer`).
- [x] `equipment` get-by-id: fold the maintenance cost SUM into the main query.
- [x] Replace the hand-rolled base64 decoder in `import_beerxml.rs` with the `base64` crate.
- [x] Add a comment in `import_beerxml.rs` noting quick-xml does not resolve external entities (so a parser swap doesn't regress XXE).
- [x] Fix stale comments: `sales/models.rs:225`, config docs (done in Phase 4), batch service header (done in Phase 1).
- [x] CI: add `cargo audit` (or `cargo deny`) to `backend.yml`. It found 8 advisories: `quick-xml` 0.36 → 0.42 (two DoS issues reachable through BeerXML import), `h2`/`rustls` patch bumps; the other four are ignored in `.cargo/audit.toml`, each with its reason and exit (`lopdf` via printpdf and `idna` via validator need major upgrades, `rsa` is never compiled, `tokio-tar` is test-only).
- [ ] Follow-up: upgrade `printpdf` to 0.8+ and `validator` to 0.20+, then drop their `.cargo/audit.toml` entries.

---

## Phase 9 — Frontend API client and auth · S/M

One PR touching `src/api/client.ts`, `src/auth/*`, `LoginPage`, `RegisterPage`.

- [x] Logout: `apiClient.post('/api/v1/auth/logout', { refresh_token })` in `try/finally` so `tokenStore.clear()` and `setUser(null)` always run; make `TopBar.handleLogout` navigate in `finally`. (Logout previously sent no `refresh_token`, so the backend's 400 left the user signed in.)
- [x] Single-flight refresh: module-level `refreshPromise`; all 401 handlers await it and re-read the token. Auth endpoints (login/register/refresh/logout) never trigger a refresh, so a wrong password shows the server's message instead of "Session expired".
- [x] Remove `window.location.href = '/login'` from the client; clear the store and let `ProtectedRoute` redirect (preserving `?from=`, including `location.search`). `AuthProvider` subscribes to the store and drops the user when tokens are cleared.
- [x] `ProtectedRoute`/`LoginPage`: allowlist `from` to same-origin paths (`startsWith('/') && !startsWith('//')`). `auth/redirect.ts` `safeRedirectPath` (also rejects `/\`).
- [x] Query defaults: `retry` only on 5xx/network; `refetchOnWindowFocus: false` for editor queries (`useRecipe`, `useLabelDesign`, `useBatch`, `useWaterAdjustments`).
- [x] Pass TanStack's `signal` through `apiClient` (`...init` already spreads it). All 61 `queryFn`s, via a codemod that only rewrote the exact `() => apiClient.get(url)` shape.
- [x] Keep the access token in memory only; persist just the refresh token (or move refresh to an httpOnly cookie if backend work is in scope). Zustand `partialize`; on reload `AuthProvider` loads `/me`, whose 401 refreshes first.
- [x] Guard the test-mode hook with `import.meta.env.DEV`. Verified absent from `pnpm build` output even with `VITE_TEST_MODE=true`.
- [x] Make Login/Register real `<form onSubmit>` elements (with `autoComplete` and `required`).
- [x] Tests: logout clears store even when the request fails; three parallel 401s trigger one refresh. Also: failed refresh clears without navigating, login 401 is not refreshed, the abort signal reaches fetch, only the refresh token is persisted, a cleared store drops the user, and `safeRedirectPath` cases (26 frontend tests, up from 10).

---

## Phase 10 — Frontend tooling and CI · S

Do **before** the large refactors so they run under lint.

- [x] `pnpm update react-router-dom` to ≥ 6.30.6; `pnpm audit --prod` clean. 6.30.6 still left two moderate advisories fixed only in 7.18, so this went straight to react-router-dom 7.18.4 (the app's router usage is v7-compatible); `pnpm audit --prod` reports no known vulnerabilities.
- [x] Add ESLint (`typescript-eslint`, `eslint-plugin-react-hooks`) with `react-hooks/exhaustive-deps` as error; fix or justify every existing violation. 35 findings fixed, none suppressed: hooks called inside callbacks (YeastKineticsPage), two dependency bugs (BatchCreatePage, CalendarPage), `any` removed from the reporting pages. Typing those pages exposed that Batch Costs read nine non-existent fields, so every cost column showed "-" (fixed), and that the reporting routes were served under a `/reporting` prefix the contract and frontend don't use (fixed separately in #55, with a test that routes every OpenAPI operation).
- [x] `frontend.yml`: add `pnpm audit --prod --audit-level=moderate`, `pnpm lint`, `pnpm types && git diff --exit-code src/api/generated.ts`, and the Playwright e2e job against the backend. The types check already caught drift (`cost_pence` on deduct allocations). The e2e harness still targeted the Go backend (`make build`, `bin/batchwise`); it now builds/uses the Rust binary, and both specs pass locally after updating stale steps (required display name, two selects on the batch form, `/calendar-events`). CI pnpm is 11 to match the lockfile writer.
- [x] Remove unused deps: `@radix-ui/react-{dialog,popover,select,tabs}`, `@axe-core/react`, `axe-core`, `autoprefixer`, `postcss`. Delete `PhysicsDemo.tsx` and `Spinner.tsx` (or wire Spinner into Suspense in Phase 13). `PhysicsDemo.tsx` deleted; `Spinner.tsx` kept for the Phase 13 Suspense fallback.

---

## Phase 11 — Frontend shared utilities · M

Depends on Phase 10 (lint) and precedes Phase 12 (the editor split should use
these).

- [ ] `src/api/qs.ts` (or `apiClient.get(path, { params })`); delete the 21 copies.
- [ ] `src/api/types.ts`: one `Page<T>` type; delete per-file redeclarations.
- [ ] `src/utils/format.ts`: `fmtDate`, `fmtPence`; replace 8 + 7 copies.
- [ ] `src/components/ui/Skeleton.tsx` and one shared `inputCls` (or an `<Input>` component).
- [ ] `createCrudHooks<T>(path, queryKey)` factory; collapse `useLibrary.ts`, `useProcurement.ts`, `useEquipment.ts`, `useContainerAssets.ts`.
- [ ] Remove `any` in the three reporting pages and `BatchDetailPage` using `components['schemas']` types.
- [ ] Unit tests for the pure helpers (`format`, `qs`, `ibu`, `ebc`, `SortableHeader.parseSort/nextSort`, `mineralForms`).

---

## Phase 12 — Recipe editor and forms · L

- [ ] Option hooks request `page_size: 200` but the API clamps to 100 and they never page; fetch until `total_pages` (or raise the cap for reference lists) so large inventories are not silently truncated. Also share `midpoint()`/stock aggregation across the three hooks.
- [ ] Extract `features/recipes/editor/`: `<FermentableRow>`, `<HopRow>`, `<YeastRow>`, `<MashStepRow>` (each `React.memo`), a generic `useIngredientRows<T>(numericFields)` hook (add/remove/update/patch/pick + custom-row set), and a shared `<NumberCell>` with `aria-label`.
- [ ] Hydrate the editor once (`useRef` guard or key on `recipeData.id`), same in `RecipeWaterChemistry.tsx`.
- [ ] Apply the same row hook to `BatchDetailPage`'s `IngredientsEditor`.
- [ ] Move the import parsers out of `InventoryImportPage.tsx` into `features/inventory/importers.ts` with unit tests; add file size guards before `FileReader`/upload (logo ≤ 2 MiB, imports ≤ N MB).
- [ ] Accessibility pass: `htmlFor`/`aria-label` on the ~116 unassociated labels (start with the five worst files); convert the `div onClick` expander in `WaterAdjustmentsPage` to a `<button aria-expanded>`.
- [ ] Add JSDoc to shared hooks and components.

---

## Phase 13 — Frontend delivery polish · S/M

- [ ] Route-level code splitting: `React.lazy` for every protected page, `<Suspense fallback={<Spinner/>}>`; verify the landing-page chunk shrinks.
- [ ] Replace `window.confirm`/`alert` with the existing toast/dialog primitives (or drop `ToastProvider`); memoize the toast context value.
- [ ] `LabelDesignEditorPage`: revoke the previous blob URL before replacing it; add `noopener` unless the handle is needed.
- [ ] Plan the react-router v7 and zustand v5 upgrades. (react-router v7 is done, in Phase 10; zustand v5 remains.)

---

## Deferred / roadmap (not defects, but decide before multi-seat sales)

- [ ] Role-based access: add a `role` column and a `require_role` layer; today the only check is `is_owner` on tenant settings, and every member can delete everything.
- [ ] User invite flow (a tenant cannot gain a second member via the API).
- [ ] Distributed rate limiting (Redis) once there is more than one replica.
- [ ] Access-token revocation (`jti` denylist) if `JWT_EXPIRY_MINUTES` is ever raised above ~15.
