# Zellij Macism Plugin — Design

**Date:** 2026-05-15
**Status:** Approved
**Goal:** Auto-switch macOS input method based on Zellij input mode, with per-pane CJK IM memory.

## Problem

When Zellij is in command modes (Pane/Tab/Resize/Move/Scroll/Session/Rename*/Tmux), single-key shortcuts conflict with CJK IMEs. User must manually toggle input method. Forgetting causes mis-typed commands.

User wants:
- In **Normal** or **Locked** mode → restore the CJK IM the user had active in that pane.
- In any other mode → force English (`com.apple.keylayout.ABC`).
- Per-pane memory: each pane remembers its own preferred CJK IM.
- Fall back to a configured default CJK IM if no record exists.
- Fall back to ABC if any switch fails.

## Non-Goals

- Tracking IM changes triggered outside Zellij mode transitions (e.g., user manually switching IM via Cmd+Space mid-session). The plugin only captures IM at the moment of leaving a CJK mode.
- Pane focus tracking. Switching focused pane within the same mode does not trigger an IM switch.
- Cross-platform support. macOS only (uses `macism`).

## Architecture

Single-file Rust plugin (`src/lib.rs`) compiled to `wasm32-wasip1`. Loaded as a background plugin in `~/.config/zellij/config.kdl`. No UI pane.

**Permissions:**
- `ReadApplicationState` — required for `ModeUpdate` events.
- `RunCommands` — required to invoke `macism`.

**External dependency:** `macism` binary (https://github.com/laishulu/macism) on `PATH` (or absolute path via config).

## State

```rust
struct MacismPlugin {
    prev_mode_class: Option<ModeClass>,
    focused_pane: Option<PaneId>,
    pane_im: HashMap<PaneId, String>,
    default_cjk: String,
    abc_id: String,
    macism_path: String,
    pending: Option<PendingOp>,
}

enum ModeClass { Cjk, Abc }   // Cjk = Normal|Locked, Abc = everything else

enum PendingOp {
    QueryThenSetAbc { pane: PaneId },
    Restore        { pane: PaneId },
}
```

`ModeClass` collapses Zellij's many input modes into the two classes the plugin cares about. Avoids re-checking the full enum on every event.

## Event Flow

### Subscribed Events

- `EventType::ModeUpdate`
- `EventType::PaneUpdate` — used **only** to keep `focused_pane` current so saves attribute to the right pane. Never triggers an IM switch.
- `EventType::RunCommandResult`

### `PaneUpdate`

Scan payload for the focused pane id, update `focused_pane`. No IM action.

### `ModeUpdate(info)`

1. Compute `new_class = classify(info.mode)` where Normal/Locked → `Cjk`, all others → `Abc`.
2. If `Some(new_class) == prev_mode_class`, no-op.
3. If transitioning **Cjk → Abc** (leaving a CJK mode):
   - `pending = Some(QueryThenSetAbc { pane: focused_pane })`
   - `run_command([macism_path], ctx={"op": "query", "pane": <id>})`
4. If transitioning **Abc → Cjk** (or initial load into Cjk):
   - target = `pane_im.get(focused).cloned().unwrap_or(default_cjk.clone())`
   - `pending = Some(Restore { pane: focused_pane })`
   - `run_command([macism_path, target], ctx={"op": "restore"})`
5. `prev_mode_class = Some(new_class)`.

### `RunCommandResult { exit_code, stdout, stderr, context }`

Match on `context["op"]`:

- **`query`**:
  - If `exit_code == Some(0)` and stdout non-empty: `pane_im.insert(ctx_pane, stdout.trim().to_string())`.
  - Always follow up with `run_command([macism_path, abc_id])` (no context — fire-and-forget).
- **`restore`**:
  - If `exit_code != Some(0)`: fire fallback `run_command([macism_path, abc_id])`.
  - Else: no-op.

Stale results (pane changed since op was issued) are accepted — saving an IM for a pane the user has navigated away from is harmless; restoring is also fine since the next mode change will overwrite.

## Config (kdl plugin args)

```kdl
load_plugins {
    "file:~/.config/zellij/plugins/zellij-macism.wasm" {
        default_cjk "im.rime.inputmethod.Squirrel.Hans"
        abc         "com.apple.keylayout.ABC"
        macism_path "macism"
    }
}
```

All three args optional. Defaults baked into plugin:
- `default_cjk` → `"im.rime.inputmethod.Squirrel.Hans"`
- `abc` → `"com.apple.keylayout.ABC"`
- `macism_path` → `"macism"`

## Error Handling

| Failure | Behavior |
|---------|----------|
| `macism` not on PATH | `RunCommandResult.exit_code = Some(127)` (or stderr message). Plugin logs once, continues. No retry. |
| `macism` query returns empty stdout | Skip pane_im update. Still switch to ABC. |
| Restore command fails | Fire fallback ABC switch. Pane's saved IM left intact (next entry will retry). |
| Permission denied | `request_permission()` on `load`. Zellij prompts user. If user denies, plugin is inert. |
| Mode toggles faster than `RunCommandResult` returns | Latest `pending` wins; older results may set IM out of order, but the next mode change re-asserts correct state within ~1 cycle. |

## Testing

**Unit tests** — pure decision function:

```rust
fn decide(prev: Option<ModeClass>, new: ModeClass, focused: PaneId, saved: Option<&String>)
    -> Option<Action>
```

Action enum: `QueryAndSwitchAbc { pane }`, `RestoreCjk { pane, target }`, `Noop`.

Cover:
- First mode update (prev = None) → emits action matching new class.
- Same-class transition → Noop.
- Cjk→Abc → QueryAndSwitchAbc.
- Abc→Cjk with saved IM → RestoreCjk{target=saved}.
- Abc→Cjk without saved IM → RestoreCjk{target=default}.

**Manual integration:**

1. `cargo build --release --target wasm32-wasip1`
2. Copy `target/wasm32-wasip1/release/zellij_macism.wasm` to `~/.config/zellij/plugins/`
3. Add to `config.kdl` `load_plugins`.
4. Restart zellij session.
5. Set IM to Squirrel Hans manually. Press `Ctrl+p` → verify IM switched to ABC. Press `Esc` → verify Squirrel restored.
6. Open second pane, set IM to a different CJK (e.g. Sogou). Cycle modes in both panes; verify each pane restores its own IM.

## Build & Distribution

- Build: `cargo build --release --target wasm32-wasip1`
- Output: `target/wasm32-wasip1/release/zellij_macism.wasm`
- Install: copy to `~/.config/zellij/plugins/zellij-macism.wasm`
- No CI, no publishing. Local use.

## Open Questions / Future Work

- If user wants pane-focus-triggered restore later, add `PaneUpdate` handler that compares previous focused pane and fires restore on switch within Cjk mode.
- If `macism` window-focus-loss bug is intolerable, swap binary for `ims-mac` via `macism_path` config.
