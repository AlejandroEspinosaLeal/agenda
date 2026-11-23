<div align="center">
  <img src="assets/icon.ico" width="64" height="64" alt="Mi Agenda Logo" />
  <h1>Mi Agenda</h1>
  <p><strong>A lightweight Rust desktop application for personal planning, monthly calendar, daily schedule, and weekly overview.</strong></p>

  ![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
  ![Platform](https://img.shields.io/badge/Platform-Windows-0078D6?logo=windows)
  ![Version](https://img.shields.io/badge/Version-1.0.0-4338CA)

  <p><strong>English</strong> | <a href="README.es.md">Español</a></p>
</div>

---

## Key Features

- **Three Planning Views**:
  - **Monthly Calendar (`Calendario Mensual`)**: 7-column grid with month navigation, category filters, today highlight (`HOY`), quick `+` button per cell, double-click to open the day's schedule, and right-hand detail & quick-add inspector.
  - **Daily Schedule (`Agenda Diaria`)**: chronological timeline with color-coded side strips, dynamic current-time indicator (`ACTUAL`), task progress bar, daily rhythm summary, and auto-saved quick notes.
  - **Weekly View (`Vista Semanal`)**: 7-column weekly board (`LUN`–`DOM`) with per-day task cards, category breakdown, editable `Intención de la Semana` note, workload balance, and summary export (`Semana_N_Resumen.txt` on the Desktop).
- **Flexible Tasks & Custom Colors**:
  - Task durations of **1 day**, **3 days**, **1 week** (`7 days`), or custom (`1–14 days`).
  - Categories (`Trabajo`, `Personal`, `Estudio`, `Salud`), priorities (`Baja`, `Media`, `Alta`), and a custom RGB color picker.
- **Accessible Light & Dark Themes**: editorial light palette (`#F8F9FC` / `#FFFFFF` / indigo `#4338CA`) and high-contrast dark palette, plus **Focus Mode** (`En foco`) and global search (`Ctrl+K` / `Ctrl+F`, clear with `Esc`).
- **Windows Integration**:
  - **System Tray Icon** (`tray-icon`) with a live numeric badge for today's pending tasks and hover tooltip summary.
  - **Floating Mini-Calendar** (`Mini`) and **draggable taskbar pill widget** (`224×34 px`) positioned near the notification area.
  - **Windows Auto-Start** via `--autostart` and **Inno Setup** installer (`Mi_Agenda_Setup.exe`).
- **Protected Local Persistence**: data is automatically saved to `%LOCALAPPDATA%\organizador_tareas\tareas.dat` in an obfuscated binary format (`OTSK`), keeping the Desktop clean of JSON files.
- **Built-in Auto-Updater**: checks [GitHub Releases](https://github.com/AlejandroEspinosaLeal/agenda/releases), downloads `Mi_Agenda_Setup.exe`, and runs a silent update (`/SILENT`) while preserving all user data in `tareas.dat`.

> [!NOTE]
> The updater only replaces the application binary at `%LOCALAPPDATA%\Programs\Mi Agenda\Mi_Agenda.exe`. Your tasks, daily notes, and weekly intentions remain untouched in `%LOCALAPPDATA%\organizador_tareas\tareas.dat`.

---

## Installation (Windows)

1. Download **`Mi_Agenda_Setup.exe`** from the latest [Releases](https://github.com/AlejandroEspinosaLeal/agenda/releases).
2. Run the installer (no administrator privileges required).
3. Optionally keep **Iniciar automáticamente con Windows** checked so the tray icon and taskbar pill launch automatically on login.

---

## Building from Source

### Prerequisites

- [Rust](https://www.rust-lang.org/) (`cargo` and `rustc`, 2021 edition)
- *(Optional)* [UPX](https://upx.github.io/) for executable compression
- *(Optional)* [Inno Setup 6](https://jrsoftware.org/isinfo.php) to build `Mi_Agenda_Setup.exe`

### Commands

```powershell
# Run unit tests
cargo test

# Run linter with warnings as errors
cargo clippy --all-targets -- -D warnings

# Build size-optimized release binary (LTO + opt-level = "z" + strip)
cargo build --release
```

The compiled executable is output to `target\release\organizador_tareas.exe`. To compress it and package the installer:

```powershell
# Compress with UPX (optional)
upx --best --lzma target\release\organizador_tareas.exe

# Build installer at target\installer\Mi_Agenda_Setup.exe
"C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer.iss
```

---

## Project Structure

```text
organizador_tareas/
├── assets/
│   └── icon.ico          # Multi-resolution application icon (16..256 px)
├── src/
│   ├── main.rs           # Entry point and --autostart flag handling
│   ├── app.rs            # Main app state, top bar, sidebar, modal, and taskbar pill
│   ├── calendar.rs       # Civil calendar math, ISO weeks, and date arithmetic
│   ├── model.rs          # Task/TaskStore data model and OTSK binary persistence
│   ├── theme.rs          # Clarity Suite light & accessible dark palettes + Segoe UI
│   ├── tray.rs           # Windows system tray icon, numeric badge, and context menu
│   ├── updater.rs        # GitHub Releases update checker and silent installer runner
│   └── views/
│       ├── mod.rs        # Shared UI helpers and category filters
│       ├── monthly.rs    # Monthly Calendar view
│       ├── daily.rs      # Daily Schedule view
│       ├── weekly.rs     # Weekly Overview view
│       └── mini.rs       # Floating Mini-Calendar view
├── build.rs              # Embeds icon.ico into Windows binary via winresource
├── Cargo.toml            # Dependencies (eframe, serde, serde_json, tray-icon) & release profile
└── installer.iss         # Inno Setup 6 script for Mi_Agenda_Setup.exe
```

---

## Keyboard Shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl + K` / `Ctrl + F` | Focus global search |
| `Esc` | Close modal or clear search |
| `←` / `→` | Previous / next month, day, or week (depending on active view) |
| `Enter` | Save task from title input |
