Title: ANSI color support in sidebar and status bar rendering
Files: src/sidebar.rs, src/status_bar.rs
Issue: none

## Goal

Add ANSI terminal color codes to the sidebar and status bar output so agents are
visually differentiated by status. This is gap #10 from the assessment — the
sidebar currently renders plain monochrome text with Unicode icons but no color,
making it hard to scan at a glance.

## Color Scheme

Use standard ANSI 256-color or 16-color escape codes so they work in all
terminals. The color assignments (matching docs/screenshot.py):

| Element             | Color                    | ANSI Code          |
|---------------------|--------------------------|---------------------|
| Thinking status/icon| Green                    | `\x1b[32m`          |
| Waiting status/icon | Yellow (needs attention) | `\x1b[33m`          |
| Idle status/icon    | Dim/gray                 | `\x1b[2m` (dim)     |
| Error status/icon   | Red                      | `\x1b[31m`          |
| Branch name         | Cyan                     | `\x1b[36m`          |
| Box borders (╭╰│─)  | Dim                      | `\x1b[2m`          |
| Title "zellai"      | Bold                     | `\x1b[1m`          |
| Reset               |                          | `\x1b[0m`          |

## Implementation Details

### sidebar.rs changes

1. **Add a `status_color()` function** that maps `AgentStatusValue` → ANSI escape
   string (just the color code, no reset). Add a corresponding `RESET` constant.

2. **Modify `render_compact_card()`** to wrap the status icon and status text in
   the appropriate color. The icon and status word should both be colored. Example:
   `│ \x1b[32m◉\x1b[0m claude \x1b[36m[main]\x1b[0m \x1b[32mthinking\x1b[0m │`

3. **Modify `render_detailed_card()`** similarly:
   - Line 1: color the icon and status text
   - Line 2: color the branch name in cyan
   - Line 3: leave message as default color

4. **Modify `render_top_border()`** to render "zellai" in bold and borders dim.

5. **Modify `render_bottom_border()`** and `render_box_line()`** to render the
   `│` border chars in dim.

6. **CRITICAL: Width calculation must ignore ANSI escapes.** The `render_box_line`
   function pads content to fill width. ANSI escape codes are zero-width but have
   non-zero byte/char length. You MUST:
   - Add a `visible_char_count(s: &str)` helper that strips ANSI escapes before
     counting chars (strip sequences matching `\x1b\[[0-9;]*m`)
   - Update `render_box_line` to use `visible_char_count` instead of
     `content.chars().count()` for padding calculation
   - Update `truncate_with_ellipsis` to be ANSI-aware OR ensure truncation
     happens BEFORE color wrapping (preferred — simpler)

   **Recommended approach:** Truncate plain text first, THEN wrap in color codes.
   This keeps `truncate_with_ellipsis` simple and avoids splitting ANSI sequences.

### status_bar.rs changes

7. **Modify `render_status_bar()`** to:
   - Color the `⚠` attention count in yellow
   - Color the `⬡` icon in bold
   - Color agent count based on dominant status (optional — skip if complex)

### Testing

- Existing tests will break because they check exact string content. Update them
  to either:
  - Strip ANSI codes before asserting (preferred — add a `strip_ansi()` test helper)
  - Or update expected strings to include ANSI codes

- Add new tests verifying:
  - `status_color()` returns correct codes for each status
  - `visible_char_count()` correctly ignores ANSI escapes
  - Cards with ANSI colors still have correct visible width
  - Box lines with colored content are padded correctly

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All 133+ existing tests must pass (updated as needed). No new clippy warnings.
