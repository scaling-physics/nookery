# Nookery

Intelligent kitchen inventory, entirely local. Track groceries, expiry dates
and freezer contents from receipts and photos, and get meals and balanced
weekly plans from what you already own.

```sh
cargo run -p nookery-cli -- add "greek yoghurt" -q 500g --bb 20.11. -s Lidl
cargo run -p nookery-cli -- ls --time
pnpm -C ui install && pnpm tauri dev
```

Rust core (`crates/`), Tauri 2 + SvelteKit UI (`ui/`). See `AGENTS.md` for
the design and roadmap.
