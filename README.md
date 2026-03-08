# AiTUI

A terminal-based AI chat tool with Vim-like keybindings. Supports any OpenAI-compatible API (DeepSeek, OpenAI, Mimo, etc.) with streaming responses.

```
┌─ AiTUI - DeepSeek (deepseek-chat) ─────────┐
│ You:                                         │
│   请解释 Rust 的生命周期                       │
│                                              │
│ AI:                                          │
│   Rust 的生命周期是编译器用来追踪引用有效性的...  │
│                                              │
├──────────────────────────────────────────────┤
│ [INSERT] > _                                 │
└──────────────────────────────────────────────┘
```

## Features

- **Vim-style keybindings** - Normal / Insert / Visual modes
- **Multi-provider** - Configure multiple API providers, switch with `Tab`
- **Streaming responses** - Real-time token-by-token output via SSE
- **Conversation history** - Auto-saved as JSON in `~/.local/share/aitui/`
- **Message selection** - `j`/`k` to navigate between messages with visual highlight
- **Clipboard support** - `y` to copy the selected message
- **Editor viewing** - `Enter` to open selected message in `$EDITOR` for easy partial copy
- **Search** - `/` to search in conversation
- **Cross-platform** - XDG-compliant paths, works on Linux and macOS

## Installation

### From source

```bash
git clone https://github.com/Loyage/aitui.git
cd aitui
cargo build --release
# Binary at target/release/aitui
```

### With Nix

```bash
nix develop  # Enter dev shell
cargo run
```

## Configuration

Create `~/.config/aitui/config.toml`:

```toml
[[provider]]
name = "DeepSeek"
api_key = "sk-your-key"
base_url = "https://api.deepseek.com"
model = "deepseek-chat"
temperature = 1.0

[[provider]]
name = "OpenAI"
api_key = "sk-your-key"
base_url = "https://api.openai.com"
model = "gpt-4o"
temperature = 1.0
```

See [config.example.toml](config.example.toml) for all options including `proxy`, `max_tokens`, and `system_prompt`.

## Keybindings

### Normal Mode

| Key | Action |
|-----|--------|
| `i` | Enter Insert mode |
| `a` | Enter Insert mode (after cursor) |
| `j` / `k` | Select next / previous message |
| `G` | Select last message |
| `g` | Select first message |
| `y` | Copy selected message to clipboard |
| `Enter` | View selected message in `$EDITOR` |
| `n` | New conversation |
| `Tab` | Switch provider |
| `/` | Search in conversation |
| `q` | Quit |
| `Ctrl+C` | Force quit |

### Insert Mode

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Esc` | Back to Normal mode |
| `Ctrl+A` | Move cursor to start |
| `Ctrl+E` | Move cursor to end |
| `Ctrl+U` | Clear input |
| `Ctrl+W` | Delete word before cursor |

## License

MIT
