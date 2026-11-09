# AGENTS.md: Nookery

Kitchen inventory that runs entirely on the user's own devices: receipts and
photos in, "what do I have, what do I use first, what do I cook" out. Read
this before changing anything; update it when reality and this file differ.

## Principles

- **Local by default; cloud only by the user's choice.** Vision and
  recipes run in a local model (Ollama on the Mac). The user can choose, per
  feature (Settings / `nookery settings`), "this Mac, cloud if it fails" or
  "cloud" (Ollama Cloud, `gemma4:31b-cloud`, via the same local Ollama after
  `ollama signin`). Default and any unreadable setting = local. Whenever
  the cloud answered, say so in the UI. Everything else is plain Rust;
  never add a cloud call that isn't behind this setting.
- **Performance first.** Cheap, exact steps before the model: barcode (ms) ->
  offline product database (sub-ms) -> OCR for dates (~100 ms) -> vision model
  only for what's left (seconds). Measure: commands report timings (`--time`,
  `openMs`/`queryMs`).
- **A ledger, not a list.** Only events are stored (`acquired`, `seen`,
  `moved`, `opened`, `used`, `binned`, `retracted` = entered by mistake,
  counted as neither eaten nor wasted; each about one lot = one batch of one
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
  `reconcile`: `name_key` (lower case, no punctuation or container/filler
  words, English singular, words sorted; colours kept) and
  `reconcile(observed, lots, place)`: equal keys first, then word-subset
  matches ("bell pepper" ~ "red bell pepper") whose smaller side has a
  non-colour word; returns matched / missing (ask, never auto-remove) /
  new. `Alias` (receipt or photo text -> item, each, place).
  `category(name) -> FoodCategory` (18 kinds, English + German words and
  compounds; "pepper" = spice, "bell pepper"/"paprika" = vegetable):
  derived for the list icons, never stored. `AiMode` + `CLOUD_MODEL`.
- `crates/store`: SQLite (rusqlite, bundled, WAL). Table `events(seq, at,
  lot, kind, body JSON)`; migrations by `PRAGMA user_version`; refuses a
  newer schema. `append` validates the whole batch against the inventory
  first and writes it in one transaction (all or nothing). Data folder:
  `$NOOKERY_DATA_DIR`, else `<OS data dir>/dev.nookery.app` (= the Tauri
  identifier, so CLI and app share `nookery.db`). The CLI and the app
  write the same file: every write runs in a `BEGIN IMMEDIATE` transaction
  that re-reads the ledger if another process appended (so a stale app
  can't write on a lot the CLI already removed, which would make replay
  fail); `refresh()` (one indexed query) reloads for reads, and the app
  calls it in `AppState::store()` before every command. Migration 2 adds
  `receipts(id, store, date, total_cents, imported_at, text)` and
  `aliases(store, key, body JSON, learned_at)`: aliases are learned memory
  (upserted per store + key), not inventory. `import_receipt` records the
  receipt, appends its events and upserts aliases in one transaction;
  a repeated receipt id is `DuplicateReceipt`.
  `append_learning(events, store, learned, at)` does the same for photo
  confirmations (`PHOTO_ALIASES` = "photo", key = `name_key` of what the
  model said).
  Migration 3: `templates(id, name UNIQUE, body, position, updated_at)`,
  seeded with Use it up / Quick weeknight / High protein / Vegetarian /
  Something new; `templates`, `save_template`, `delete_template`,
  `order_templates`.
- `crates/receipt`: pure text -> ledger. `parse_text` reads German receipt
  text (digital or OCR) into `Line`s with exact cents: item lines
  `text price [A|B]`; count (`2 Stk x 0,99`) and weight
  (`0,512 kg x 2,99 EUR/kg`) lines attach to the item above or below,
  whichever amount matches; Pfand, Leergut and discounts get their own
  `LineKind`; collection stops at SUMME/ZU ZAHLEN. `check_total` demands
  lines = total to the cent. `Receipt::id` (FNV-1a 64 over the content)
  catches re-imports. `propose` guesses from learned aliases (`alias_key`:
  upper case, no price column); `to_events` turns confirmations into
  `Acquired` (`Source::Receipt { receipt: Some(id) }`) plus aliases to
  learn. Chains: REWE, EDEKA, Netto, denn's (the user's stores). Fixtures in
  `tests/fixtures/` are SYNTHETIC; real receipts go in git-ignored
  `fixtures/private/` and should drive per-chain tweaks (none yet).
