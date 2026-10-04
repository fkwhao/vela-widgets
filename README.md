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
- Light and dark themes, with a blue accent that can be adjusted in the appearance page.
- In-browser preview mode with local storage when the app is run outside Tauri.

The Windows login-start setting, shell-level desktop pinning, and display/DPI-aware recovery still need native implementation and validation. Widget windows request tool-window behavior and are excluded from the taskbar, but their final Alt+Tab behavior still needs manual Windows validation. Window bounds are saved in this slice, but restoring them safely after monitor or scaling changes remains unfinished.

The visual system in this first slice is a starting point, not a fixed design standard. Component proportions, density, materials, and colors remain open to iteration.

## Run

Component progress and interface requirements are recorded in [组件清单](产品设计/组件清单.md) and [界面设计规范](产品设计/界面设计规范.md).

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

## Validation

```powershell
npm run build
npm test
cd src-tauri
cargo test --locked
```

Browser previews of the new widgets use their actual preset dimensions. Timer refreshes pause when the document is hidden and realign on focus/visibility changes. Native occlusion, dragging, IME and multi-monitor behavior still require Windows manual validation.
