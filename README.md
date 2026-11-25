<div align="center">
  <img src="assets/icon.ico" width="64" height="64" alt="Mi Agenda Logo" />
  <h1>Mi Agenda</h1>
  <p><strong>A lightweight Rust desktop application for personal planning, time-blocking, daily focus, and weekly overview.</strong></p>

  ![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
  ![Platform](https://img.shields.io/badge/Platform-Windows-0078D6?logo=windows)
  ![Version](https://img.shields.io/badge/Version-1.2.2-4338CA)

  <p><strong>English</strong> | <a href="README.es.md">Español</a></p>
</div>

---

## Key Features

- **Three Interactive Planning Views**:
  - **Monthly Calendar (`Calendario Mensual`)**: 7-column grid with Today badge (`HOY`), hover `+` quick-add, up to 3 compact blocks per cell + `+N más`, **multi-day drag selection** to create multi-day tasks/events at once, and a collapsible contextual right-hand inspector with debounced daily notes.
  - **Daily Schedule (`Agenda Diaria`)**: real `06:00–23:00` (expandable to `24h`) time-blocking grid with an `All Day` strip, live current-time indicator (`ACTUAL`), pure-Rust parallel overlap column layout, and daily rhythm summary.
  - **Weekly View (`Vista Semanal`)**: dual **Schedule Mode** (shared 7-day hourly grid) and **Board Mode**, collapsible analytical footer (category hours, `Intención de la Semana`, workload balance), and export menu.
- **Direct Manipulation & Backlog**:
  - Click or drag on empty time slots (15-min snap) to create blocks, drag blocks across hours/days, resize duration from the bottom handle, and drag unscheduled tasks from the sidebar **Backlog** directly onto the calendar.
- **Command Palette (`Ctrl+K`) & Natural Language Entry**:
  - Centered modal to search tasks/events/notes, run quick commands, and create blocks using natural language (e.g. `"Estudiar Rust 18:00-19:30 #Estudio"` or `"Reunión mañana 10:00 1h #Trabajo"`).
- **Focus Mode (`F`)**:
  - Hides sidebars, centers the active block, and runs a configurable **Pomodoro (`25/5`) / Deep Work (`50/10`) / Remaining Block** timer alongside session notes (zero CPU when paused or idle).
- **Real Export**:
  - Export weekly summaries to **Markdown (`.md`)**, plain text (`.txt`), or standard **iCalendar (`.ics` RFC 5545)** with `VCALENDAR`/`VEVENT`/`RRULE`.
- **Custom Color Palettes & Light/Dark Mode**:
  - 6 WCAG AA-compliant color palettes (**Índigo**, **Océano**, **Bosque**, **Terracota**, **Rosa**, **Lavanda**) with persisted Light/Dark mode preference.
- **Interactive Onboarding Tour (`?`)**:
  - First-launch spotlight walkthrough that automatically navigates across Monthly, Weekly, Daily, Backlog, `Ctrl+K`, Focus Mode, and Color Palettes/Notifications (replayable anytime via `?`).
- **Windows Integration & Auto-Updater**:
  - **Daily Windows Toast Notification** once per day at launch/midnight with today's pending items (also triggerable from the tray menu or `Ctrl+K`).
  - **System Tray Icon** (`tray-icon`) with live pending badge, `--autostart` support, and **Inno Setup** installer (`Mi_Agenda_Setup.exe`).
  - **Automatic GitHub Releases Updater**: checks `releases/latest`, automatically downloads `Mi_Agenda_Setup.exe`, installs silently (`/SILENT /CLOSEAPPLICATIONS`), and relaunches while preserving `%LOCALAPPDATA%\organizador_tareas\tareas.dat` (`OTSK` v2 with automatic v1 migration).

---

## Installation (Windows)

1. Download **`Mi_Agenda_Setup.exe`** from the latest [Releases](https://github.com/AlejandroEspinosaLeal/agenda/releases).
2. Run the installer (no administrator privileges required).
3. Optionally keep **Iniciar automáticamente con Windows** checked so daily notifications and the tray icon run on login.

---

## Building from Source

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

Optional compression and installer packaging:

```powershell
upx --best --lzma target\release\organizador_tareas.exe
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
│   ├── app.rs            # Main state, header, sidebar, Command Palette, Focus Mode
│   ├── calendar.rs       # Civil calendar math, ISO weeks, and date arithmetic
│   ├── export.rs         # Markdown (.md) and RFC 5545 iCalendar (.ics) exporters
│   ├── focus.rs          # Focus/Pomodoro timer state machine
│   ├── model.rs          # v2 Task/Event model (chrono) and OTSK binary persistence
│   ├── nlp.rs            # Natural-language quick-entry parser for Ctrl+K
│   ├── theme.rs          # WCAG AA light & dark palettes, spacing, radius, categories
│   ├── timeblock.rs      # Interval overlap partitioning and 15-min snap helpers
│   ├── tour.rs           # Interactive multi-view spotlight onboarding tour
│   ├── tray.rs           # Windows system tray icon and daily Toast notifications
│   ├── updater.rs        # Automatic GitHub Releases downloader and silent updater
│   └── views/
│       ├── mod.rs        # Shared UI helpers and unified category filter bar
│       ├── monthly.rs    # Monthly view with multi-day drag and inspector
│       ├── daily.rs      # Daily time-blocking grid view
│       └── weekly.rs     # Dual Schedule/Board weekly view
├── build.rs              # Embeds icon.ico into Windows binary via winresource
├── Cargo.toml            # Dependencies (chrono, eframe, serde, serde_json, tray-icon)
└── installer.iss         # Inno Setup 6 script for Mi_Agenda_Setup.exe
```

---

## Keyboard Shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl + K` | Open Command Palette & quick natural-language entry |
| `Ctrl + F` | Focus view filter search |
| `M` / `W` / `D` | Switch to Monthly / Weekly / Daily view |
| `T` | Jump to Today |
| `C` | Open New Task / Event modal |
| `F` | Toggle Focus Mode |
| `←` / `→` | Previous / next month, week, or day |
| `Esc` | Close Command Palette, modal, tour, Focus Mode, or inspector |
