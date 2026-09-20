# Design unification, phase 2: internal pages and status colours

Phase 1 (#74) retargeted the shared tokens (`--color-accent`, `--color-danger`,
`--color-success`, the base font, and the default corner radius) to the
landing page's own palette, and hand-rebuilt the shell chrome and auth pages.
Most of the 49 internal pages under `src/features/*` picked up that change for
free, since they already read those tokens for surfaces, borders and text.

This phase covers what phase 1 didn't touch: the internal data pages'
remaining off-brand colours, and the status/category badges scattered across
the app. It's scoping only — nothing here is implemented yet.

## What the audit found

**49 internal page files**, median 235 lines, largest 718 (`RecipeEditorPage`).
30 contain a `<table>` (list pages); 31 already use `bg-[var(--color-surface)]`
for their surfaces, so the token foundation is already in reasonable shape.
The gaps are:

**1. Hardcoded danger/success colours, not tokens (mechanical, ~25 files).**
`text-red-600`, `bg-red-50`/`bg-red-100`, `text-green-600`, `bg-green-100` and
similar appear across 22 files for error text and 16 for success/positive
text — usually inline validation messages and confirmation banners, not
status badges. Beyond being off-brand, this is a real (if minor) correctness
gap: raw Tailwind palette colours don't move with `--color-danger`/
`--color-success`, which do adapt in the app's dark-mode block
(`styles/tokens.css`). A user in dark mode sees a jarring light-red banner
that the rest of the page didn't get.

**2. Five to seven duplicated status-badge implementations, no shared
component.** `PurchaseOrdersPage`, `LabelRecordsPage`, `ComplianceAuditPage`,
`DutyReturnsPage` and `ContainerAssetsListPage` each define their own
`StatusBadge`/`getStatusBadge`/`eventBadgeClass` function, each with its own
colour choices (some raw Tailwind, some already `--color-*`), its own shape
(`rounded-full` vs `rounded`), and its own padding. `AllergenBadges` is a
sixth, separate pill component with the same shape but built independently.
None of this is wrong exactly, but there's no single answer to "what does a
neutral/positive/warning/negative badge look like in this app."

**3. A real bug: two different colour maps for the same batch statuses.**
`features/batches/hooks/useBatches.ts` and `features/dashboard/DashboardPage.tsx`
each define their own `STATUS_COLORS` for the batch FSM
(planned/brewing/fermenting/conditioning/packaging/completed/cancelled/spoiled),
and they disagree — e.g. `fermenting` is `--srm-6` in one and `--srm-7` in the
other, and the dashboard's copy is missing `spoiled` entirely. A batch shows
one colour on its own list and a different one on the dashboard. Independent
of the branding work, this should be a single source of truth.

**4. No shared "elevated card" treatment.** Only 3 files
(`RecipesListPage`, `RecipeEditorPage`, `RecipeWaterChemistry`) use the
`p-4 rounded shadow` filter-panel pattern; the other ~27 list pages use a
flatter `bg-[var(--color-surface)] border` box with no shadow. Neither
matches the landing page's soft, warm-toned card shadow
(`0 24px 60px rgba(122,59,20,.10)`) used everywhere on the marketing site and
now on the auth pages. This is the single biggest remaining visual gap
between "looks like the landing page" and "looks like a generic dashboard."

**5. `CostReportsPage`'s cost-breakdown legend is a different kind of
problem.** It colours six simultaneous categories (ingredients, energy,
labour, water, overhead, duty) with `bg-green-500`, `bg-yellow-500`,
`bg-blue-500`, `bg-cyan-500`, `bg-purple-500`, `bg-orange-500`. This isn't a
status — it's a small multi-series chart legend, and collapsing it to the
single brand accent would destroy the thing it's for (telling six bars
apart). It needs its own deliberately-chosen, brand-consistent *categorical*
palette (six distinct, warm-leaning swatches), not the accent/danger/success
vocabulary the rest of this phase is about.

**6. The `DashboardPage` is the highest-leverage single page.** It's the
first thing every user sees after signing in, and it's structurally close to
the landing page's own hero mockup (a batch status card with telemetry
tiles) — the product the landing page is already advertising. Right now it's
plain bordered boxes with no shadow and the STATUS_COLORS bug above. Bringing
it visibly in line with the landing's own preview of it is worth doing as a
page in its own right, not just inheriting whatever the shared tokens give it.

## Proposed shared building blocks

- **`components/ui/Badge.tsx`** — a single component with a small, closed set
  of tones (`neutral`, `info`, `positive`, `warning`, `negative`), each mapped
  to the app's own tokens (`--color-success`/`-success-bg`,
  `--color-danger`/`-danger-bg`, etc., plus one or two new tokens for
  info/neutral if the existing set doesn't cover them). One shape (`rounded-full`,
  matching the landing's pill language), one padding scale.
- **A canonical `BATCH_STATUS_COLORS`** in `features/batches/hooks/useBatches.ts`,
  imported by `DashboardPage` instead of redefined. Whether batch-status colour
  keeps riding on the SRM beer-colour scale (as today) or gets its own small
  "workflow stage" palette is a real design call worth a decision, not just a
  bug fix — the SRM scale already means something else (a beer's actual
  colour) elsewhere in the app, and reusing it for workflow status is a
  coincidence, not a design decision.
- **A `Panel`/`Card` convention** (a shared class constant, like the existing
  `inputCls` in `components/ui/styles.ts`, or a small wrapper component) for
  the filter box + table container list pages already all have, giving it the
  landing's shadow and radius in one place.
- **A small categorical chart palette** (5–8 warm, distinct swatches) for
  `CostReportsPage` and any other multi-series legend, kept separate from the
  brand accent.

## Proposed phases

- [x] **Phase A — tokenize hardcoded danger/success colours + fix the batch
  status-colour duplication.** Mechanical, scriptable, no visual-taste calls,
  and it fixes a real (if small) dark-mode bug and a real data-inconsistency
  bug. Lowest risk, good first step. (32 files: every raw `bg-red-*`/`text-red-*`/
  `text-green-*`/`text-yellow-*` used for danger/success/warning semantics now
  reads `--color-danger`/`--color-success`/`--color-warning`; `DashboardPage`
  imports the batch-status colour map from `useBatches.ts` instead of keeping
  its own, divergent copy. `CostReportsPage`'s categorical cost-breakdown
  legend was left alone, per Phase E below.)
- **Phase B — the shared `Badge` component**, migrating the five to seven
  existing status-badge implementations onto it.
- **Phase C — the `Panel`/`Card` treatment**, applied to the ~30 list pages'
  filter boxes and table containers. The single biggest visual lift toward
  "looks like the landing page."
- **Phase D — `DashboardPage` redesign**, using Phase A–C's building blocks
  plus its own layout work, since it's the one page worth bespoke attention.
- **Phase E — `CostReportsPage`'s categorical palette**, and cleanup of any
  remaining stray off-brand colours turned up along the way.

Each phase is its own branch and PR, following the workflow used for the rest
of this plan: tests where practical, full local checks plus e2e, PR, wait for
CI, merge only once you've said to.

## Deliberately out of scope

Column choices, form layouts, table density, and business logic on any of
these 49 pages. This phase is about shared visual language (colour, elevation,
badges) that already exists in some form on most pages — not a rewrite of
their structure or information architecture. A page that needs restructuring
for reasons other than colour/elevation is a separate piece of work.
