use super::*;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockSettings {
    #[serde(default)]
    pub theme: ClockTheme,
    pub hour12: bool,
    pub show_seconds: bool,
    pub cities: Vec<ClockCity>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClockTheme {
    #[default]
    Default,
    Classic,
    Digital,
    World,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockCity {
    pub name: String,
    pub time_zone: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSettings {
    pub text: String,
    pub color: String,
    #[serde(default)]
    pub notes: Vec<NoteItem>,
    #[serde(default)]
    pub active_id: Option<i64>,
    #[serde(default)]
    pub default_delete_after_hours: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteItem {
    pub id: i64,
    pub text: String,
    pub color: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub delete_after_hours: Option<u32>,
    #[serde(default)]
    pub retention_override: bool,
}
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
fn cleanup_expired(connection: &Connection, now: i64) -> Result<Option<AppSnapshot>, String> {
    let document: String = connection
        .query_row("SELECT document FROM app_settings WHERE id=1", [], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    let mut settings: Settings = serde_json::from_str(&document).map_err(|e| e.to_string())?;
    let count = settings.note.notes.len();
    settings.note.normalize();
    settings.note.expire(now);
    if settings.note.notes.len() == count {
        return Ok(None);
    }
    write_settings(connection, &settings).map_err(|e| e.to_string())?;
    read_snapshot(connection)
        .map(Some)
        .map_err(|e| e.to_string())
}
pub fn start_note_expiry_worker(app: AppHandle) {
    // One quiet sweep per minute also handles notes whose desktop window is disabled.
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
        let state = app.state::<AppState>();
        let result = lock_database(state.inner())
            .and_then(|connection| cleanup_expired(&connection, now_ms()));
        if let Ok(Some(snapshot)) = result {
            let _ = app.emit("vela://snapshot-updated", snapshot);
        }
    });
}
impl Default for NoteSettings {
    fn default() -> Self {
        Self {
            text: String::new(),
            color: "#3b67b8".into(),
            notes: Vec::new(),
            active_id: None,
            default_delete_after_hours: None,
        }
    }
}
impl NoteSettings {
    pub fn normalize(&mut self) {
        for note in &mut self.notes {
            if note.delete_after_hours.is_some() {
                note.retention_override = true;
            }
        }
        if self.active_id.is_none() {
            self.notes.push(NoteItem {
                id: 1,
                text: self.text.clone(),
                color: self.color.clone(),
                created_at: now_ms(),
                delete_after_hours: None,
                retention_override: false,
            });
            self.active_id = Some(1);
        }
        if !self.notes.iter().any(|n| Some(n.id) == self.active_id) {
            self.active_id = Some(self.notes.first().map_or(0, |n| n.id));
        }
        if let Some(note) = self.notes.iter().find(|n| Some(n.id) == self.active_id) {
            self.text = note.text.clone();
            self.color = note.color.clone();
        } else {
            self.text.clear();
        }
    }
    fn next_id(&self) -> i64 {
        let clock = now_ms();
        clock.max(self.notes.iter().map(|n| n.id).max().unwrap_or(0) + 1)
    }
    pub fn expire(&mut self, now: i64) {
        for note in &mut self.notes {
            if note.created_at == 0 {
                note.created_at = now;
            }
        }
        let default_hours = self.default_delete_after_hours;
        self.notes.retain(|n| {
            (if n.retention_override || n.delete_after_hours.is_some() {
                n.delete_after_hours
            } else {
                default_hours
            })
            .is_none_or(|h| now < n.created_at.saturating_add(i64::from(h) * 3600000))
        });
        self.normalize();
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountdownItem {
    pub id: i64,
    pub title: String,
    pub date: String,
    pub yearly: bool,
    pub count_up: bool,
    pub created_date: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountdownInput {
    pub id: Option<i64>,
    pub title: String,
    pub date: String,
    pub yearly: bool,
    pub count_up: bool,
    pub created_date: String,
}

pub fn initialize(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch("CREATE TABLE IF NOT EXISTS countdowns (
      id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, date TEXT NOT NULL,
      yearly INTEGER NOT NULL DEFAULT 0, count_up INTEGER NOT NULL DEFAULT 0, created_date TEXT NOT NULL
    );")
}
pub fn read_countdowns(connection: &Connection) -> rusqlite::Result<Vec<CountdownItem>> {
    let mut statement = connection.prepare(
        "SELECT id, title, date, yearly, count_up, created_date FROM countdowns ORDER BY id ASC",
    )?;
    let rows = statement.query_map([], |r| {
        Ok(CountdownItem {
            id: r.get(0)?,
            title: r.get(1)?,
            date: r.get(2)?,
            yearly: r.get(3)?,
            count_up: r.get(4)?,
            created_date: r.get(5)?,
        })
    })?;
    rows.collect()
}
fn valid_date(value: &str) -> bool {
    if value.len() != 10 || value.as_bytes()[4] != b'-' || value.as_bytes()[7] != b'-' {
        return false;
    }
    if !value
        .bytes()
        .enumerate()
        .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let y: u32 = value[0..4].parse().unwrap_or(0);
    let m: u32 = value[5..7].parse().unwrap_or(0);
    let d: u32 = value[8..10].parse().unwrap_or(0);
    let days = match m {
        2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    y >= 1 && d >= 1 && d <= days
}
fn write_countdown(connection: &Connection, item: &CountdownInput) -> Result<(), String> {
    let title = item.title.trim();
    if title.is_empty()
        || title.chars().count() > 80
        || !valid_date(&item.date)
        || !valid_date(&item.created_date)
    {
        return Err("请输入有效的日期和 1–80 字的事件名称。".into());
    }
    if let Some(id) = item.id {
        let changed = connection
            .execute(
                "UPDATE countdowns SET title=?1, date=?2, yearly=?3, count_up=?4 WHERE id=?5",
                params![title, item.date, item.yearly, item.count_up, id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("这个事件已被删除。".into());
        }
    } else {
        connection.execute("INSERT INTO countdowns (title,date,yearly,count_up,created_date) VALUES (?1,?2,?3,?4,?5)", params![title, item.date, item.yearly, item.count_up, item.created_date]).map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
pub fn set_clock_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    clock: ClockSettings,
) -> Result<AppSnapshot, String> {
    let zones = [
        "Asia/Shanghai",
        "Asia/Tokyo",
        "Asia/Singapore",
        "Asia/Dubai",
        "Europe/London",
        "Europe/Paris",
        "America/New_York",
        "America/Los_Angeles",
        "Australia/Sydney",
        "Pacific/Auckland",
    ];
    if clock.cities.len() > 4
        || clock.cities.iter().any(|c| {
            c.name.trim().is_empty()
                || c.name.chars().count() > 40
                || !zones.contains(&c.time_zone.as_str())
        })
    {
        return Err("最多选择四个支持的城市。".into());
    }
    update_settings(state.inner(), |s| {
        s.clock = clock;
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn save_note(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    id: i64,
) -> Result<AppSnapshot, String> {
    if text.chars().count() > 20000 {
        return Err("便签最多保存 20000 字。".into());
    }
    update_settings(state.inner(), |s| {
        let note = s
            .note
            .notes
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or("这个便签已被删除。")?;
        note.text = text;
        s.note.normalize();
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}
fn normalize_note_color(color: &str) -> Result<String, String> {
    if color.len() != 7
        || !color.starts_with('#')
        || !color.bytes().skip(1).all(|c| c.is_ascii_hexdigit())
    {
        return Err("请选择有效的颜色。".into());
    }
    Ok(color.to_ascii_lowercase())
}
#[tauri::command]
pub fn set_note_color(
    app: AppHandle,
    state: State<'_, AppState>,
    color: String,
) -> Result<AppSnapshot, String> {
    let color = normalize_note_color(&color)?;
    update_settings(state.inner(), |s| {
        if let Some(note) = s
            .note
            .notes
            .iter_mut()
            .find(|n| Some(n.id) == s.note.active_id)
        {
            note.color = color.clone();
        }
        s.note.color = color;
        s.note.normalize();
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn create_note(app: AppHandle, state: State<'_, AppState>) -> Result<AppSnapshot, String> {
    update_settings(state.inner(), |s| {
        if s.note.notes.len() >= 200 {
            return Err("最多保存 200 篇便签。".into());
        }
        let id = s.note.next_id();
        s.note.notes.push(NoteItem {
            id,
            text: String::new(),
            color: s.note.color.clone(),
            created_at: now_ms(),
            delete_after_hours: None,
            retention_override: false,
        });
        s.note.active_id = Some(id);
        s.note.normalize();
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn select_note(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<AppSnapshot, String> {
    update_settings(state.inner(), |s| {
        if !s.note.notes.iter().any(|n| n.id == id) {
            return Err("这个便签已被删除。".into());
        }
        s.note.active_id = Some(id);
        s.note.normalize();
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn delete_note(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<AppSnapshot, String> {
    update_settings(state.inner(), |s| {
        let index = s
            .note
            .notes
            .iter()
            .position(|n| n.id == id)
            .ok_or("这个便签已被删除。")?;
        s.note.notes.remove(index);
        if s.note.active_id == Some(id) {
            s.note.active_id = Some(
                s.note
                    .notes
                    .get(index)
                    .or_else(|| s.note.notes.last())
                    .map_or(0, |n| n.id),
            );
        }
        s.note.normalize();
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn restore_note(
    app: AppHandle,
    state: State<'_, AppState>,
    item: NoteItem,
) -> Result<AppSnapshot, String> {
    let color = normalize_note_color(&item.color)?;
    if item.text.chars().count() > 20000 {
        return Err("便签最多保存 20000 字。".into());
    }
    update_settings(state.inner(), |s| {
        if s.note.notes.len() >= 200 {
            return Err("最多保存 200 篇便签。".into());
        }
        let id = s.note.next_id();
        // Undo preserves the creation date and explicitly disables expiry.
        s.note.notes.push(NoteItem {
            id,
            text: item.text,
            color,
            created_at: item.created_at,
            delete_after_hours: None,
            retention_override: true,
        });
        s.note.active_id = Some(id);
        s.note.normalize();
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn set_note_expiry(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    hours: Option<u32>,
    inherit: Option<bool>,
) -> Result<AppSnapshot, String> {
    if hours.is_some_and(|h| h == 0 || h > 87600) {
        return Err("请输入 1–87600 小时。".into());
    }
    update_settings(state.inner(), |s| {
        let note = s
            .note
            .notes
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or("这个便签已被删除。")?;
        note.retention_override = !inherit.unwrap_or(false);
        note.delete_after_hours = if note.retention_override { hours } else { None };
        s.note.expire(now_ms());
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn set_note_default_expiry(
    app: AppHandle,
    state: State<'_, AppState>,
    hours: Option<u32>,
) -> Result<AppSnapshot, String> {
    if hours.is_some_and(|h| h == 0 || h > 87600) {
        return Err("请输入 1–87600 小时。".into());
    }
    update_settings(state.inner(), |s| {
        s.note.default_delete_after_hours = hours;
        s.note.expire(now_ms());
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn open_note_link(url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") && !url.starts_with("mailto:") {
        return Err("不支持打开这个链接。".into());
    }
    if url.contains(['\0', '\r', '\n']) {
        return Err("无效的链接。".into());
    }
    #[cfg(target_os = "windows")]
    {
        use windows::{
            core::{w, PCWSTR},
            Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        };
        let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
        let result = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(wide.as_ptr()),
                None,
                None,
                SW_SHOWNORMAL,
            )
        };
        if result.0 as usize <= 32 {
            return Err("链接未能打开。".into());
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("请在 Windows 上打开链接。".into())
    }
}
#[tauri::command]
pub fn save_countdown(
    app: AppHandle,
    state: State<'_, AppState>,
    item: CountdownInput,
) -> Result<AppSnapshot, String> {
    {
        let connection = lock_database(state.inner())?;
        write_countdown(&connection, &item)?;
    }
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn delete_countdown(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<AppSnapshot, String> {
    {
        let connection = lock_database(state.inner())?;
        connection
            .execute("DELETE FROM countdowns WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
    }
    publish_snapshot(&app, state.inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn background_cleanup_persists_expiration_even_with_note_window_disabled() {
        let db = Connection::open_in_memory().unwrap();
        initialize_database(&db).unwrap();
        let mut settings = read_settings(&db).unwrap();
        assert!(!settings.widgets["note"].enabled);
        settings.note.notes[0].text = "到期内容".into();
        settings.note.notes[0].created_at = 1000;
        settings.note.notes[0].delete_after_hours = Some(24);
        write_settings(&db, &settings).unwrap();
        assert!(cleanup_expired(&db, 1000 + 24 * 3600000 - 1)
            .unwrap()
            .is_none());
        let cleaned = cleanup_expired(&db, 1000 + 24 * 3600000).unwrap().unwrap();
        assert!(cleaned.settings.note.notes.is_empty());
        assert!(read_snapshot(&db).unwrap().settings.note.notes.is_empty());
        assert!(cleanup_expired(&db, now_ms()).unwrap().is_none());
    }
    #[test]
    fn legacy_note_migration_preserves_text_color_and_creation_time() {
        let db = Connection::open_in_memory().unwrap();
        initialize_database(&db).unwrap();
        let mut old = serde_json::to_value(Settings::default()).unwrap();
        old["note"] = serde_json::json!({ "text": "# 旧便签\n正文", "color": "#12abcd" });
        db.execute(
            "UPDATE app_settings SET document=?1 WHERE id=1",
            [old.to_string()],
        )
        .unwrap();
        let first = read_settings(&db).unwrap().note;
        let reloaded = read_settings(&db).unwrap().note;
        assert_eq!(first.notes.len(), 1);
        assert_eq!(first.notes[0].text, "# 旧便签\n正文");
        assert_eq!(first.notes[0].color, "#12abcd");
        assert!(first.notes[0].created_at > 0);
        assert_eq!(reloaded.notes[0].created_at, first.notes[0].created_at);
    }
    #[test]
    fn expiration_uses_creation_time_and_does_not_recreate_deleted_notes() {
        let mut note = NoteSettings::default();
        note.normalize();
        note.notes[0].created_at = 1000;
        note.notes[0].delete_after_hours = Some(24);
        note.notes.push(NoteItem {
            id: 2,
            text: "永久保存".into(),
            color: "#3b67b8".into(),
            created_at: 1000,
            delete_after_hours: None,
            retention_override: false,
        });
        note.expire(1000 + 24 * 3600000 - 1);
        assert_eq!(note.notes.len(), 2);
        note.expire(1000 + 24 * 3600000);
        assert_eq!(note.notes.len(), 1);
        assert_eq!(note.active_id, Some(2));
        note.notes.clear();
        note.normalize();
        assert_eq!(note.active_id, Some(0));
        assert_eq!(note.text, "");
        let mut reloaded: NoteSettings =
            serde_json::from_str(&serde_json::to_string(&note).unwrap()).unwrap();
        reloaded.normalize();
        assert!(reloaded.notes.is_empty());
    }
    #[test]
    fn multiple_notes_keep_independent_text_and_colors_after_reload() {
        let db = Connection::open_in_memory().unwrap();
        initialize_database(&db).unwrap();
        let mut settings = read_settings(&db).unwrap();
        settings.note.notes[0].text = "# 工作".into();
        let id = settings.note.next_id();
        settings.note.notes.push(NoteItem {
            id,
            text: "# 生活".into(),
            color: "#12abcd".into(),
            created_at: now_ms(),
            delete_after_hours: Some(168),
            retention_override: true,
        });
        settings.note.active_id = Some(id);
        settings.note.normalize();
        write_settings(&db, &settings).unwrap();
        let reloaded = read_snapshot(&db).unwrap().settings.note;
        assert_eq!(reloaded.notes[0].text, "# 工作");
        assert_eq!(reloaded.text, "# 生活");
        assert_eq!(reloaded.color, "#12abcd");
        assert_eq!(reloaded.notes[1].delete_after_hours, Some(168));
    }
    #[test]
    fn default_retention_preserves_overrides_and_legacy_durations() {
        let mut settings: NoteSettings = serde_json::from_value(serde_json::json!({
            "text": "", "color": "#3b67b8", "activeId": 1,
            "defaultDeleteAfterHours": 24,
            "notes": [
                {"id":1,"text":"继承","color":"#3b67b8","createdAt":1000},
                {"id":2,"text":"永久","color":"#3b67b8","createdAt":1000,"retentionOverride":true},
                {"id":3,"text":"旧独立","color":"#3b67b8","createdAt":1000,"deleteAfterHours":168}
            ]
        }))
        .unwrap();
        settings.normalize();
        assert!(settings.notes[2].retention_override);
        settings.expire(1000 + 24 * 3600000);
        assert_eq!(
            settings.notes.iter().map(|n| n.id).collect::<Vec<_>>(),
            vec![2, 3]
        );
        settings.default_delete_after_hours = Some(1);
        settings.expire(1000 + 25 * 3600000);
        assert_eq!(settings.notes.len(), 2);
        settings.notes[1].retention_override = false;
        settings.notes[1].delete_after_hours = None;
        settings.expire(1000 + 25 * 3600000);
        assert_eq!(settings.notes.len(), 1);
        assert_eq!(settings.notes[0].id, 2);
    }
    #[test]
    fn global_expiry_is_persisted_by_background_cleanup() {
        let db = Connection::open_in_memory().unwrap();
        initialize_database(&db).unwrap();
        let mut settings = read_settings(&db).unwrap();
        settings.note.default_delete_after_hours = Some(1);
        settings.note.notes[0].created_at = 1000;
        write_settings(&db, &settings).unwrap();
        assert!(cleanup_expired(&db, 1000 + 3600000)
            .unwrap()
            .unwrap()
            .settings
            .note
            .notes
            .is_empty());
        assert!(read_settings(&db).unwrap().note.notes.is_empty());
    }
    #[test]
    fn custom_note_colors_are_validated_and_persisted() {
        assert_eq!(normalize_note_color("#12ABCD").unwrap(), "#12abcd");
        for invalid in ["#fff", "red", "#12ggff", "#12345678", "#一二三", "url(x)"] {
            assert!(normalize_note_color(invalid).is_err());
        }
        let db = Connection::open_in_memory().unwrap();
        initialize_database(&db).unwrap();
        let mut settings = read_settings(&db).unwrap();
        settings.note.notes[0].color = normalize_note_color("#12ABCD").unwrap();
        settings.note.normalize();
        write_settings(&db, &settings).unwrap();
        assert_eq!(read_settings(&db).unwrap().note.color, "#12abcd");
    }
    #[test]
    fn dates_reject_impossible_days() {
        assert!(valid_date("2024-02-29"));
        assert!(!valid_date("2025-02-29"));
        assert!(!valid_date("2026-04-31"));
        assert!(!valid_date("0000-01-01"));
        assert!(!valid_date("日期无效"));
    }
    #[test]
    fn clock_themes_migrate_and_round_trip_with_custom_cities() {
        let legacy: ClockSettings = serde_json::from_value(serde_json::json!({
            "hour12": false, "showSeconds": true,
            "cities": [{"name":"东京", "timeZone":"Asia/Tokyo"}]
        }))
        .unwrap();
        assert!(matches!(legacy.theme, ClockTheme::Default));
        assert_eq!(legacy.cities[0].time_zone, "Asia/Tokyo");
        for theme in ["default", "digital", "classic", "world"] {
            let mut value = serde_json::to_value(&legacy).unwrap();
            value["theme"] = serde_json::json!(theme);
            let clock: ClockSettings = serde_json::from_value(value).unwrap();
            let stored = serde_json::to_value(clock).unwrap();
            assert_eq!(stored["theme"], theme);
            assert_eq!(stored["cities"][0]["name"], "东京");
        }
        let mut invalid = serde_json::to_value(&legacy).unwrap();
        invalid["theme"] = serde_json::json!("unknown");
        assert!(serde_json::from_value::<ClockSettings>(invalid).is_err());
    }

    #[test]
    fn old_settings_keep_existing_widgets_and_gain_disabled_defaults() {
        let db = Connection::open_in_memory().unwrap();
        initialize_database(&db).unwrap();
        let mut old = serde_json::to_value(Settings::default()).unwrap();
        old.as_object_mut().unwrap().remove("clock");
        old.as_object_mut().unwrap().remove("note");
        for kind in ["clock", "note", "countdown"] {
            old["widgets"].as_object_mut().unwrap().remove(kind);
        }
        old["widgets"]["calendar"]["x"] = serde_json::json!(123);
        db.execute(
            "UPDATE app_settings SET document=?1 WHERE id=1",
            [old.to_string()],
        )
        .unwrap();
        let restored = read_settings(&db).unwrap();
        assert_eq!(restored.widgets["calendar"].x, Some(123.0));
        assert_eq!(restored.widgets.len(), 5);
        assert!(!restored.widgets["note"].enabled);
    }
    #[test]
    fn countdown_and_note_survive_snapshot_reload() {
        let db = Connection::open_in_memory().unwrap();
        initialize_database(&db).unwrap();
        let mut input = CountdownInput {
            id: None,
            title: "  旅行  ".into(),
            date: "2026-12-01".into(),
            yearly: false,
            count_up: false,
            created_date: "2026-10-02".into(),
        };
        write_countdown(&db, &input).unwrap();
        let row = read_countdowns(&db).unwrap().remove(0);
        assert_eq!(row.title, "旅行");
        input.id = Some(row.id);
        input.title = "纪念日".into();
        input.yearly = true;
        write_countdown(&db, &input).unwrap();
        assert!(read_countdowns(&db).unwrap()[0].yearly);
        let mut settings = read_settings(&db).unwrap();
        settings.note.notes[0].text = "**想法**\n- 第一项".into();
        settings.note.normalize();
        write_settings(&db, &settings).unwrap();
        assert_eq!(
            read_snapshot(&db).unwrap().settings.note.text,
            settings.note.text
        );
        input.date = "2026-02-30".into();
        assert!(write_countdown(&db, &input).is_err());
        db.execute("DELETE FROM countdowns WHERE id=?1", [row.id])
            .unwrap();
        assert!(read_countdowns(&db).unwrap().is_empty());
    }
}