- `crates/vision`: photo -> sightings with the local Ollama model
  (`qwen3-vl:8b-instruct`; NOT the `qwen3-vl:8b` tag, which is the thinking
  edition and takes minutes). `tiles` cuts the photo (EXIF applied) into
  overlapping zoomed tiles (fridge/freezer 5 strips, pantry 4x2);
  `Ollama::list_items` streams one request per tile (`think: false`,
  temperature 0, no food examples in the prompt, `num_predict` 250) and
  reads `name | printed text | count` lines, cutting off repeats (3 in a
  row), container-only runs (6) and runaway lists (12 items). `merge`
  joins overlapping tiles (max count, not sum). `plan` maps names through
  learned photo aliases and `reconcile`s; `decide` turns the user's
  answers into `Seen` / `Acquired { Source::Photo }` and aliases to learn.
  New sightings are only added when the user says so. Env:
  `NOOKERY_OLLAMA_URL`, `NOOKERY_VISION_MODEL`. Real-photo check:
  `cargo test -p nookery-vision --release --test real_photo -- --ignored --nocapture`.
- `crates/recipes` (M5): ideas and full recipes from stock via a local text
  model (`TextModel`; `OllamaText`, default `qwen3-vl:8b-instruct` = the
  photo model, so nothing else loads; `$NOOKERY_TEXT_MODEL`). The model
  writes text only: `check` decides HAVE / NEED against the ledger with
  `names_match` (model "uses" not in stock -> need; staples dropped;
  expired-only matches -> warning, never an ingredient), score
  `3*use_soon + have - 2*need`. `ideas_prompt` groups USE SOON / FRIDGE /
  FREEZER / PANTRY, leaves expired out, and only asks to prefer use-soon
  items when there are some (otherwise the model invents them).
  Real-stock check (copies the DB, never writes it):
  `cargo test -p nookery-recipes --release --test real_stock -- --ignored --nocapture`.
- `crates/cli`: `nookery add|ls|use|open|bin|move|seen|log` (`--time`,
  `--data`). Lots are addressed by their short id.
  `nookery receipt FILE|-` (`src/receipt.rs`) prints the parsed lines
  (1-based), each item's learned-alias proposal (`-> milk 1l, fridge` or
  `-> ?`) and the lines-vs-total check; read-only unless `--import`.
  `--store` overrides detection, `--set N=item[,qty][,place][,bb]` answers
  a line, `--skip N` leaves it untracked, `--ignore-total` allows a
  mismatch. Import refuses (writing nothing) on unknown store, re-import,
  bad total, or unanswered item lines.
  `nookery photo FILE --in <place>` (`src/photo.rs`) scans, prints the
  sightings (learned name or "seen as", count, `= item left [id]` or
  `new`), the lots not in the photo, and per-tile timings; read-only
  unless `--import` (matched -> seen; `--add N[=item,qty,bb]` adds and
  learns a rename; `--skip N`; unmatched without `--add` left out).
  `nookery recipes [-t NAME] [--note TEXT] [-n N] [--cook N]` (ideas with
  have/need/expired and lot ids; `--cook` streams the full recipe) and
  `nookery templates [add NAME TEXT | edit ID | rm ID | order ID...]`.
  `nookery settings [photos|recipes <local|fallback|cloud> | cloud-model
  NAME | check-cloud]`; `photo` and `recipes` follow the setting, take
  per-run `--local` / `--cloud`, name the model that answered and print
  "sent to the cloud: ..." when it was the cloud.
