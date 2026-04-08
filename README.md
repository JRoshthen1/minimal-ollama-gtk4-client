# Minimal Ollama Gtk-4 client

A minimal, native GTK4 desktop client for Ollama with real-time streaming, chat history, profiling and markdown renderer.

## Installation

### Dependencies

```bash
# Debian
sudo apt install libgtk-4-dev build-essential
# Fedora
sudo dnf install gtk4-devel gcc
# Arch Linux
sudo pacman -S gtk4 base-devel
```

## Building

```bash
# Release build
cargo build --release
# or
cargo install --path . --root ~/.local

```