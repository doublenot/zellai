Title: Show ports and PR/CI info in detailed sidebar cards
Files: src/sidebar.rs, src/status_bar.rs
Issue: none

## Goal

Render the `ports` and `pr_number`/`pr_ci_status` fields in the detailed sidebar
cards. These fields are already defined in the `AgentStatus` struct and parsed by
the status bridge, but the sidebar never displays them. This is gaps #2 and #3
from the assessment (display side).

## Implementation Details

### sidebar.rs changes — detailed card expansion

Currently `render_detailed_card()` produces 3 lines:
1. `icon name — status`
2. `branch ● working_dir`
3. `last_message`

Expand to 4 lines when there's extra metadata to show:

1. `icon name — status` (unchanged)
2. `branch ● working_dir` (unchanged)
3. **NEW: ports + PR/CI info line** (only when data exists)
4. `last_message` (was line 3)

Line 3 format examples:
- Ports only: `   🔌 :3000 :5173`
- PR only: `   PR #42 ✓ passing`
- Both: `   🔌 :3000 | PR #42 ✓ passing`
- Neither: **skip this line entirely** (card stays 3 lines)

For PR/CI status, use icons:
- passing → `✓` (in green if ANSI colors from task_01 are present, but don't depend on it)
- failing → `✗` (in red)
- pending → `⏳`

**Update `DETAILED_ROWS` to be dynamic.** Currently it's a constant `3`. The
density calculation uses this to determine if detailed cards fit. Options:
- **Option A (recommended):** Keep `DETAILED_ROWS = 3` as the baseline. When a
  card has extra metadata, it uses 4 rows. Adjust the density chooser to account
  for this by passing a function or pre-computing the actual row count per agent.
- **Option B (simpler):** Change `DETAILED_ROWS = 4` unconditionally. Wastes one
  row per card when there's no metadata, but simpler code.

  **Go with Option B** — it's simpler and the extra blank line provides visual
  breathing room between cards anyway. If the extra line exists but has no
  ports/PR data, render it as a thin separator: `   ─────` (dim line within the card).

Actually, **go with a cleaner approach**: Change `DETAILED_ROWS` to 4. Line 3 is
always rendered. If no ports and no PR info, show a dim separator or just leave
it as a blank padded line within the card. This keeps the density math simple.

### Implementation steps

1. **Change `DETAILED_ROWS` from 3 to 4.**

2. **Update `render_detailed_card()`** to produce 4 lines:
   - Line 1: icon + name — status (existing)
   - Line 2: branch ● working_dir (existing)
   - Line 3: ports + PR/CI metadata (new)
   - Line 4: last message (moved from line 3)

3. **Add a `render_metadata_line()` helper:**
   ```rust
   fn render_metadata_line(agent: &AgentStatus, inner_width: usize) -> String
   ```
   - If agent has ports: format as `:3000 :5173`
   - If agent has pr_number: format as `PR #42` + ci status icon
   - Join with ` | ` separator
   - If neither: return empty or dim line

4. **Add `ci_status_icon()` helper:**
   ```rust
   fn ci_status_icon(status: &CiStatus) -> &'static str
   ```
   Maps CiStatus variants to icons.

### status_bar.rs changes (minor)

5. No changes needed — the status bar already shows agent count and attention
   count, which is the right level of detail for a single-line segment.

### Testing

6. **Update existing detailed card tests** — they'll break because cards are now
   4 lines instead of 3. Update `assert_eq!(lines.len(), 3)` → `4`.

7. **Add new tests:**
   - Agent with ports → line 3 shows port numbers
   - Agent with pr_number + ci_status → line 3 shows PR info
   - Agent with both → line 3 shows both joined with ` | `
   - Agent with neither → line 3 is empty/separator
   - `ci_status_icon` maps correctly

8. **Update density tests** — `DETAILED_ROWS` changing from 3 to 4 will shift
   some density boundary tests. Update expected values.

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All tests must pass. Cards with metadata should display the info; cards without
should degrade gracefully to a blank/separator line.
