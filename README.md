# Vela Widgets

Vela is a Windows desktop-widget app built as one Tauri 2 + Rust + Vue 3 + TypeScript repository. This is the first implementation slice; the visual direction and interaction details remain open to iteration.

## Current slice

- A hidden-on-launch manager window that opens from a widget's right-click menu.
- Independent calendar and to-do windows; first launch enables the calendar so the manager always has a desktop entry point.
- Local SQLite storage for component preferences and to-do items.
- Light and dark themes, with a blue accent that can be adjusted in the appearance page.
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
