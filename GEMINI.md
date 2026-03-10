# AiTUI Project Context

AiTUI is a terminal-based AI chat tool written in Rust, featuring Vim-like keybindings and supporting multiple OpenAI-compatible API providers.

## Project Overview

- **Core Technologies**: Rust, `ratatui` (TUI), `tokio` (Async runtime), `reqwest` (HTTP/SSE), `serde` (Serialization).
- **Architecture**:
    - **App State Machine**: `src/app.rs` manages the central `App` struct, containing mode (Normal/Insert/Visual/Setup), conversation list, input buffer, and configuration.
    - **Event Loop**: `src/event.rs` and `src/main.rs` handle the asynchronous event loop, routing keyboard input, API tokens, and background tasks.
    - **UI Layer**: `src/ui.rs` uses `ratatui` for rendering. It implements a horizontal split with a sidebar for conversation lists and a main chat area.
    - **API Integration**: `src/api.rs` handles streaming requests via SSE and connection testing for the setup wizard.
    - **Persistence**: `src/history.rs` manages XDG-compliant storage for conversation history in JSON format.
    - **Keybindings**: `src/keymap.rs` provides a configurable, Vim-inspired input system.

## Building and Running

- **Development Environment**: Recommended to use `nix develop` to ensure all system dependencies (`openssl`, `pkg-config`, `xclip`) are available.
- **Run**: `cargo run` (or `nix develop -c cargo run`).
- **Build**: `cargo build --release`.
- **Configuration**:
    - Config file: `~/.config/aitui/config.toml`.
    - Keybindings: `~/.config/aitui/keybindings.toml`.
    - If no config exists, the app starts in **Setup Mode** to guide the user through provider configuration.

## Development Conventions

- **Vim Modes**: Strict adherence to Normal, Insert, and Visual modes.
    - `i`/`a`/`A`/`I`: Enter Insert mode.
    - `v`: Enter Visual mode.
    - `Esc`: Return to Normal mode.
- **Navigation**:
    - `j`/`k`: Navigate messages.
    - `Ctrl+j`/`Ctrl+k`: Switch between conversations.
- **Asynchronous Flow**: API requests are spawned as background tasks using `tokio::spawn`, communicating back to the UI thread via `mpsc` channels and custom `Event` enums.
- **UI State**: Use `App` methods (e.g., `conversation()`, `conversation_mut()`) to access the active state safely.
- **XDG Compliance**: Always use `dirs` or environment variables to locate configuration and data paths.

## Key Files

- `src/main.rs`: Entry point and terminal lifecycle management.
- `src/app.rs`: Main state logic and transition rules.
- `src/ui.rs`: Layout and widget rendering.
- `src/config.rs`: Configuration structure and persistence.
- `src/api.rs`: OpenAI-compatible API client implementation.
- `flake.nix`: Reproducible development environment definition.
