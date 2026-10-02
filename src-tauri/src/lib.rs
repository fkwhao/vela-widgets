use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, sync::Mutex, thread, time::Duration};
use tauri::{
    webview::WebviewWindowBuilder, AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize,
    State, WebviewUrl, WindowEvent,
};

#[cfg(target_os = "windows")]
mod native_surface;

const WIDGET_KINDS: [&str; 2] = ["calendar", "todo"];
const SNAPSHOT_UPDATED_EVENT: &str = "vela://snapshot-updated";
const CONTEXT_MENU_WIDTH: f64 = 188.0;
const CONTEXT_MENU_HEIGHT: f64 = 110.0;
// The to-do menu adds a "新建待办" item (31px) and a divider (7px).
const TODO_CONTEXT_MENU_HEIGHT: f64 = 148.0;
const CONTEXT_MENU_CORNER_RADIUS: u8 = 11;

fn default_widget_transparency() -> u8 {
    12
}

fn default_widget_corner_radius() -> u8 {
    19
}

pub struct AppState {
    database: Mutex<Connection>,
    context_menu_widget: Mutex<Option<String>>,
    context_menu_ready: Mutex<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetSettings {
    pub enabled: bool,
    pub always_on_top: bool,
    pub locked: bool,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    pub accent_color: String,
    #[serde(default = "default_widget_transparency")]
    pub widget_transparency: u8,
    #[serde(default = "default_widget_corner_radius")]
    pub widget_corner_radius: u8,
    pub week_starts_monday: bool,
    pub widgets: HashMap<String, WidgetSettings>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub id: i64,
    pub title: String,
    pub due_date: Option<String>,
    pub completed: bool,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub settings: Settings,
    pub todos: Vec<TodoItem>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut widgets = HashMap::new();
        widgets.insert(
            "calendar".to_string(),
            WidgetSettings {
                enabled: true,
                always_on_top: false,
                locked: false,
                x: None,
                y: None,
                width: 332.0,
                height: 450.0,
            },
        );
        widgets.insert(
            "todo".to_string(),
            WidgetSettings {
                enabled: false,
                always_on_top: false,
                locked: false,
                x: None,
                y: None,
                width: 350.0,
                height: 430.0,
            },
        );
        Self {
            theme: "light".to_string(),
            accent_color: "#3b67b8".to_string(),
            widget_transparency: default_widget_transparency(),
            widget_corner_radius: default_widget_corner_radius(),
            week_starts_monday: true,
            widgets,
        }
    }
}

fn app_data_path(app: &tauri::App) -> Result<PathBuf, Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    if let Some(directory) = std::env::var_os("VELA_DEV_DATA_DIR") {
        let directory = PathBuf::from(directory);
        fs::create_dir_all(&directory)?;
        return Ok(directory.join("vela.sqlite3"));
    }

    let directory = app.path().app_data_dir()?;
    fs::create_dir_all(&directory)?;
    Ok(directory.join("vela.sqlite3"))
}

fn webview_data_path(app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(debug_assertions)]
    if let Some(directory) = std::env::var_os("VELA_DEV_DATA_DIR") {
        let directory = PathBuf::from(directory).join("webview2");
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        return Ok(directory);
    }

    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("webview2");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory)
}

fn initialize_database(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS app_settings (
           id INTEGER PRIMARY KEY CHECK (id = 1),
           document TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS todos (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           title TEXT NOT NULL,
           due_date TEXT,
           completed INTEGER NOT NULL DEFAULT 0,
           created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );",
    )?;
    let has_settings: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM app_settings WHERE id = 1)",
        [],
        |row| row.get(0),
    )?;
    if !has_settings {
        let document = serde_json::to_string(&Settings::default()).unwrap_or_else(|_| "{}".into());
        connection.execute(
            "INSERT INTO app_settings (id, document) VALUES (1, ?1)",
            [document],
        )?;
    }
    Ok(())
}

fn read_settings(connection: &Connection) -> rusqlite::Result<Settings> {
    let document: Option<String> = connection
        .query_row(
            "SELECT document FROM app_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(document
        .and_then(|value| serde_json::from_str::<Settings>(&value).ok())
        .unwrap_or_default())
}

fn write_settings(connection: &Connection, settings: &Settings) -> rusqlite::Result<()> {
    let document = serde_json::to_string(settings).unwrap_or_else(|_| "{}".into());
    connection.execute(
        "INSERT INTO app_settings (id, document) VALUES (1, ?1)
         ON CONFLICT(id) DO UPDATE SET document = excluded.document",
        [document],
    )?;
    Ok(())
}

