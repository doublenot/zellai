Title: Regenerate screenshot and fix stale README text
Files: README.md, docs/screenshot.svg
Issue: none (priority task — manual dispatch)

## Context

The priority task for this session is: "Attempt to take a screenshot via `zellij action dump-screen`."

Zellij is not available in the CI environment, so `zellij action dump-screen` cannot be used directly. However, the project already has a CI-compatible screenshot pipeline:

1. `cargo run --bin screenshot` — renders sample agent data through the sidebar/status_bar functions
2. `python3 docs/screenshot.py` — captures that output and renders it as a styled SVG via the `rich` library

The SVG is already present at `docs/screenshot.svg` and appears current, but we should regenerate it to ensure it reflects any rendering changes and commit it if updated.

Additionally, `README.md` line 23 still says "Coming soon — yoyo will scaffold the plugin in its first growth session." This is outdated — the plugin has been fully built for many sessions. Fix this.

## Steps

1. Run `python3 docs/screenshot.py` to regenerate `docs/screenshot.svg`
2. Check `git diff docs/screenshot.svg` to see if anything changed
3. Edit `README.md` to remove the stale "Coming soon" line. Replace it with a brief description like "Clone and build:" since the build instructions follow immediately after.
4. Verify: `cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib`
5. Commit: `git add README.md docs/screenshot.svg && git commit -m "yoyo: regenerate screenshot, fix stale README text"`
