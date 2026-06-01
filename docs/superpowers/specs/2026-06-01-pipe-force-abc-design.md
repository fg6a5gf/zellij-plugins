# Zellij Macism Plugin v2 — Pipe Message Force-ABC

**Date:** 2026-06-01
**Status:** Approved
**Goal:** 扩展现有 plugin,支持通过 `MessagePlugin` keybind 触发强制切 ABC 输入法,适用于 yazi/harpoon 等通过浮动 pane 启动的工具。

## Problem

现有 plugin 仅监听 `ModeUpdate`,只有 zellij 输入模式切换时(Ctrl+p/t/n 等)才切 ABC。但用户场景:

- `Ctrl+f` 启动 yazi(浮动 shell pane)
- `Ctrl+y` 启动 harpoon(浮动 plugin pane)

两种情况 zellij 输入模式仍是 Normal,`ModeUpdate` 不触发 → IM 不切换 → yazi/harpoon 单键操作被 CJK IM 拦截。

## Goal

按 `Ctrl+f`/`Ctrl+y` 时:
1. Plugin 收到通知 → query 当前 IM → save 到 `pane_im[shell_pane]` → 切 ABC
2. 用户从 yazi/harpoon 退出(pane 关闭)→ plugin 检测 `PaneClosed` → 恢复 shell pane 的 saved IM

## Non-Goals

- 自动检测哪些命令需要 ABC(如 vim、less)。仅响应明确配置的 keybind。
- 支持嵌套 force_abc(yazi 内启动新 floating pane)。第二次 force_abc 是 noop,设计可接受。

## Architecture

基于 v1 plugin 扩展。新增:

- `pipe()` lifecycle method 处理 `PipeMessage`(name=`force_abc`,source=`Keybind`)
- `Event::PaneClosed` handler 触发 IM 恢复
- `state::forced_abc_pane: Option<PaneId>` 记忆被强制切 ABC 的 pane id

不变: 现有 `ModeUpdate`/`PaneUpdate`/`RunCommandResult` 流程。

## State 变更

```rust
#[derive(Default)]
pub struct State {
    pub prev_class: Option<ModeClass>,
    pub focused_pane: Option<PaneId>,
    pub pane_im: HashMap<PaneId, String>,
    pub forced_abc_pane: Option<PaneId>,  // 新增
}
```

`forced_abc_pane` 语义: 上一次响应 `force_abc` 消息时被切 ABC 的 pane id。`None` 表示当前没有"待恢复"状态。

## Pure Logic 新增

### `decide_force_abc`

```rust
pub fn decide_force_abc(state: &mut State) -> Action {
    if state.forced_abc_pane.is_some() {
        return Action::Noop;  // 重入保护: 已 forced 状态,跳过
    }
    let pane = match state.focused_pane {
        Some(p) => p,
        None => return Action::Noop,
    };
    state.forced_abc_pane = Some(pane);
    Action::QueryThenSwitchAbc { pane }
}
```

复用 `Action::QueryThenSwitchAbc` 走现有 query+save+ABC 流程。

### `decide_pane_closed`

```rust
pub fn decide_pane_closed(state: &mut State, default_cjk: &str) -> Action {
    let pane = match state.forced_abc_pane.take() {  // take 清空状态
        Some(p) => p,
        None => return Action::Noop,
    };
    let target = state
        .pane_im
        .get(&pane)
        .cloned()
        .unwrap_or_else(|| default_cjk.to_string());
    Action::Restore { pane, target }
}
```

复用 `Action::Restore` 走现有 IM 切换流程。

## Event Handlers (main.rs)

### `pipe()` 方法

```rust
fn pipe(&mut self, msg: PipeMessage) -> bool {
    if matches!(msg.source, PipeSource::Keybind) && msg.name == "force_abc" {
        let action = decide_force_abc(&mut self.state);
        self.execute(action);
    }
    false
}
```

注: `PipeSource::Keybind` 表示来自 `MessagePlugin` keybind action(非 CLI 也非其他 plugin)。`is_private`/`payload` 不检查。

### `Event::PaneClosed` handler

```rust
Event::PaneClosed(_pane_id) => {
    let action = decide_pane_closed(&mut self.state, &self.config.default_cjk);
    self.execute(action);
}
```

注: 不依赖 `_pane_id`(被关 pane 是 yazi/harpoon,不是要恢复的 shell pane)。完全靠 `forced_abc_pane` 状态决定。

### Subscribe 加

```rust
subscribe(&[
    EventType::ModeUpdate,
    EventType::PaneUpdate,
    EventType::RunCommandResult,
    EventType::PaneClosed,  // 新增
]);
```

`pipe()` 不需要 subscribe(永远调)。

## config.kdl Keybind 改动

现有:

```kdl
bind "Ctrl f" {
    Run "yazi" { floating true; close_on_exit true; }
}
bind "Ctrl y" {
    LaunchOrFocusPlugin "file:~/.config/zellij/plugins/harpoon.wasm" {
        floating true; move_to_focused_tab true;
    }
}
```

改为:

```kdl
bind "Ctrl f" {
    MessagePlugin "file:~/.config/zellij/plugins/zellij_macism.wasm" {
        name "force_abc"
    }
    Run "yazi" { floating true; close_on_exit true; }
}
bind "Ctrl y" {
    MessagePlugin "file:~/.config/zellij/plugins/zellij_macism.wasm" {
        name "force_abc"
    }
    LaunchOrFocusPlugin "file:~/.config/zellij/plugins/harpoon.wasm" {
        floating true; move_to_focused_tab true;
    }
}
```

