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
  Migration 5 (sync between the Mac and the phone): every event has a
  stable `uid` (new rows UUID v7, older rows random hex) so devices can
  tell which events they share: `synced_events_after(seq)`,
  `merge_events(&[(uid, Event)])` (each judged alone: known uid =
  duplicate, invalid against this ledger = rejected with a reason, the
  rest appended; one transaction), `replace_ledger` (the phone takes the
  Mac's copy; refuses one that doesn't replay). `docs(kind, id, body JSON,
  updated_at, deleted)` holds small synced documents that are not
  inventory (`DOC_FAVOURITE`, `DOC_PLAN`, `DOC_SHOPPING`): `put_doc`,
  `delete_doc` (a tombstone, so deletions sync), `docs_changed_since`,
  `merge_docs` (last writer wins; same instant: deletion, then larger
  body). Templates and settings stay per device.
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
- `crates/plan` (M5 extras + M6): weekly plan and shopping, pure and
  C-free (`--no-default-features` = the planner without SQLite; default
  feature `store` adds `ops`, the store-backed operations shared by the
  app and the CLI). `plan_week` picks dinners with `good_lp` + `microlp`
  (pure Rust, builds for Android): binary x per idea, y per use-soon lot
  (each lot counts once); maximise score + 4 per lot covered - needs;
  exactly `days` dinners, at most 2 sharing a main protein category, at
  least ceil(days/2) with a vegetable (relaxed in that order if
  infeasible); most urgent first; the rest are `spares` for `swap_day`.
  "Balanced" = food categories only, no nutrition numbers yet.
  Favourites, plans (id = the Monday) and shopping items are `docs` (they
  sync), never tables. `cook` writes one batch of `Used { qty }`
  (`BadInput("amount:<i>" | "lot:<i>")`) and bumps the favourite's count;
  `put_away` turns ticked shopping items into lots (fridge for chilled
  categories, else pantry).
- `crates/sync` (M7): encrypted Mac <-> phone sync over the home network,
  adapted from finopsis's lan-sync. The Mac app is the hub (port 47480,
  listening only while `sync.sharing` is on; off by default); the phone is
  the client (same code, role `cfg!(mobile)`). A 128-bit secret typed as a
  code (`ABCD-EFGH-...`); per connection HKDF-SHA256 keys from the secret
  and two nonces, XChaCha20-Poly1305 frames with a counter in the AAD (no
  replay or reorder). Requests: `Hello`, `Sync` (the phone's own events
  since `sync.pushed_seq` + docs since `sync.docs_since`; the Mac
  `merge_events` each, rejecting conflicts, and returns its whole ledger +
  docs; the phone `replace_ledger`s, so it always equals the Mac, and
  lists which of its changes were dropped; `prepare`/`finish` retry if the
  phone's ledger moved mid-sync), `ModelInfo`/`ModelChunk` (serves
  `PHONE_MODELS` = `qwen3:4b-instruct` from the Mac's Ollama store;
  `download_model` resumes and checks sha256), `Generate` (the Mac's
  local Ollama only; `MacText` is a `TextModel`). Docs sync uses the Mac's
  clock (`server_time`); a badly skewed phone clock could lose an edit.
