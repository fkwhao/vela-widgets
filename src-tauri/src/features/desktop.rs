use crate::*;

pub const LABEL: &str = "desktop";
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSettings {
    #[serde(default)]
    pub coordinate_version: u8,
    #[serde(default)]
    pub editing: bool,
    #[serde(default)]
    pub always_on_top: bool,
    #[serde(default)]
    pub saved_layouts: Vec<SavedLayout>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Placement {
    pub kind: String,
    pub size: String,
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedLayout {
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub placements: Vec<Placement>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub backdrop: bool,
}

pub fn migrate(settings: &mut Settings, origin: (f64, f64), bounds: (f64, f64)) {
    if settings.desktop.coordinate_version == 0 {
        settings.desktop.always_on_top = settings
            .widgets
            .values()
            .any(|w| w.enabled && w.always_on_top);
        for widget in settings.widgets.values_mut() {
            if let (Some(x), Some(y)) = (widget.x, widget.y) {
                widget.x = Some((x - origin.0).clamp(0.0, (bounds.0 - widget.width).max(0.0)));
                widget.y = Some((y - origin.1).clamp(0.0, (bounds.1 - widget.height).max(0.0)));
            }
        }
        settings.desktop.coordinate_version = 1;
    }
    settings.desktop.editing = false;
    for widget in settings.widgets.values_mut() {
        widget.always_on_top = settings.desktop.always_on_top;
    }
}

pub fn create(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("无法读取主显示器。")?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let window = WebviewWindowBuilder::new(
        app,
        LABEL,
        WebviewUrl::App("index.html?view=desktop".into()),
    )
    .data_directory(webview_data_path(app)?)
    .title("Vela 桌面组件")
    .position(
        area.position.x as f64 / scale,
        area.position.y as f64 / scale,
    )
    .inner_size(
        area.size.width as f64 / scale,
        area.size.height as f64 / scale,
    )
    .resizable(false)
    .maximizable(false)
    .decorations(false)
    .transparent(true)
    .no_redirection_bitmap(true)
    .shadow(false)
    .skip_taskbar(true)
    .always_on_top(settings.desktop.always_on_top)
    .visible(false)
    .build()
    .map_err(|e| e.to_string())?;
    exclude_widget_from_switcher(&window)?;
    // Start with an empty native region. The page supplies occupied rectangles before showing.
    #[cfg(target_os = "windows")]
    native_surface::update_desktop_regions(
        &window,
        Vec::new(),
        false,
        settings.widget_transparency,
    )?;
    Ok(())
}

#[tauri::command]
pub fn get_desktop_bounds(app: AppHandle) -> Result<serde_json::Value, String> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("桌面画布尚未准备好。")?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"width":size.width as f64/scale,"height":size.height as f64/scale}))
}
#[tauri::command]
pub async fn update_desktop_regions(
    window: tauri::WebviewWindow,
    rects: Vec<CanvasRect>,
    editing: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if window.label() != LABEL {
        return Err("只有桌面画布可以更新交互区域。".into());
    }
    if rects.len() > 80
        || rects.iter().any(|r| {
            [r.x, r.y, r.width, r.height, r.radius]
                .iter()
                .any(|v| !v.is_finite())
                || r.width < 0.0
                || r.height < 0.0
                || r.radius < 0.0
        })
    {
        return Err("无效的画布区域。".into());
    }
    let transparency = {
        let db = lock_database(state.inner())?;
        read_settings(&db)
            .map_err(|e| e.to_string())?
            .widget_transparency
    };
    #[cfg(target_os = "windows")]
    native_surface::update_desktop_regions(&window, rects, editing, transparency)?;
    #[cfg(not(target_os = "windows"))]
    let _ = (rects, editing, transparency);
    window.show().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_desktop_editing(
    app: AppHandle,
    state: State<'_, AppState>,
    editing: bool,
) -> Result<AppSnapshot, String> {
    update_settings(state.inner(), |s| {
        s.desktop.editing = editing;
        Ok(())
    })?;
    let next = publish_snapshot(&app, state.inner())?;
    if editing {
        if let Some(window) = app.get_webview_window(LABEL) {
            window.set_focus().map_err(|e| e.to_string())?;
        }
        if let Some(manager) = app.get_webview_window("manager") {
            let _ = manager.close();
        }
    }
    Ok(next)
}

fn validate_placements(placements: &[Placement]) -> Result<(), String> {
    if placements.is_empty() || placements.len() > WIDGET_KINDS.len() {
        return Err("布局至少需要一个组件。".into());
    }
    let mut seen = std::collections::HashSet::new();
    for p in placements {
        validate_widget(&p.kind)?;
        if !seen.insert(&p.kind)
            || !WIDGET_SIZES.contains(&p.size.as_str())
            || !p.x.is_finite()
            || !p.y.is_finite()
            || p.x < 0.0
            || p.y < 0.0
            || p.x > 100000.0
            || p.y > 100000.0
        {
            return Err("布局包含无效的位置或尺寸。".into());
        }
    }
    Ok(())
}
fn apply_placements(s: &mut Settings, placements: &[Placement]) {
    for (kind, widget) in &mut s.widgets {
        if !widget.enabled {
            continue;
        }
        if let Some(p) = placements.iter().find(|p| &p.kind == kind) {
            // A locked card without persisted coordinates uses its displayed
            // default position. Materialize it so other cards cannot shift it.
            if !widget.locked || widget.x.is_none() || widget.y.is_none() {
                widget.x = Some(p.x);
                widget.y = Some(p.y);
            }
        }
    }
}
#[tauri::command]
pub async fn apply_desktop_layout(
    app: AppHandle,
    state: State<'_, AppState>,
    placements: Vec<Placement>,
) -> Result<AppSnapshot, String> {
    validate_placements(&placements)?;
    update_settings(state.inner(), |s| {
        apply_placements(s, &placements);
        Ok(())
    })?;
    let next = publish_snapshot(&app, state.inner())?;
    Ok(next)
}
#[tauri::command]
pub fn save_desktop_layout(
    app: AppHandle,
    state: State<'_, AppState>,
    layout: SavedLayout,
) -> Result<AppSnapshot, String> {
    validate_placements(&layout.placements)?;
    if layout.name.trim().is_empty()
        || layout.name.chars().count() > 40
        || !layout.width.is_finite()
        || !layout.height.is_finite()
        || layout.width <= 0.0
        || layout.height <= 0.0
    {
        return Err("请填写有效的布局名称。".into());
    }
    update_settings(state.inner(), |s| {
        if let Some(existing) = s
            .desktop
            .saved_layouts
            .iter_mut()
            .find(|saved| saved.name == layout.name)
        {
            *existing = layout;
        } else {
            if s.desktop.saved_layouts.len() >= 12 {
                return Err("最多保存12份布局。".into());
            }
            s.desktop.saved_layouts.push(layout);
        }
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_converts_screen_coordinates_once_and_preserves_content() {
        let mut s = Settings::default();
        s.note.text = "keep".into();
        let w = s.widgets.get_mut("calendar").unwrap();
        w.x = Some(-800.0);
        w.y = Some(100.0);
        migrate(&mut s, (-1000.0, 0.0), (1200.0, 800.0));
        assert_eq!(s.widgets["calendar"].x, Some(200.0));
        migrate(&mut s, (-1000.0, 0.0), (1200.0, 800.0));
        assert_eq!(s.widgets["calendar"].x, Some(200.0));
        assert_eq!(s.note.text, "keep");
    }
    #[test]
    fn invalid_and_duplicate_layouts_are_rejected() {
        let p = Placement {
            kind: "clock".into(),
            size: "small".into(),
            x: 24.0,
            y: 24.0,
        };
        assert!(validate_placements(&[p.clone()]).is_ok());
        assert!(validate_placements(&[p.clone(), p.clone()]).is_err());
        assert!(validate_placements(&[Placement { x: f64::NAN, ..p }]).is_err());
    }
    #[test]
    fn locked_default_position_is_materialized_without_becoming_movable() {
        let mut s = Settings::default();
        let widget = s.widgets.get_mut("calendar").unwrap();
        widget.enabled = true;
        widget.locked = true;
        widget.x = None;
        widget.y = None;
        let p = Placement {
            kind: "calendar".into(),
            size: "small".into(),
            x: 24.0,
            y: 24.0,
        };
        apply_placements(&mut s, &[p.clone()]);
        assert_eq!(s.widgets["calendar"].x, Some(24.0));
        apply_placements(
            &mut s,
            &[Placement {
                x: 500.0,
                y: 500.0,
                ..p
            }],
        );
        assert_eq!(s.widgets["calendar"].x, Some(24.0));
        assert_eq!(s.widgets["calendar"].y, Some(24.0));
        assert!(s.widgets["calendar"].locked);
    }
    #[test]
    fn applying_layout_preserves_locked_widgets_and_contents_through_serialization() {
        let mut s = Settings::default();
        s.note.text = "unsaved ideas".into();
        s.widgets.get_mut("note").unwrap().enabled = true;
        let clock = s.widgets.get_mut("clock").unwrap();
        clock.enabled = true;
        clock.size = "medium".into();
        (clock.width, clock.height) = widget_dimensions("medium");
        let locked = s.widgets.get_mut("calendar").unwrap();
        locked.enabled = true;
        locked.locked = true;
        locked.x = Some(123.0);
        locked.y = Some(45.0);
        let placements = vec![
            Placement {
                kind: "clock".into(),
                size: "small".into(),
                x: 400.0,
                y: 250.0,
            },
            Placement {
                kind: "media".into(),
                size: "large".into(),
                x: 800.0,
                y: 24.0,
            },
        ];
        apply_placements(&mut s, &placements);
        assert!(s.widgets["calendar"].enabled);
        assert_eq!(s.widgets["calendar"].x, Some(123.0));
        assert!(s.widgets["clock"].enabled);
        assert_eq!(s.widgets["clock"].y, Some(250.0));
        assert!(s.widgets["note"].enabled);
        assert!(!s.widgets["media"].enabled);
        assert_eq!(s.widgets["clock"].size, "medium");
        assert_eq!(s.widgets["clock"].width, 364.0);
        s.desktop.saved_layouts.push(SavedLayout {
            name: "Work".into(),
            width: 1920.0,
            height: 1080.0,
            placements,
        });
        let restored: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(restored.note.text, "unsaved ideas");
        assert_eq!(restored.desktop.saved_layouts[0].placements[0].x, 400.0);
        assert!(restored.widgets["calendar"].locked);
    }
}
