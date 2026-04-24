Title: Add named wrapper binaries (zellai-codex, zellai-gemini, zellai-aider)
Files: src/bin/zellai/main.rs, src/bin/zellai/run.rs
Issue: none

## Context

The founding vision specifies named wrappers: `zellai-codex`, `zellai-gemini`, `zellai-aider`, `zellai-opencode` as standalone commands that are thin shims around `zellai run`. Currently only `zellai run <command>` exists.

The approach: add a `wrap` subcommand (or detect argv[0]) that acts as a named wrapper. Since creating separate binary crates for each agent would bloat the build, the standard pattern is:

1. Add a `wrap` subcommand to the CLI: `zellai wrap --agent codex -- codex <args>`
2. Document that users can create shell aliases or symlinks: `alias zellai-codex='zellai wrap --agent codex -- codex'`
3. OR detect the binary name from argv[0] — if the binary is invoked as `zellai-codex`, auto-set agent to codex

Option 3 is the cleanest UX but requires users to create symlinks. Let's implement **both**: argv[0] detection in `main.rs` + a `wrap` subcommand as fallback.

## Steps

1. In `src/bin/zellai/main.rs`, at the top of `main()`, check `std::env::args().next()` (argv[0]):
   - If it ends with `zellai-codex`, `zellai-claude`, `zellai-gemini`, `zellai-aider`, or `zellai-opencode`:
     - Extract the agent name from the suffix
     - Collect remaining args as the command
     - If no command args, use the agent name as the command (e.g., `zellai-codex` runs `codex`)
     - Call `run::run_with_agent(agent, command_args)` and exit
   - Otherwise, proceed with normal clap parsing

2. In `src/bin/zellai/run.rs`, refactor to expose a `run_with_agent(agent: &str, args: Vec<String>)` entry point that the named wrapper path can call. The existing `run()` function should delegate to this after parsing its own args.

3. Add a `wrap` subcommand to the clap CLI in `main.rs`:
   ```
   zellai wrap --agent codex -- codex --model o3
   ```
   This explicitly sets the agent kind and runs the trailing command.

4. Verify: `cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib`
5. Also verify the host binary builds: `cargo build && cargo clippy`
6. Commit: `git add -A && git commit -m "yoyo: add named wrapper support (argv[0] detection + wrap subcommand)"`

## Notes

- Don't create actual symlinks in the repo — that's a user/installer concern
- Document the symlink approach in a comment or help text
- Keep `zellai run <command>` working exactly as before — this is additive
- The `run.rs` refactor must not break existing tests