- `crates/llm` (M7): a GGUF model in the app's own process via llama.cpp
  (`llama-cpp-2`), behind feature `llama` (needs cmake + a C++ build: use
  the Android SDK's cmake 3.22.1 on PATH; desktop builds don't enable it).
  `LlamaText::load(path, LlamaOptions::default())` implements `TextModel`
  (the model's own chat template, no thinking; temperature 0.7, top-k 20,
  top-p 0.8, light repeat penalty; 4096 context; UTF-8-safe streaming;
  Metal on macOS, CPU on Android). Measured on the M5 with the
  `qwen3:4b-instruct` blob: load 0.2-5 s, 4 ideas 6.7 s (44 tok/s), recipe
  7.6 s. Estimated on the S25 (not measured): 10-15 tok/s, ~25-35 s for 4
  ideas. `source scripts/android-env.sh` before an Android build: it sets
  the NDK toolchain and `GGML_CPU_ARM_ARCH=armv8.6-a+dotprod+i8mm` (the
  S25 needs it for speed; changing it needs `cargo clean -p
  llama-cpp-sys-2`). Real run: `cargo test -p nookery-llm --features llama
  --release --test real_model -- --ignored --nocapture`.
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
  Layout (2026-11-09): `routes/+layout.svelte` has a banner (logo mark
  64 px, wordmark, tagline "Your kitchen, kept kindly", a parsley sprig,
  scalloped bottom edge) with Kitchen / Recipes / Plan / Shopping and the
  Settings gear. Under 640 px the banner is sticky and slims on scroll,
  the tabs move to `lib/TabBar.svelte` (bottom bar) and touch targets are
  >= 44 px (CSS in the `/* ---- phone ---- */` block of `layout.css`).
  `lib/platform.ts` (`isAndroid`, `html.android`), `lib/back.svelte.ts`
  (Android back closes menus/dialogs only while one is open: `pushBack`,
  `watchDialogs` for every native `<dialog>`).
  Recipe art: `lib/art/DishArt.svelte` (`{ title, categories, size?,
  hero?, decorative? }`) draws a procedural inline-SVG dish; `dish.ts`
  (pure, tested): `vesselFor(title)` (12 vessels from English/German title
  words), `toppingKinds(categories)`, `plan(title, categories)` seeded by
  FNV-1a -> mulberry32 of the title, so a recipe always looks the same.
  Colours from `--c-*` and the `/* ---- recipe art ---- */` tokens
  (`--art-*`); steam off under reduced motion. `KitchenScene`,
  `CookingLoader`: header, empty and loading illustrations.
  `lib/recipes/IdeaCard.svelte`, `RecipeSheet.svelte`; `FavouriteButton`,
  `CookedSheet` ("all / some / none" per lot, one `cooked` write),
  `Favourites` (opens a saved recipe without the model). The heart is
  tied to its favourite by id (`Cooking.favouriteId`, bound from
  `FavouriteButton`, falling back to the title), so when the recipe chat
  changes a saved recipe the heart shows "Save changes", which calls
  `update_favourite` (same id, reason, saved date and cooked count;
  `ops::update_favourite`). `routes/plan`
  (make my week, day cards with DishArt, swap, cooked, add what's missing
  to shopping) and `routes/shopping` (aisle groups, tick, clear, put away
  with Undo); shared state in `lib/plan/`.
  Android (`src-tauri/gen/android`, id `dev.nookery.app`):
  `scripts/android.sh build --target aarch64 --debug --apk` (defaults
  JAVA_HOME = Android Studio's JBR, the SDK and the newest NDK; sources
  `scripts/android-env.sh`). On mobile the database is in
  `app_data_dir()`, opened in `setup`; the bindings export is
  desktop-debug only (it crashed on Android's read-only filesystem).
  Android themes set the paper background (no white flash).
  Phone side (`src-tauri/src/phone.rs`): `Writer` = desktop `ChosenText`
  / on-phone `LlamaText` / `MacText`, chosen by `ai.phone_recipes` =
  auto | phone | mac (default auto: phone if the model is downloaded,
  else the Mac); `suggest_recipes`, `write_recipe`, `recipe_chat` and
  `make_plan` all pick their model through `Writer::for_state`. The model
  file is `<app data>/models/qwen3-4b-instruct.gguf`, fetched from the
  paired Mac (`download_model_cancellable`: resumable `.part`, sha256),
  loaded lazily in a blocking thread, calls serialised (6 threads,
  `ai.phone_threads`). Errors `PhoneModelMissing`, `MacUnreachable`.
  Photos on the phone go to the Mac (`Request::ScanPhoto`, resized to <=
  2000 px); the Mac scans with its own photo setting, the phone builds the
  review locally and syncs after applying. QR pairing: the Mac shows
  `nookery://pair?code=&addr=&name=` as an SVG made in Rust (`qrcode`),
  only while sharing is on; the phone scans it with
  `tauri-plugin-barcode-scanner` (mobile only; `capabilities/mobile.json`,
  `CAMERA`), parsed by `lib/phone/pairing.ts`. Auto-sync at start and on
  foreground (at most every 30 s, `lib/phone/autosync.svelte.ts`), with a
  quiet header badge. Emulator note: the AVDs have 2 GB RAM (the 2.5 GB
  model is killed on load; `-memory 6144` loads it but the emulated CPU
  gave no answer in 9.5 min): measure on the real S25 (`phone model:`
  line in logcat). The debug APK is ~680 MB (unstripped llama.cpp); use a
  release build for the phone: `scripts/android.sh build --target aarch64
  --apk`. Release signing reads git-ignored
  `src-tauri/gen/android/keystore.properties` (`storeFile`, `password`,
  `keyAlias`); the key is `~/.android/nookery-release.jks` (made
  2026-11-10, never commit it; updates to the installed app must be
  signed with the same key, or the phone needs an uninstall = data loss).
  Without the properties file the release APK is unsigned.
  Recipe chat (`crates/recipes/src/chat.rs`): `chat_prompt(recipe, stock,
  history, message)` (the current recipe, the stock use-soon first without
  expired, the last 6 turns); questions get a plain answer, changes a
  reply + `---` + the whole revised recipe; `parse_chat` -> `ChatAnswer {
  reply, recipe }` (a recipe only counts with ingredients and steps).
  Command `recipe_chat` (no suggestion id, so favourites work too).
  `lib/recipes/RecipeChat.svelte` under the recipe: quick chips, streamed
  replies, the revised recipe replaces `c.recipe` in place with a
  highlight of changed lines and Undo; state in `Cooking.chat`. Real check
  (2026-11-09, `qwen3-vl:8b-instruct`, real stock copy): 7-9 s a turn,
  "more filling for 3" added rice from stock and set serves 3:
  `cargo test -p nookery-recipes --release --test real_stock chat --
  --ignored --nocapture`.
  Quick add (`lib/kitchen/QuickAdd.svelte`) collapses only after a click
  elsewhere has finished: collapsing mid-click moved the list under the
  pointer and the click was lost.
- `README.md` is for end users (a day in the kitchen, privacy, getting
  started, FAQ). Its pictures come from `pnpm readme:media`
  (`scripts/readme-media/capture.mjs` + `assemble.py`, Pillow, no ffmpeg):
  Playwright drives the UI on its SYNTHETIC browser data (own Vite on
  5473 unless `README_MEDIA_URL`; `README_MEDIA_ONLY=kitchen,phone` for a
  subset) and writes GIFs and stills to `docs/media/` (never edit by
  hand). `.github/workflows/readme-media.yml` runs on every push,
  compares `capture.mjs --hash` (ui/src, ui/static, assets, the script,
  the lockfile) with `docs/media/.source-hash` and only then re-records
  and commits `docs: refresh README media [skip ci]`. When a screen's
  labels or flow change, check the selectors in `capture.mjs` (visible
  text: "Suggest", "Cook this", "Plan my week", "Put away", `label.seg`).

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
- Small models for the phone (2026-11-09, same real stock, Ollama on the
  Mac, `NOOKERY_TEXT_MODEL=<m>` + the real-stock test): `qwen3:1.7b` 4 s
  but unusable (lists the whole pantry as one recipe); `gemma3n:e2b` 5 s,
  good ideas; `qwen3:4b-instruct` 6 s ideas / 8.5 s recipe, the best
  recipe (picked for the phone, ~2.5 GB GGUF); `gemma3n:e4b` 7 s, odd
  recipe text ("1 available meat"); `qwen3-vl:8b-instruct` 9 s.

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
   (first version 2026-11-09: CLI + Recipes tab + templates; favourites,
   "used some" after cooking and per-recipe illustrations done the same
   day).
7. **M6 weekly balanced plan** (first version 2026-11-09: `good_lp` +
   `microlp`, balanced by food categories; plan + shopping list in CLI and
   app; next: nutrition from USDA FDC and Open Food Facts; the model
   writes steps only, never nutrition numbers).
8. **M7 phone app** (Tauri Android, first version 2026-11-09: runs on the
   emulator; encrypted sync with the Mac; on-phone recipes via llama.cpp;
   not yet on the user's S25): camera, barcode/OCR on the phone, hard
   cases sent to the Mac over the home network.

## Environment notes

- Ollama on this Mac: standalone app in /Applications (not Homebrew),
  updated 0.13.5 -> 0.40.15 on 2026-11-08 (0.13.5 was too old for
  `qwen3-vl`: registry answered 412). Also pulled 2026-11-09 for the
  phone benchmark: `qwen3:4b-instruct` (KEEP: served to the phone),
  `qwen3:1.7b`, `gemma3n:e2b`, `gemma3n:e4b` (removed 2026-11-10).
  `llama3.1:8b` predates this project; leave it.
  Pulled: `qwen3-vl:8b-instruct` (used)
  (`qwen3-vl:8b`, the thinking edition, was too slow and was removed). When
  swapping the app, make sure the old process has quit: the stale 0.13.5
  app's updater shut the server down mid-request once.
- Installed app: `pnpm tauri build --bundles app`, then
  `ditto target/release/bundle/macos/Nookery.app /Applications/Nookery.app`.
  Unsigned local build; it uses the real data folder
  (`~/Library/Application Support/dev.nookery.app`).
