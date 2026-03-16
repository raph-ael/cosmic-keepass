# cosmic-keepass

A native KeePass password manager panel applet for the [COSMIC](https://system76.com/cosmic) desktop environment.

Quick access to your passwords directly from the panel — search, copy, and manage entries without leaving your workflow.

![COSMIC Panel Applet](https://img.shields.io/badge/COSMIC-Panel%20Applet-blue)
![License: MIT](https://img.shields.io/badge/License-MIT-green)

## Features

- **Panel applet** with lock/unlock icon
- **Master password unlock** with async decryption (non-blocking UI)
- **Search passwords** — results appear instantly as you type
- **One-click copy** — password or username to clipboard via `wl-copy`
- **Entry details** — view title, username, URL, notes
- **Show/hide all** — toggle to list all passwords
- **Create new entries** — separate window with full form
- **Create new databases** — start fresh from settings
- **Auto-lock** — configurable timeout
- **Translations:** English, German
- **Supports KDBX3 and KDBX4** databases (compatible with KeePassXC, KeePass2)

## Installation

### Prerequisites

```bash
sudo apt install wl-clipboard
```

### Build & Install

```bash
git clone https://github.com/raph-ael/cosmic-keepass.git
cd cosmic-keepass
cargo build --release
sudo just install
```

### Add to Panel

Right-click the COSMIC panel → Edit Panel → Applets → Add "KeePass"

## Usage

1. **Click the lock icon** in the panel
2. **Enter master password** → database unlocks
3. **Type to search** — matching entries appear below
4. For each entry: click 🔑 to copy password, 👤 to copy username, ⋮ for details
5. **"New password"** opens a form to add entries
6. **"Show all"** toggles the full password list

### First-time Setup

1. Click the applet → Settings
2. Either point to an existing `.kdbx` file or create a new one
3. Save

## Configuration

Config: `~/.config/cosmic-keepass/config.json`

## License

MIT
