# cosmic-keepass

COSMIC panel applet for KeePass password management. Search, copy, and manage passwords from .kdbx databases.

## Features
- Master password unlock with async decryption (non-blocking UI)
- Search-first interface: results appear as you type
- Per-entry buttons: copy password, copy username, show details
- Show/hide all passwords toggle
- Create new entries via separate window (--new-entry)
- Create new .kdbx databases from settings
- Auto-lock with configurable timeout
- Supports KDBX3 and KDBX4 (compatible with KeePassXC, KeePass2)
- i18n: English, German

## Build & Install
```bash
sudo apt install wl-clipboard
cargo build --release
sudo just install
```

## Development
```bash
just run              # Run applet
cargo run --release -- --settings    # Settings window
cargo run --release -- --new-entry   # New entry window
```

## Project Structure
- `src/main.rs` — Entry point, CLI flags (--settings, --new-entry)
- `src/app.rs` — Panel applet UI: unlock, search, entry list, details view
- `src/settings.rs` — Settings window (database path, auto-lock, create DB)
- `src/new_entry.rs` — New entry window (master password auth + entry form)
- `src/kdbx.rs` — KeePass database operations (open, list entries, add entry, create)
- `src/config.rs` — Config struct, load/save
- `src/i18n.rs` — Localization with fl!() macro
- `i18n/` — Fluent translation files (en, de)

## Key Patterns
- Uses `keepass` crate with `save_kdbx4` feature for read/write
- Clipboard via `wl-copy` (piped stdin)
- Async unlock: `Task::perform` with `tokio::task::spawn_blocking` to avoid UI freeze
- Popup uses `cosmic::applet::menu_button` for full-width items with hover
- Padding `[space_xxs, 0]` for edge-to-edge buttons, `[0, space_xs]` for text inputs
- Config at `~/.config/cosmic-keepass/config.json`

## Known Issues
- Text input in applet popups requires a click to receive keyboard events (COSMIC panel limitation, see https://github.com/pop-os/cosmic-panel/issues/580)

## GitHub
- Repo: https://github.com/raph-ael/cosmic-keepass
- Listed in: https://github.com/cosmic-utils/cosmic-project-collection
