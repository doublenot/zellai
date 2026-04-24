Title: Generate plugin interface screenshot and add to README.md
Files: src/bin/screenshot.rs, docs/screenshot.py, README.md, Cargo.toml (if needed)
Issue: none (manual dispatch)

## Description

The manually dispatched priority task: take a screenshot of the plugin interface and add it to README.md at the top.

Since the plugin runs as WASM inside Zellij and we can't launch Zellij in CI, the approach is:

1. **Write a Rust binary `src/bin/screenshot.rs`** that:
   - Creates sample `AgentStatus` data (4-5 agents in various states: thinking, waiting with attention, idle, error)
   - Calls `render_sidebar()` with a realistic terminal size (e.g., 30 cols × 24 rows)
   - Calls `render_status_bar()` with the same agents
   - Prints the combined output to stdout
   - This binary compiles for the host target (not WASM), using only the pure-logic modules from `lib.rs`

2. **Write a Python script `docs/screenshot.py`** that:
   - Runs `cargo run --bin screenshot` to capture the text output
   - Uses `rich` (already installed) to render it with terminal colors into a styled console
   - Exports as SVG using `rich.console.Console.export_svg()` — this produces a clean terminal-styled image
   - Saves to `docs/screenshot.svg`

3. **Update README.md** to include the screenshot at the top, right after the title/tagline:
   ```markdown
   ![zellai sidebar screenshot](docs/screenshot.svg)
   ```

### Sample agent data to use:
- `claude-backend` — `Thinking`, branch `feat/api-v2`, dirty, message "Using Edit…"
- `codex-frontend` — `Waiting`, branch `fix/login-ui`, needs_attention=true, message "Needs input on auth flow"
- `aider-tests` — `Running`, branch `main`, message "Running test suite…"
- `claude-docs` — `Idle`, branch `docs/readme`, no message
- `gemini-refactor` — `Error`, branch `refactor/db`, needs_attention=true, message "Build failed"

### Implementation notes:
- The `screenshot` binary should NOT be compiled for `wasm32-wasip1` — add it as `[[bin]]` in Cargo.toml and it will only be built for host target
- Import types from `zellai` lib crate: `use zellai::{sidebar, status_bar, config, status}`
- The lib crate's public API needs to re-export the rendering functions and types. Check `src/lib.rs` to see what's currently pub.
- If modules aren't public, make them `pub` in `lib.rs` (they contain no WASM-specific code)

### Verification:
```bash
cargo run --bin screenshot          # should print sidebar text
python3 docs/screenshot.py          # should generate docs/screenshot.svg
cargo build --target wasm32-wasip1  # main plugin still builds
cargo test --lib                    # all tests pass
cargo clippy --target wasm32-wasip1 # no warnings
```

After generating: `git add docs/screenshot.svg README.md && git commit`