- `src-tauri`: thin commands (`get_inventory`, `add_item`, `lot_action`)
  that convert to UI types (`LotView`, `LotStatus`, `CommandError`). Typed
  bindings via tauri-specta: `ui/src/lib/bindings.ts` is GENERATED; run
  `cargo test -p nookery` after changing a command and commit the result.
  `src/receipt.rs`: `parse_receipt` / `import_receipt` over plain
  functions (`parse_receipt_in(&Store, ..)`, `import_receipt_in(&mut Store,
  ..)`) so they're tested without Tauri. Import re-parses the text
  server-side (never trusts UI amounts) and refuses a total mismatch, no
  store, or a re-import (`CommandError::AlreadyImported`); field errors are
  `BadInput("<field>[:<line>]")`.
  `src/photo.rs`: `scan_photo` (async: the model runs in `spawn_blocking`
  with the store unlocked; progress via `Channel<ScanProgress>`) keeps the
  last `Plan` in `AppState.last_scan` under a scan id; `apply_photo` runs
  `decide` + `append_learning` and clears it (no double apply).
  `CommandError::{VisionUnavailable, VisionModelMissing}`.
  `src/recipes.rs`: template commands (ids are `u32` in the UI: specta
  can't export `i64` as a number) and `suggest_recipes` (async, model in
  `spawn_blocking`, store unlocked; result kept in
  `AppState.last_suggestions` under an id) / `write_recipe` (streams text
  via `Channel<String>`). `CommandError::{RecipeModelUnavailable,
  RecipeModelMissing}`; `NoStock` -> `BadInput("stock")`.
  `src/settings.rs`: `get_ai_settings` / `set_ai_settings` (modes typed as
  `'local' | 'fallback' | 'cloud'`) and `check_cloud` (one tiny cloud
  request: signed in or not). Scan and recipe commands build `Chosen` /
  `ChosenText` from the stored mode and put `answered_by()` + `usedCloud`
  in their views. A 401 is `CommandError::CloudSignIn`.
