# Zellij Setup

Terminal: iTerm2 + Zellij

## Auto-attach (bash)

```bash
if [[ -z "$ZELLIJ" ]]; then
  exec zellij attach --create main
fi
```

Add to `~/.bashrc` or `~/.bash_profile`. Auto-attaches to session `main`; creates if not exists.
`cmd+t` in iTerm2 opens new tab → attaches to same `main` session (not independent).

---

## Plugins

| Plugin | Type | 触发 |
|--------|------|------|
| harpoon.wasm | 第三方 | `Ctrl+y` |
| tab-bar | 内置 | 顶部常驻 |
| status-bar | 内置 | 底部常驻 |
| session-manager | 内置 | `Ctrl+o` → `w` |
| plugin-manager | 内置 | `Ctrl+o` → `p` |
| layout-manager | 内置 | `Ctrl+o` → `l` |
| configuration | 内置 | `Ctrl+o` → `c` |
| about | 内置 | `Ctrl+o` → `a` |
| share | 内置 | `Ctrl+o` → `s` |

---

## Key Bindings (`clear-defaults=true`)

### 全局（任意模式）

| 快捷键 | 动作 |
|--------|------|
| `Ctrl+f` | 悬浮窗打开 yazi（退出自动关闭） |
| `Ctrl+y` | 打开 harpoon（悬浮） |
| `Ctrl+g` | 切换 locked 模式 |
| `Ctrl+p` | 进入 pane 模式 |
| `Ctrl+t` | 进入 tab 模式 |
| `Ctrl+n` | 进入 resize 模式 |
| `Ctrl+h` | 进入 move 模式 |
| `Ctrl+s` | 进入 scroll 模式 |
| `Ctrl+o` | 进入 session 模式 |
| `Ctrl+b` | 进入 tmux 模式 |
| `Ctrl+q` | 退出 zellij |
| `Alt+h/j/k/l` | 切换 pane/tab（无需进模式） |
| `Alt+f` | 切换悬浮 pane |
| `Alt+[/]` | 切换 swap layout |
| `Alt+n` | 新建 pane |
| `Alt+i/o` | 移动 tab 左/右 |
| `Alt+p` | TogglePaneInGroup |
| `Alt+Shift+p` | ToggleGroupMarking |
| `Alt+=/+/-` | 调整 pane 大小 |

### Pane 模式（`Ctrl+p`）

| 快捷键 | 动作 |
|--------|------|
| `h/j/k/l` | 切换 pane |
| `d` | 向下分割 |
| `r` | 向右分割 |
| `s` | stacked pane |
| `w` | 切换悬浮 pane |
| `e` | 悬浮/嵌入切换 |
| `f` | 全屏 |
| `z` | 切换 pane 边框显示 |
| `i` | 固定 pane（pinned） |
| `c` | 重命名 pane |
| `x` | 关闭 pane |
| `n` | 新建 pane |

### Tab 模式（`Ctrl+t`）

| 快捷键 | 动作 |
|--------|------|
| `1-9` | 跳转到指定 tab |
| `h/k` | 上一个 tab |
| `j/l` | 下一个 tab |
| `n` | 新建 tab |
| `x` | 关闭 tab |
| `r` | 重命名 tab |
| `[` | pane 移到左边 tab |
| `]` | pane 移到右边 tab |
| `b` | break pane |

### Scroll 模式（`Ctrl+s`）

| 快捷键 | 动作 |
|--------|------|
| `j/k` | 上下滚动 |
| `h/l` | 翻页 |
| `u/d` | 半页滚动 |
| `e` | 编辑器打开 scrollback |
| `s` | 搜索 |

### Session 模式（`Ctrl+o`）

| 快捷键 | 动作 |
|--------|------|
| `w` | session manager |
| `d` | detach |
| `p` | plugin manager |
| `l` | layout manager |
| `c` | configuration |
| `a` | about |
| `s` | share |

---

## Layout

默认布局（`default.kdl`）：顶部 tab-bar + 底部 status-bar，均 borderless。