两个 action 顺序执行: 先发消息切 ABC,再启动工具。

## Edge Cases

| 场景 | 行为 |
|------|------|
| 重复按 Ctrl+f(yazi 已开) | `forced_abc_pane.is_some()` → `decide_force_abc` 返回 Noop。无副作用 |
| 重复按 Ctrl+y(harpoon focus 已存在) | 同上 |
| force_abc 期间用户按 Ctrl+p(进入 Pane mode) | `ModeUpdate` 触发 `decide(Cjk→Abc)`,但 IM 已是 ABC。Query+save 把 ABC 写入 pane_im(覆盖原 CJK)。**Bug 风险** |
| force_abc 期间用户切 pane(Alt+h) | `focused_pane` 变,`forced_abc_pane` 不动。关 yazi 时仍恢复原 pane,语义正确 |
| yazi 关闭时不调 PaneClosed(不应该,但理论上) | `forced_abc_pane` 永不清,下次 force_abc 走 noop 路径。需要手动 `Ctrl+g` Locked → Normal 触发 mode-driven 恢复(也不会触发,因为 prev_class 仍为 Cjk)。**回退方案: 任何 ModeUpdate 进入 Cjk 模式时,一并清 forced_abc_pane** |
| Plugin 在 force_abc 状态崩溃重启 | 状态丢失,IM 留在 ABC。用户手动切回 |

### Bug 修复: forced 状态下跳过 query

场景 3(force_abc 期间用户按 Ctrl+p 进入 Pane mode):
- `ModeUpdate` 触发 `decide(Cjk→Abc)`
- 但 IM 已是 ABC(force_abc 已切过)
- 若执行 query+save → ABC 写入 `pane_im[shell_pane]`,污染 saved IM
- 关 yazi 时恢复成 ABC,语义错误

**修复**: `decide()` 在 Cjk→Abc 路径检查 `forced_abc_pane.is_some()`,有值时返回 Noop:

```rust
pub fn decide(state: &mut State, new_class: ModeClass, default_cjk: &str) -> Action {
    let pane = match state.focused_pane {
        Some(p) => p,
        None => return Action::Noop,
    };

    if state.prev_class == Some(new_class) {
        return Action::Noop;
    }

    state.prev_class = Some(new_class);

    match new_class {
        ModeClass::Abc => {
            if state.forced_abc_pane.is_some() {
                return Action::Noop;  // 已 forced,IM 已是 ABC,跳过避免污染 pane_im
            }
            Action::QueryThenSwitchAbc { pane }
        }
        ModeClass::Cjk => {
            let target = state
                .pane_im
                .get(&pane)
                .cloned()
                .unwrap_or_else(|| default_cjk.to_string());
            Action::Restore { pane, target }
        }
    }
}
```

## Tests

新增 6 个 test 到 `tests/state_tests.rs`:

1. `force_abc_saves_pane_and_emits_query` — focus=5, decide_force_abc → forced=Some(5), Action::QueryThenSwitchAbc{5}
2. `force_abc_no_focus_noop` — focus=None → Action::Noop, forced 不变
3. `force_abc_already_forced_is_noop` — forced=Some(3), decide_force_abc → Noop, forced 不变
4. `pane_closed_restores_forced_with_saved_im` — forced=Some(5), pane_im[5]="X" → Action::Restore{5,"X"}, forced=None
5. `pane_closed_falls_back_to_default_when_no_saved` — forced=Some(5), pane_im 空 → Restore{5, default_cjk}, forced=None
6. `pane_closed_no_forced_is_noop` — forced=None → Noop
7. `decide_cjk_to_abc_skips_query_when_forced` — forced=Some(5), prev=Cjk, new=Abc → Noop(避免污染 pane_im)

## Manual Test Plan

1. 重新构建:
   ```bash
   cargo build --release --target wasm32-wasip1 --features wasm-plugin
   cp target/wasm32-wasip1/release/zellij-macism.wasm ~/.config/zellij/plugins/zellij_macism.wasm
   ```
2. 改 `~/.config/zellij/config.kdl` 中 `Ctrl f`/`Ctrl y` 加 `MessagePlugin` action
3. `zellij kill-all-sessions && zellij attach --create main`
4. Shell pane 切到 Squirrel(`macism im.rime.inputmethod.Squirrel.Hans`)
5. 按 `Ctrl+f` 启动 yazi → 验证 `macism` 输出 ABC
6. yazi 内按 `q` 退出 → 验证 IM 恢复 Squirrel
7. 按 `Ctrl+y` 启动 harpoon → 验证 ABC,关 harpoon 验证恢复
8. 多 pane 测试: pane A 设 Squirrel,pane B 设 Sogou。在 A 按 Ctrl+f,关 yazi 验证 A 恢复 Squirrel。切到 B 按 Ctrl+f,关 yazi 验证 B 恢复 Sogou。

## Open Questions

无。所有 API 字段已查证(`PipeMessage`, `PipeSource::Keybind`, `Event::PaneClosed(PaneId)`, `MessagePlugin` keybind action)。