- `ui/`: SvelteKit (static, SPA) + Svelte 5 runes + Tailwind 4, Inter.
  Imports use `#lib/...` with `.js` extensions. `lib/api.ts` is the only
  door to the backend: in Tauri it calls the bindings, in a plain browser it
  serves SYNTHETIC sample data (`pnpm -C ui dev`, then screenshot with
  Playwright from finopsis's `ui/node_modules`). Colour tokens in
  Palette in `routes/layout.css` (light + warm dark): a calm autumn
  kitchen. Oat/parchment neutrals, espresso ink; `--sage` (parsley green)
  = fresh + primary action, `--saffron` (lemon-amber) = use soon,
  `--terracotta` (rust) = expired, `--ice` (dusk blue) = freezer;
  `--parsley`/`--lemon`/`--pumpkin` decorative, `--backdrop` modal scrim,
  `--c-*` food-icon tints. Normal text pairs checked >= 4.5:1 both themes.
  Logo: an arched nook with lemons and flat-leaf parsley (keep leaflets
  rounded and scalloped, not spiky): `assets/logo.svg` (app icon tile),
  `assets/logo-mark.svg` (header `lib/Logo.svelte`, favicon). Regenerate
  icons: export logo.svg to `assets/icon-1024.png`, then
  `pnpm tauri icon assets/icon-1024.png`. `lib/FoodIcon.svelte`: 18 food
  category glyphs (24 grid, tinted tile) for `LotView.category`.
  Kitchen (`routes/+page.svelte`) is a thin shell over `lib/kitchen/`
  (`LotRow`, `LotMenu`, `PlaceGroup`, `UseFirst`, `QuickAdd`,
  `UndoToast`, `SelectBar`). Removing (used up / thrown away / wrong
  entry) never writes at once: `pending.svelte.ts` hides the row, shows
  Undo, and writes after 6 s or on `visibilitychange`/`pagehide`/destroy
  (never if undone, never twice). Names shown via `displayName()`; stored
  names untouched.
  `routes/receipt/+page.svelte`: paste or open a `.txt` receipt, review
  and edit each item line (item, amount, place, best before, track),
  sum banner, import; `format.ts` has `formatCents`, `formatDiff`,
  `parseFieldError`.
  `routes/photo/+page.svelte`: place + photo, scan with progress, review
  "Already here" (default Seen), "New?" (default Ignore), "Not in the
  photo" (read-only + Used up / Still there), Apply. `api.ts` serves a
  synthetic review in a browser and exports `progressChannel`.
  `routes/+layout.svelte`: Kitchen / Recipes tab bar (on those two routes;
  photo and receipt keep their own header). `routes/recipes/+page.svelte`:
  template chips, note, count, suggestion cards (have / would need /
  expired warning), streamed recipe, "I cooked it" (marks ticked lots used
  via `lotAction`). `lib/TemplatesDialog.svelte` (native `<dialog>`),
  `lib/recipes-state.svelte.ts` (keeps Recipes state across tabs). The
  photo screen requires an explicit place (a pantry scan once landed in
  the fridge because Fridge was preselected).
  `routes/settings/+page.svelte` (gear in the header): "Where the AI runs"
  per feature, what is sent, cloud model, Check cloud with the
  `ollama signin` fix. `lib/CloudTag.svelte` marks cloud answers (ice);
  `lib/ErrorBanner.svelte` shows the sign-in error with a Settings link.

## Commands

- Rust: `cargo fmt --all && cargo clippy --workspace --all-targets && cargo test --workspace`
- Binaries: the CLI is `nookery`, the app is `nookery-app` (both used to be
  `nookery` and overwrote each other in `target/`). Don't run
  `target/*/nookery-app` by hand: it opens the real database.
- UI: `pnpm -C ui check && pnpm -C ui test && pnpm -C ui exec prettier --check src`
- App: `pnpm tauri dev` from the repo root. Use
  `NOOKERY_DATA_DIR=$(mktemp -d) pnpm tauri dev` while developing so the
  real kitchen data is never touched.
- CLI: `cargo run -p nookery-cli -- ls --time`

## Measured (M0, 2026-11-08, M5 MacBook, release build)

- Open + replay 5 events 0.34 ms; 10,005 events 6.4 ms; listing 5,000 lots
  4.4 ms (mostly printing).
- Photos (M3, 2026-11-09, `qwen3-vl:8b-instruct`, user's 924x2000 JPEGs):
  fridge 14 s / 30 sightings (5 tiles, 2-4 s each), pantry 20 s (8 tiles);
  first scan after Ollama starts adds ~10-15 s model load.
- Recipes (M5, 2026-11-09, 17 lots): 4 ideas in ~10 s, full recipe ~8 s.

## Roadmap

1. **M0 (done)**: workspace, ledger, store, CLI, Tauri shell with
   Use first / Fridge / Freezer / Pantry and quick add.
2. **M1 receipts** (`crates/receipt`; text import done 2026-11-08 in
   CLI and app; next: real receipts, eBon PDF, OCR of paper). The user
   shops at REWE, EDEKA, Netto and denn's Biomarkt. Paper photos (OCR +
   per-store profiles: line format, weights `0,512 kg x 2,99`, Pfand,
   discounts, multi-buy) and digital receipts (REWE eBon PDF; EDEKA/Netto
   app receipts if the user has them: ask). Per-store alias table learned
   from every confirmation. Line sum must equal the receipt total. Real
   receipts go in git-ignored `fixtures/private/`.
3. **M2 single items**: barcode (`rxing`) + offline Open Food Facts in
   SQLite; OCR (`ocrs`) for best-before dates.
4. **M3 photos of fridge/freezer/pantry** (first version done 2026-11-09:
   CLI + app; next: freezer photo, benchmark `qwen3-vl:4b-instruct`, crops
   per sighting, HEIC input): vision model behind a
   `VisionModel` trait (Ollama `qwen3-vl:8b-instruct` first;
   later in-process via mistral.rs/Metal). Reconcile against the ledger:
   matched (refresh), expected-but-not-seen (ask), seen-but-never-bought
   (add as `Source::Photo`). Never double-count a receipt line and a photo.
5. **M4 shelf life**: USDA FoodKeeper (public domain) for missing dates,
   opened and freezer windows.
6. **M5 recipes** from stock, scored by use-it-up value and what's missing
   (first version 2026-11-09: CLI + Recipes tab + templates; next: save
   favourites, partial "used some" after cooking).
7. **M6 weekly balanced plan** (`good_lp` + HiGHS; nutrition from USDA FDC
   and Open Food Facts; the model writes steps only, never nutrition numbers)
   and a shopping list.
8. **M7 phone app** (Tauri mobile): camera, barcode/OCR on the phone, hard
   cases sent to the Mac over the home network (finopsis's paired, encrypted
   lan-sync pattern).

## Environment notes

- Ollama on this Mac: standalone app in /Applications (not Homebrew),
  updated 0.13.5 -> 0.40.15 on 2026-11-08 (0.13.5 was too old for
  `qwen3-vl`: registry answered 412). Pulled: `qwen3-vl:8b-instruct` (used)
  and `qwen3-vl:8b` (thinking edition, too slow; can be removed). When
  swapping the app, make sure the old process has quit: the stale 0.13.5
  app's updater shut the server down mid-request once.
- Installed app: `pnpm tauri build --bundles app`, then
  `ditto target/release/bundle/macos/Nookery.app /Applications/Nookery.app`.
  Unsigned local build; it uses the real data folder
  (`~/Library/Application Support/dev.nookery.app`).
