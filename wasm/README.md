# Prototype: brewing-physics → WebAssembly

This is a **standalone prototype crate** that compiles the backend's pure
brewing-physics (`src/pkg/*`) to WebAssembly via `wasm-bindgen`, so the React
frontend can run the *exact same* calculations the Rust server runs — no
reimplementation, no round-trip.

It is intentionally **not** part of the main `batchwise` crate's build (it has
its own `[workspace]` table). It includes the `pkg` source files directly with
`#[path = "../../src/pkg/*.rs"]`, so there is a single source of truth: the same
`.rs` files power both the server and the WASM bundle.

## Why this approach
`src/pkg/*` is pure `std` Rust with zero external dependencies and no
`internal/` imports (enforced by the project's layer rules), so it cross-compiles
to `wasm32-unknown-unknown` cleanly. The heavy backend deps (sqlx, tokio, axum)
never enter the WASM build.

## Build
```bash
# one-time tooling
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.100

# build + generate JS/TS bindings into the frontend
./build.sh
```
`build.sh` compiles to wasm and runs `wasm-bindgen --target web`, emitting the
glue + `.wasm` + `.d.ts` into `../frontend/src/lib/physics/wasm/`.

## Use from React
```ts
import { useBrewingPhysics } from "@/lib/physics/useBrewingPhysics";

const physics = useBrewingPhysics();
if (physics.ready) {
  const abv = physics.calculateAbv(1.050, 1.010); // 5.25
}
```

## Productionising later
For a real rollout, extract `src/pkg` into its own workspace crate
(`batchwise-pkg`) that both `batchwise` and this WASM crate depend on, instead of
the `#[path]` includes used here for a zero-impact prototype.