fn read_todos(connection: &Connection) -> rusqlite::Result<Vec<TodoItem>> {
    let mut statement = connection.prepare(
        "SELECT id, title, due_date, completed, created_at
         FROM todos
         ORDER BY completed ASC, due_date IS NULL ASC, due_date ASC, id DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(TodoItem {
            id: row.get(0)?,
            title: row.get(1)?,
            due_date: row.get(2)?,
            completed: row.get::<_, i64>(3)? != 0,
            created_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

fn read_snapshot(connection: &Connection) -> rusqlite::Result<AppSnapshot> {
    Ok(AppSnapshot {
        settings: read_settings(connection)?,
        todos: read_todos(connection)?,
    })
}

fn lock_database(state: &AppState) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
    state
        .database
        .lock()
        .map_err(|_| "本地数据暂时不可用，请重启 Vela 后重试。".to_string())
}

fn update_settings(
    state: &AppState,
    update: impl FnOnce(&mut Settings) -> Result<(), String>,
) -> Result<(), String> {
    let connection = lock_database(state)?;
    let mut settings = read_settings(&connection).map_err(|error| error.to_string())?;
    update(&mut settings)?;
    write_settings(&connection, &settings).map_err(|error| error.to_string())
}

fn publish_snapshot(app: &AppHandle, state: &AppState) -> Result<AppSnapshot, String> {
    let snapshot = {
        let connection = lock_database(state)?;
        read_snapshot(&connection).map_err(|error| error.to_string())?
    };
    app.emit(SNAPSHOT_UPDATED_EVENT, snapshot.clone())
        .map_err(|error| error.to_string())?;
    Ok(snapshot)
}

#[cfg(target_os = "windows")]
fn apply_widget_material(window: &tauri::WebviewWindow, settings: &Settings) -> Result<(), String> {
    native_surface::apply(window, settings.widget_transparency)
}

#[cfg(not(target_os = "windows"))]
fn apply_widget_material(
    _window: &tauri::WebviewWindow,
    _settings: &Settings,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_widget_corners(window: &tauri::WebviewWindow, radius: u8) -> Result<(), String> {
    // Composition and CSS retain fractional pixel coverage at the curved edge.
    // A GDI HWND region would turn that coverage into a jagged binary cutout.
    native_surface::update_backdrop_geometry(window, radius)
}

#[cfg(not(target_os = "windows"))]
fn apply_widget_corners(_window: &tauri::WebviewWindow, _radius: u8) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_context_menu_material(window: &tauri::WebviewWindow) -> Result<(), String> {
    // The context menu has its own native HostBackdropBrush so its acrylic
    // samples the desktop behind the popup, independently of widget opacity.
    native_surface::apply(window, 50)
}

#[cfg(not(target_os = "windows"))]
fn apply_context_menu_material(_window: &tauri::WebviewWindow) -> Result<(), String> {
    Ok(())
}

fn sync_widget_surfaces(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let settings = {
        let connection = lock_database(state)?;
        read_settings(&connection).map_err(|error| error.to_string())?
    };

    for kind in WIDGET_KINDS {
        if let Some(window) = app.get_webview_window(kind) {
            apply_widget_material(&window, &settings)?;
            apply_widget_corners(&window, settings.widget_corner_radius)?;
        }
    }
    Ok(())
}

fn validate_widget(kind: &str) -> Result<(), String> {
    if WIDGET_KINDS.contains(&kind) {
        Ok(())
    } else {
        Err("找不到这个组件。".to_string())
    }
}

fn create_widget_window(
    app: &AppHandle,
    kind: &str,
    settings: &WidgetSettings,
    app_settings: &Settings,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(kind) {
        apply_widget_material(&window, app_settings)?;
        apply_widget_corners(&window, app_settings.widget_corner_radius)?;
        window.show().map_err(|error| error.to_string())?;
        return Ok(());
    }

    let title = if kind == "calendar" {
        "日历"
    } else {
        "待办"
    };
    let url = WebviewUrl::App(format!("index.html?view={kind}").into());
    let data_directory = webview_data_path(app)?;
    let mut builder = WebviewWindowBuilder::new(app, kind, url)
        .data_directory(data_directory)
        .title(title)
        .inner_size(settings.width, settings.height.max(320.0))
        .min_inner_size(280.0, 320.0)
        .resizable(!settings.locked)
        .decorations(false)
        .transparent(true)
        .no_redirection_bitmap(true)
        .shadow(false)
        .always_on_top(settings.always_on_top)
        .skip_taskbar(true)
        .visible(false);

    if let (Some(x), Some(y)) = (settings.x, settings.y) {
        builder = builder.position(x, y);
    } else {
        builder = builder.center();
    }

    let window = builder.build().map_err(|error| error.to_string())?;
    exclude_widget_from_switcher(&window)?;
    apply_widget_material(&window, app_settings)?;
    apply_widget_corners(&window, app_settings.widget_corner_radius)?;
    window.show().map_err(|error| error.to_string())?;
    Ok(())
}

fn exclude_widget_from_switcher(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE,
            SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_CAPTION, WS_EX_APPWINDOW,
            WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU,
        };

        let handle = window.hwnd().map_err(|error| error.to_string())?;
        unsafe {
            // Defensively keep the widget borderless after applying native window
            // styles; this also prevents a caption from reappearing on frame refresh.
            let current_style = GetWindowLongPtrW(handle, GWL_STYLE);
            let captionless_style = current_style
                & !((WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0) as isize);
            SetWindowLongPtrW(handle, GWL_STYLE, captionless_style);

            let current = GetWindowLongPtrW(handle, GWL_EXSTYLE);
            let tool_window =
                (current | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize);
            SetWindowLongPtrW(handle, GWL_EXSTYLE, tool_window);
            SetWindowPos(
                handle,
                None,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
            )
            .map_err(|error| error.to_string())?;
        }
    }

    #[cfg(not(target_os = "windows"))]
    let _ = window;

    Ok(())
}

fn create_manager_window(app: &tauri::App) -> Result<(), String> {
    let data_directory = webview_data_path(app.handle())?;
    let visible = cfg!(debug_assertions)
        && std::env::var("VELA_SHOW_MANAGER")
            .map(|value| value == "1")
            .unwrap_or(false);
    WebviewWindowBuilder::new(
        app,
        "manager",
        WebviewUrl::App("index.html?view=manager".into()),
    )
    .data_directory(data_directory)
    .title("Vela Widgets")
    .inner_size(1000.0, 720.0)
    .min_inner_size(780.0, 580.0)
    .center()
    .visible(visible)
    .resizable(true)
    .decorations(true)
    .skip_taskbar(false)
    .build()
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot, String> {
    let connection = lock_database(&state)?;
    read_snapshot(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
async fn show_manager(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("manager")
        .ok_or_else(|| "中控窗口尚未准备好。".to_string())?;
    if window.is_minimized().unwrap_or(false) {
        window.unminimize().map_err(|error| error.to_string())?;
    }
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
fn get_context_menu_widget(state: State<'_, AppState>) -> Result<String, String> {
    state
        .context_menu_widget
        .lock()
        .map_err(|_| "右键菜单暂时不可用。".to_string())?
        .clone()
        .ok_or_else(|| "没有正在显示的组件菜单。".to_string())
}

// Must stay async: building a WebView window from a synchronous command
// deadlocks the main thread on Windows (WebView2).
#[tauri::command]
async fn show_context_menu(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    x: f64,
    y: f64,
) -> Result<(), String> {
    validate_widget(&kind)?;
    if !x.is_finite() || !y.is_finite() {
        return Err("右键菜单位置无效。".to_string());
    }
    let widget = app
        .get_webview_window(&kind)
        .ok_or_else(|| "组件窗口尚未准备好。".to_string())?;
    let scale = widget.scale_factor().map_err(|error| error.to_string())?;
    let widget_position = widget.outer_position().map_err(|error| error.to_string())?;
    let menu_height = if kind == "todo" {
        TODO_CONTEXT_MENU_HEIGHT
    } else {
        CONTEXT_MENU_HEIGHT
    };
    let popup_width = (CONTEXT_MENU_WIDTH * scale).round() as u32;
    let popup_height = (menu_height * scale).round() as u32;
    let mut popup_x = widget_position.x + (x * scale).round() as i32;
    let mut popup_y = widget_position.y + (y * scale).round() as i32;

    if let Some(monitor) = widget
        .current_monitor()
        .map_err(|error| error.to_string())?
    {
        let origin = monitor.position();
        let monitor_size = monitor.size();
        let max_x = origin.x + monitor_size.width as i32 - popup_width as i32;
        let max_y = origin.y + monitor_size.height as i32 - popup_height as i32;
        popup_x = popup_x.clamp(origin.x, max_x.max(origin.x));
        popup_y = popup_y.clamp(origin.y, max_y.max(origin.y));
    }

    *state
        .context_menu_widget
        .lock()
        .map_err(|_| "右键菜单暂时不可用。".to_string())? = Some(kind.clone());

    let (menu, already_created) = if let Some(window) = app.get_webview_window("context-menu") {
        (window, true)
    } else {
        let window = WebviewWindowBuilder::new(
            &app,
            "context-menu",
            WebviewUrl::App("index.html?view=context-menu".into()),
        )
        .data_directory(webview_data_path(&app)?)
        .title("Vela")
        .inner_size(CONTEXT_MENU_WIDTH, menu_height)
        .min_inner_size(CONTEXT_MENU_WIDTH, CONTEXT_MENU_HEIGHT)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .no_redirection_bitmap(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()
        .map_err(|error| error.to_string())?;
        exclude_widget_from_switcher(&window)?;
        apply_context_menu_material(&window)?;
        apply_widget_corners(&window, CONTEXT_MENU_CORNER_RADIUS)?;
        (window, false)
    };

    menu.set_size(PhysicalSize::new(popup_width, popup_height))
        .map_err(|error| error.to_string())?;
    menu.set_position(PhysicalPosition::new(popup_x, popup_y))
        .map_err(|error| error.to_string())?;
    apply_widget_corners(&menu, CONTEXT_MENU_CORNER_RADIUS)?;
    let ready = *state
        .context_menu_ready
        .lock()
        .map_err(|_| "右键菜单暂时不可用。".to_string())?;
    if already_created {
        if !ready {
            // The popup page never finished loading; let the widget fall back
            // to its in-window menu instead of silently showing nothing.
            return Err("右键菜单窗口尚未准备好。".to_string());
        }
        menu.show().map_err(|error| error.to_string())?;
        menu.set_focus().map_err(|error| error.to_string())?;
        let _ = app.emit_to("context-menu", "vela://context-menu-kind", kind);
    }
    Ok(())
}

#[tauri::command]
fn context_menu_ready(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let window = app
        .get_webview_window("context-menu")
        .ok_or_else(|| "右键菜单窗口尚未准备好。".to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    *state
        .context_menu_ready
        .lock()
        .map_err(|_| "右键菜单暂时不可用。".to_string())? = true;
    Ok(())
}

#[tauri::command]
fn dismiss_context_menu(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("context-menu") {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn set_widget_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    enabled: bool,
) -> Result<AppSnapshot, String> {
    validate_widget(&kind)?;
    let (settings, previous_enabled) = {
        let connection = lock_database(&state)?;
        let mut settings = read_settings(&connection).map_err(|error| error.to_string())?;
        if !enabled
            && settings
                .widgets
                .values()
                .filter(|widget| widget.enabled)
                .count()
                <= 1
        {
            return Err("至少保留一个桌面组件，才能从桌面打开 Vela 偏好设置。".to_string());
        }
        let widget = settings
            .widgets
            .get_mut(&kind)
            .ok_or_else(|| "找不到这个组件的设置。".to_string())?;
        let previous_enabled = widget.enabled;
        widget.enabled = enabled;
        write_settings(&connection, &settings).map_err(|error| error.to_string())?;
        (settings, previous_enabled)
    };

    let window_result = if enabled {
        let widget = settings.widgets.get(&kind).expect("validated widget kind");
        create_widget_window(&app, &kind, widget, &settings)
    } else if let Some(window) = app.get_webview_window(&kind) {
        window.close().map_err(|error| error.to_string())
    } else {
        Ok(())
    };

    if let Err(error) = window_result {
        let _ = update_settings(state.inner(), |settings| {
            if let Some(widget) = settings.widgets.get_mut(&kind) {
                widget.enabled = previous_enabled;
            }
            Ok(())
        });
        return Err(error);
    }

    publish_snapshot(&app, state.inner())
}

#[tauri::command]
async fn set_widget_layer(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    always_on_top: bool,
) -> Result<AppSnapshot, String> {
    validate_widget(&kind)?;
    update_settings(state.inner(), |settings| {
        let widget = settings
            .widgets
            .get_mut(&kind)
            .ok_or_else(|| "找不到这个组件的设置。".to_string())?;
        widget.always_on_top = always_on_top;
        Ok(())
    })?;
    if let Some(window) = app.get_webview_window(&kind) {
        window
            .set_always_on_top(always_on_top)
            .map_err(|error| error.to_string())?;
    }
    publish_snapshot(&app, state.inner())
}

#[tauri::command]
async fn set_widget_locked(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    locked: bool,
) -> Result<AppSnapshot, String> {
    validate_widget(&kind)?;
    update_settings(state.inner(), |settings| {
        let widget = settings
            .widgets
            .get_mut(&kind)
            .ok_or_else(|| "找不到这个组件的设置。".to_string())?;
        widget.locked = locked;
        Ok(())
    })?;
    if let Some(window) = app.get_webview_window(&kind) {
        window
            .set_resizable(!locked)
            .map_err(|error| error.to_string())?;
    }
    publish_snapshot(&app, state.inner())
}

#[tauri::command]
async fn set_week_starts_monday(
    app: AppHandle,
    state: State<'_, AppState>,
    monday: bool,
) -> Result<AppSnapshot, String> {
    update_settings(state.inner(), |settings| {
        settings.week_starts_monday = monday;
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}

#[tauri::command]
async fn set_theme(
    app: AppHandle,
    state: State<'_, AppState>,
    theme: String,
) -> Result<AppSnapshot, String> {
    if !["light", "dark", "system"].contains(&theme.as_str()) {
        return Err("不支持这个主题。".to_string());
    }
    update_settings(state.inner(), |settings| {
        settings.theme = theme;
        Ok(())
    })?;
    sync_widget_surfaces(&app, state.inner())?;
    publish_snapshot(&app, state.inner())
}

#[tauri::command]
async fn set_accent_color(
    app: AppHandle,
    state: State<'_, AppState>,
    color: String,
) -> Result<AppSnapshot, String> {
    let valid_hex = color.len() == 7
        && color.starts_with('#')
        && color
            .chars()
            .skip(1)
            .all(|character| character.is_ascii_hexdigit());
    if !valid_hex {
        return Err("请选择有效的颜色。".to_string());
    }
    update_settings(state.inner(), |settings| {
        settings.accent_color = color;
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}

#[tauri::command]
async fn set_widget_appearance(
    app: AppHandle,
    state: State<'_, AppState>,
    widget_transparency: u8,
    widget_corner_radius: u8,
) -> Result<AppSnapshot, String> {
    if widget_transparency > 100 || !(8..=30).contains(&widget_corner_radius) {
        return Err("组件透明度或圆角数值无效。".to_string());
    }
    update_settings(state.inner(), |settings| {
        settings.widget_transparency = widget_transparency;
        settings.widget_corner_radius = widget_corner_radius;
        Ok(())
    })?;
    sync_widget_surfaces(&app, state.inner())?;
    publish_snapshot(&app, state.inner())
}

#[tauri::command]
fn save_widget_bounds(
    state: State<'_, AppState>,
    kind: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    validate_widget(&kind)?;
    let connection = lock_database(&state)?;
    let mut settings = read_settings(&connection).map_err(|error| error.to_string())?;
    let widget = settings
        .widgets
        .get_mut(&kind)
        .ok_or_else(|| "找不到这个组件的设置。".to_string())?;
    widget.x = Some(x);
    widget.y = Some(y);
    widget.width = width.max(280.0);
    widget.height = height.max(320.0);
    write_settings(&connection, &settings).map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_todo(
    app: AppHandle,
    state: State<'_, AppState>,
    title: String,
    due_date: Option<String>,
) -> Result<AppSnapshot, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("先写下待办内容。".to_string());
    }
    {
        let connection = lock_database(&state)?;
        connection
            .execute(
                "INSERT INTO todos (title, due_date) VALUES (?1, ?2)",
                params![title, due_date],
            )
            .map_err(|error| error.to_string())?;
    }
    publish_snapshot(&app, state.inner())
}

#[tauri::command]
async fn update_todo(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    title: String,
    due_date: Option<String>,
) -> Result<AppSnapshot, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("待办内容不能为空。".to_string());
    }
    {
        let connection = lock_database(&state)?;
        let updated = connection
            .execute(
                "UPDATE todos SET title = ?1, due_date = ?2 WHERE id = ?3",
                params![title, due_date, id],
            )
            .map_err(|error| error.to_string())?;
        if updated == 0 {
            return Err("这条待办已不存在，请刷新后重试。".to_string());
        }
    }
    publish_snapshot(&app, state.inner())
}

#[tauri::command]
async fn set_todo_completed(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    completed: bool,
) -> Result<AppSnapshot, String> {
    {
        let connection = lock_database(&state)?;
        connection
            .execute(
                "UPDATE todos SET completed = ?1 WHERE id = ?2",
                params![completed, id],
            )
            .map_err(|error| error.to_string())?;
    }
    publish_snapshot(&app, state.inner())
}

#[tauri::command]
async fn delete_todo(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<AppSnapshot, String> {
    {
        let connection = lock_database(&state)?;
        connection
            .execute("DELETE FROM todos WHERE id = ?1", [id])
            .map_err(|error| error.to_string())?;
    }
    publish_snapshot(&app, state.inner())
}

#[tauri::command]
fn exit_vela(app: AppHandle) {
    app.exit(0);
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let database_path = app_data_path(app).map_err(|error| {
                std::io::Error::other(format!(
                    "failed to resolve Vela's local data directory: {error}"
                ))
            })?;
            let connection = Connection::open(&database_path).map_err(|error| {
                std::io::Error::other(format!("failed to open Vela's local database: {error}"))
            })?;
            initialize_database(&connection).map_err(|error| {
                std::io::Error::other(format!(
                    "failed to initialize Vela's local database: {error}"
                ))
            })?;
            let initial_settings = read_settings(&connection).map_err(|error| {
                std::io::Error::other(format!("failed to load Vela's settings: {error}"))
            })?;
            app.manage(AppState {
                database: Mutex::new(connection),
                context_menu_widget: Mutex::new(None),
                context_menu_ready: Mutex::new(false),
            });

            create_manager_window(app).map_err(|error| {
                std::io::Error::other(format!("failed to create manager window: {error}"))
            })?;

            for (kind, settings) in &initial_settings.widgets {
                if settings.enabled {
                    create_widget_window(app.handle(), kind, settings, &initial_settings).map_err(
                        |error| {
                            std::io::Error::other(format!(
                                "failed to create {kind} widget window: {error}"
                            ))
                        },
                    )?;
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if WIDGET_KINDS.contains(&window.label())
                && matches!(
                    event,
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
                )
            {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                if let Ok(connection) = lock_database(state.inner()) {
                    if let Ok(settings) = read_settings(&connection) {
                        if let Some(widget_window) = app.get_webview_window(window.label()) {
                            if let Err(error) =
                                apply_widget_corners(&widget_window, settings.widget_corner_radius)
                            {
                                eprintln!("failed to update widget corner geometry: {error}");
                            }
                        }
                    }
                }
            }
            if WIDGET_KINDS.contains(&window.label())
                && matches!(event, WindowEvent::ThemeChanged(_))
            {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                if let Ok(connection) = lock_database(state.inner()) {
                    if let Ok(settings) = read_settings(&connection) {
                        if settings.theme == "system" {
                            if let Some(widget_window) = app.get_webview_window(window.label()) {
                                if let Err(error) = apply_widget_material(&widget_window, &settings)
                                {
                                    eprintln!("failed to update widget material: {error}");
                                }
                            }
                        }
                    }
                }
            }
            if window.label() == "manager" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            if window.label() == "context-menu" {
                match event {
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                        if let Some(menu_window) =
                            window.app_handle().get_webview_window("context-menu")
                        {
                            if let Err(error) =
                                apply_widget_corners(&menu_window, CONTEXT_MENU_CORNER_RADIUS)
                            {
                                eprintln!("failed to update context menu corner geometry: {error}");
                            }
                        }
                    }
                    WindowEvent::Focused(false) => {
                        // WebView2 can report focus loss while dispatching the
                        // clicked menu item. Defer dismissal so its click handler runs.
                        let app = window.app_handle().clone();
                        thread::spawn(move || {
                            thread::sleep(Duration::from_millis(180));
                            if let Some(menu) = app.get_webview_window("context-menu") {
                                if let Ok(false) = menu.is_focused() {
                                    let _ = menu.hide();
                                }
                            }
                        });
                    }
                    WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_context_menu_widget,
            show_manager,
            show_context_menu,
            context_menu_ready,
            dismiss_context_menu,
            set_widget_enabled,
            set_widget_layer,
            set_widget_locked,
            set_week_starts_monday,
            set_theme,
            set_accent_color,
            set_widget_appearance,
            save_widget_bounds,
            create_todo,
            update_todo,
            set_todo_completed,
            delete_todo,
            exit_vela
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Vela Widgets");
}
