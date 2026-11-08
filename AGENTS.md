# AGENTS.md: Nookery

Kitchen inventory that runs entirely on the user's own devices: receipts and
photos in, "what do I have, what do I use first, what do I cook" out. Read
this before changing anything; update it when reality and this file differ.

## Principles

- **Local only.** No cloud APIs for recognition or recipes. Vision runs in a
  local model (Ollama on the Mac); everything else is plain Rust.
- **Performance first.** Cheap, exact steps before the model: barcode (ms) ->
  offline product database (sub-ms) -> OCR for dates (~100 ms) -> vision model
  only for what's left (seconds). Measure: commands report timings (`--time`,
  `openMs`/`queryMs`).
- **A ledger, not a list.** Only events are stored (`acquired`, `seen`,
  `moved`, `opened`, `used`, `binned`, each about one lot = one batch of one
  thing). The inventory is replayed from them (`Inventory::replay`), so every
  number is explainable and nothing is lost. Never store derived state as
  truth; add a new event kind instead.
- **Ask before removing.** Lots not confirmed for a while become
  `Unconfirmed` ("probably used up"); the user confirms. Nothing disappears
  on a guess.

## Layout

- `crates/core`: pure domain, no I/O, no clock (callers pass `today`).
  `Qty` (integer thousandths: 1 piece / 1 g / 1 ml = 1000; parses `2`, `0.5`,
  `500g`, `1,5 kg`, `750ml`, `1l`), `Event`/`EventKind`/`LotId` (UUID v7,
  time-ordered; `short()` = last 8 hex chars, since the start is a
  timestamp), `Inventory` (replay/apply/check; rejects duplicate lots,
  unknown lots, events on gone lots, unit mismatches), `status`/`use_first`
  (expired > use soon (<= 2 days) > fresh by date > unconfirmed > undated;
  freezing pauses the printed date; confirm windows fridge 14 d, pantry 90 d,
  freezer 180 d), `parse_date` (`2026-11-20`, `20.11.2026`, `20.11.`: next
  such day, `+3`).
- `crates/store`: SQLite (rusqlite, bundled, WAL). Table `events(seq, at,
  lot, kind, body JSON)`; migrations by `PRAGMA user_version`; refuses a
  newer schema. `append` validates the whole batch against the inventory
  first and writes it in one transaction (all or nothing). Data folder:
  `$NOOKERY_DATA_DIR`, else `<OS data dir>/dev.nookery.app` (= the Tauri
  identifier, so CLI and app share `nookery.db`).
- `crates/cli`: `nookery add|ls|use|open|bin|move|seen|log` (`--time`,
  `--data`). Lots are addressed by their short id.
- `src-tauri`: thin commands (`get_inventory`, `add_item`, `lot_action`)
  that convert to UI types (`LotView`, `LotStatus`, `CommandError`). Typed
  bindings via tauri-specta: `ui/src/lib/bindings.ts` is GENERATED; run
  `cargo test -p nookery` after changing a command and commit the result.
- `ui/`: SvelteKit (static, SPA) + Svelte 5 runes + Tailwind 4, Inter.
  Imports use `#lib/...` with `.js` extensions. `lib/api.ts` is the only
  door to the backend: in Tauri it calls the bindings, in a plain browser it
  serves SYNTHETIC sample data (`pnpm -C ui dev`, then screenshot with
  Playwright from finopsis's `ui/node_modules`). Colour tokens in
  `routes/layout.css` (light + dark): sage = fresh, saffron = use soon,
  terracotta = expired, ice = freezer.

## Commands

- Rust: `cargo fmt --all && cargo clippy --workspace --all-targets && cargo test --workspace`
- UI: `pnpm -C ui check && pnpm -C ui test && pnpm -C ui exec prettier --check src`
- App: `pnpm tauri dev` from the repo root. Use
  `NOOKERY_DATA_DIR=$(mktemp -d) pnpm tauri dev` while developing so the
  real kitchen data is never touched.
- CLI: `cargo run -p nookery-cli -- ls --time`

## Measured (M0, 2026-11-08, M5 MacBook, release build)

- Open + replay 5 events 0.34 ms; 10,005 events 6.4 ms; listing 5,000 lots
  4.4 ms (mostly printing).

## Roadmap

1. **M0 (done)**: workspace, ledger, store, CLI, Tauri shell with
   Use first / Fridge / Freezer / Pantry and quick add.
2. **M1 receipts** (`crates/receipt`): paper photos (OCR + per-store
   profiles: line format, weights `0,512 kg x 2,99`, Pfand, discounts,
   multi-buy) and digital receipts (REWE eBon PDF, Lidl Plus). Per-store
   alias table mapping receipt text to products, learned from every
   confirmation. Line sum must equal the receipt total. Stores and test
   receipts: ask the user; real receipts go in git-ignored `fixtures/private/`.
3. **M2 single items**: barcode (`rxing`) + offline Open Food Facts in
   SQLite; OCR (`ocrs`) for best-before dates.
4. **M3 photos of fridge/freezer/pantry**: vision model behind a
   `VisionModel` trait (Ollama `qwen3-vl:8b` first; benchmark `qwen3-vl:4b`;
   later in-process via mistral.rs/Metal). Reconcile against the ledger:
   matched (refresh), expected-but-not-seen (ask), seen-but-never-bought
   (add as `Source::Photo`). Never double-count a receipt line and a photo.
5. **M4 shelf life**: USDA FoodKeeper (public domain) for missing dates,
   opened and freezer windows.
6. **M5 recipes** from stock, scored by use-it-up value and what's missing.
7. **M6 weekly balanced plan** (`good_lp` + HiGHS; nutrition from USDA FDC
   and Open Food Facts; the model writes steps only, never nutrition numbers)
   and a shopping list.
8. **M7 phone app** (Tauri mobile): camera, barcode/OCR on the phone, hard
   cases sent to the Mac over the home network (finopsis's paired, encrypted
   lan-sync pattern).

## Environment notes

- Ollama on this Mac: standalone app in /Applications (not Homebrew),
  updated 0.13.5 -> 0.40.15 on 2026-11-08 (0.13.5 was too old for
  `qwen3-vl`: registry answered 412). `qwen3-vl:8b` pulled the same day.
- Installed app: `pnpm tauri build --bundles app`, then
  `ditto target/release/bundle/macos/Nookery.app /Applications/Nookery.app`.
  Unsigned local build; it uses the real data folder
  (`~/Library/Application Support/dev.nookery.app`).
