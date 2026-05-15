# zellij-macism

Zellij plugin that auto-switches macOS input method via [macism](https://github.com/laishulu/macism) based on Zellij input mode. Per-pane CJK IM memory.

## Behavior

- **Normal / Locked mode** → restores the CJK IM the pane last used (or `default_cjk` if none).
- **Any other mode** (Pane / Tab / Resize / Move / Scroll / Session / Rename* / Tmux / Search) → switches to ABC (English).

## Requirements

- macOS
- [macism](https://github.com/laishulu/macism) installed and on PATH (or supply absolute path via `macism_path` config).
- Zellij with plugin permission support.

## Build

```bash
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1
```

Output: `target/wasm32-wasip1/release/zellij_macism.wasm`.

## Install

```bash
mkdir -p ~/.config/zellij/plugins
cp target/wasm32-wasip1/release/zellij_macism.wasm ~/.config/zellij/plugins/
```

## Configure

Add to `~/.config/zellij/config.kdl`:

```kdl
load_plugins {
    "file:~/.config/zellij/plugins/zellij_macism.wasm" {
        default_cjk "im.rime.inputmethod.Squirrel.Hans"
        abc "com.apple.keylayout.ABC"
        macism_path "/opt/homebrew/bin/macism"
    }
}
```

All three keys are optional; defaults shown above.

## Permissions

On first load, Zellij will prompt for `ReadApplicationState` and `RunCommands`. Approve both.

## Test

```bash
cargo test
```
