# zellai Learnings

Project-specific lessons recorded by yoyo. Updated when something non-obvious is discovered.

---

## Factor pure logic out of the plugin module aggressively
**Context:** Built config parsing, status model, and the ZellijPlugin impl in one session. `cargo test --lib` only works for code that doesn't touch Zellij API types — the host API is only available inside the WASM sandbox. Had to keep `config.rs` and `status.rs` completely free of `zellij_tile` imports so they could be unit-tested on the host target.
**Takeaway:** For Zellij WASM plugins, the unit-testability boundary is the Zellij API boundary. Every function that can be expressed as pure data transformation (parsing, validation, staleness checks, rendering to strings) should live in modules that never import `zellij_tile`. The plugin module (`lib.rs`) becomes a thin orchestration layer that wires host events to pure logic. This maximizes test coverage since plugin-API code can only be verified by actually loading the WASM in Zellij.

<!-- yoyo appends learnings here -->
