# Minimal Ollama Gtk-4 client

A minimal, native GTK4 desktop client for Ollama with real-time streaming and markdown support.

## Installation

### Dependencies

#### Ubuntu/Debian
```bash
sudo apt install libgtk-4-dev build-essential
```

#### Fedora
```bash
sudo dnf install gtk4-devel gcc
```

#### Arch Linux
```bash
sudo pacman -S gtk4 base-devel
```

## Configuration

Configuration file is automatically created at:
- Linux: `~/.config/ollama-chat-gtk4/config.toml`
- Windows: `%APPDATA%\ollama-chat-gtk4\config.toml`
- macOS: `~/Library/Application Support/ollama-chat-gtk4/config.toml`

## Building

```bash
# Release build
cargo build --release
# or
cargo install --path . --root ~/.local

```

### Configuration Options

```toml
[ui]
window_font_size = 16
chat_font_size = 18
input_font_size = 16
code_font_family = "BlexMono Nerd Font Mono"

[colors]
chat_background = "#ffffff"
code_background = "#f5f5f5"
window_background = "#fafafa"
primary_text = "#333333"
code_text = "#d63384"
link_text = "#0066cc"
think_text = "#6666cc"
send_button = "#007bff"
stop_button = "#dc3545"

[ollama]
url = "http://localhost:11434"
timeout_seconds = 120

[streaming]
batch_size = 20
batch_timeout_ms = 100
```

## Building

```bash
# Release build
cargo build --release
# or
cargo install --path . --root ~/.local

```

MIT licence