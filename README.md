# Vela Widgets

Vela is a Windows desktop-widget app built as one Tauri 2 + Rust + Vue 3 + TypeScript repository. This is the first implementation slice; the visual direction and interaction details remain open to iteration.

## Current slice

- A hidden-on-launch manager window that opens from a widget's right-click menu.
- Independent calendar, to-do, clock, note, and countdown windows; first launch enables the calendar so the manager always has a desktop entry point.
- Local SQLite storage for component preferences, notes, countdown events and to-do items, with backward-compatible settings migration.
- Three fixed size presets for each widget, shared appearance, position locking and native context menus.
- Calendar: medium/large month navigation, offline 2026 mainland China holiday and adjusted-workday markers, independent visibility controls and optional daily updates. Manual checks are available; validated updates persist in SQLite and failures keep existing data. The public update source becomes available after publishing the holiday files to main; see [holiday data maintenance](data/holidays/README.md).
- Clock: 12/24-hour display, optional seconds, and up to three offline world-clock cities with automatic daylight-saving offsets.
- Notes: multiple autosaved Markdown notes in one window, titles from the first nonempty line, desktop create/delete with undo, right-click list, per-note colors and 3D vertical switching. Rendering includes tables, tasks, footnotes, code highlighting, KaTeX and Mermaid. Images load only on request; raw HTML displays as text.
- Per-note retention defaults to never delete; optional durations start at creation time, and expired notes are removed while running or on the next launch. Existing single-note content migrates automatically.
- Countdown: create/edit/delete events in preferences, annual recurrence and elapsed-day mode. Feb 29 recurrences use Feb 28 in non-leap years.
- To-do and countdown use manual vertical pagination with shared arrows and page indicators; notes retain a thin scrollbar and enter editing only from the edit button.
- Habits: local daily records with configurable weekdays, start date, daily count target, goal days, Emoji, colour and encouragement. Card, list and weekly report styles support all three sizes; the focused card switches only after reaching the day's target. The manager includes create/edit/pause/delete/reorder, monthly/weekly/yearly reports, backfill/undo and optional mood, rating and numeric results. Historical records keep their original target. Opt-in scheduled reminders reuse the native clock reminder window while the app is running; completed/rest/paused habits do not remind, and past reminders are not replayed. Preset streak milestones are calculated from records; custom rewards are not part of this slice.
- New widgets are disabled by default; enable them from preferences. Widget content works offline; holiday updates connect only when explicitly requested or enabled.
- Now playing: an opt-in Windows SMTC widget that follows the system's current media session. Every size has progress, seeking when supported, and a player volume slider with mute; medium and large also show artist, previous/next and elapsed/total time. Core Audio volume targets only sessions with the exact source executable or package identity, including the player's output on non-default devices; unavailable or ambiguous matches disable volume without falling back to system volume. Browser media shares its browser application's mixer volume. Metadata, playback, timeline, audio-session and device updates use system events; closing the widget releases subscriptions. Covers come from the system thumbnail stream (raster formats up to 2 MiB); Vela makes no media network requests and stores no playback history. Missing covers, missing timeline, unsupported controls, no media and connection failures have explicit states. Browser preview cannot read system media.
- Light and dark themes, with a blue accent that can be adjusted in the appearance page.
- Now-playing themes: Classic, Cover card, Colour vinyl and Minimal strip, selectable in the now-playing preferences with previews of all three sizes. Cover card uses a portrait 170 × 364 medium window; the other medium themes use 364 × 170. Switching themes and sizes resizes the native window and keeps it on the current monitor. Cover card and vinyl sample three colours locally from the system cover, with readable white text and a fallback for missing artwork. The selection persists locally and older settings migrate to Classic. Vinyl rotation follows playback and respects reduced motion. Themes reuse SMTC metadata and controls; lyrics, listening statistics, favourites, downloads and audio spectrum data are not available.
- In-browser preview mode with local storage when the app is run outside Tauri.

The Windows login-start setting, shell-level desktop pinning, and display/DPI-aware recovery still need native implementation and validation. Widget windows request tool-window behavior and are excluded from the taskbar, but their final Alt+Tab behavior still needs manual Windows validation. Window bounds are saved in this slice, but restoring them safely after monitor or scaling changes remains unfinished.

The visual system in this first slice is a starting point, not a fixed design standard. Component proportions, density, materials, and colors remain open to iteration.

## Run


```powershell
npm install
```

For a development run whose WebView2 profile and SQLite file stay inside this checkout:

```powershell
$env:VELA_DEV_DATA_DIR = (Join-Path (Get-Location) "src-tauri\.dev-data")
npm start
```

`npm start` launches the native Tauri app. `npm run dev` is only a browser layout preview; it cannot create or move native desktop widget windows.

The manager is hidden on normal launch. To show it at startup while reviewing the manager UI, set this debug-only switch first:

```powershell
$env:VELA_SHOW_MANAGER = "1"
```

For a browser-only layout preview:

```powershell
npm run dev
```

The Tauri build uses the installed Microsoft Edge WebView2 runtime. The first build downloads Rust and JavaScript dependencies if they are not already cached. The debug-only data path and manager preview switches are ignored by release builds.

## Project structure

Frontend code is grouped by feature rather than by file type:

```text
src/
  main.ts                   Application bootstrap
  app/                      Root view and shared application state
  features/
    calendar/               Calendar widgets, editors and holiday logic
    clock/                  Clock widgets, themes and clock tools
    countdown/              Countdown widget and date calculations
    habits/                 Habit widgets, management, reports and styles
    notes/                  Notes, Markdown rendering and retention
    media/                  Now-playing widget and runtime media session UI
    todos/                  To-do widget and pagination
  shared/
    ui/                     Reusable controls and icons
    widgets/                Common widget frame, settings and previews
    composables/            Shared Vue lifecycle helpers
    styles/                 Global theme and shared widget styles
    types.ts                Shared settings and snapshot contracts
  views/                    Manager, context-menu and reminder windows
  infrastructure/           Tauri commands and browser storage adapter
src-tauri/src/
  features/                 Native clock, habit, holiday, media and storage modules
  platform/                 Windows-specific surface integration
  lib.rs                    App setup, shared database and window coordination
  main.rs                   Native executable entry
```

Keep feature-specific UI and pure logic together in `features/<name>`. Put controls shared by multiple features in `shared`; window-level composition belongs in `views`. Application state lives in `app`, while native communication and browser persistence stay in `infrastructure`. Shared snapshot contracts remain centralized because the frontend and native backend exchange one snapshot.

Tests in `tests/` load pure logic from its feature directory. Holiday source data remains in `data/holidays/`; moving code must preserve its bundled data references. This layout change does not alter Tauri command names, window labels, database tables or stored settings.

## Validation

```powershell
npm run build
npm test
cd src-tauri
cargo test --locked
```

Browser previews of the new widgets use their actual preset dimensions. Timer refreshes pause when the document is hidden and realign on focus/visibility changes. Native occlusion, dragging, IME and multi-monitor behavior still require Windows manual validation.

The now-playing fixture and native integration checks are documented in [media validation](tests/media-manual.md). These opt-in checks require an actual Windows desktop media service; the default test suite remains independent of external players. The silent fixture validates play/pause, track changes, seeking, cover reads and SMTC events. The volume integration check restores the fixture's original mixer volume and verifies other applications and endpoint volume stay unchanged. Third-party player support depends on the metadata and controls it exposes to Windows.
