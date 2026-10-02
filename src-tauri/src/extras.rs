use super::*;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockSettings {
    pub hour12: bool,
    pub show_seconds: bool,
    pub cities: Vec<ClockCity>,
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
}
impl Default for NoteSettings {
    fn default() -> Self {
        Self {
            text: String::new(),
            color: "#3b67b8".into(),
        }
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
    if clock.cities.len() > 3
        || clock.cities.iter().any(|c| {
            c.name.trim().is_empty()
                || c.name.chars().count() > 40
                || !zones.contains(&c.time_zone.as_str())
        })
    {
        return Err("最多选择三个支持的城市。".into());
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
) -> Result<AppSnapshot, String> {
    if text.chars().count() > 20000 {
        return Err("便签最多保存 20000 字。".into());
    }
    update_settings(state.inner(), |s| {
        s.note.text = text;
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
        s.note.color = color;
        Ok(())
    })?;
    publish_snapshot(&app, state.inner())
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
    fn custom_note_colors_are_validated_and_persisted() {
        assert_eq!(normalize_note_color("#12ABCD").unwrap(), "#12abcd");
        for invalid in ["#fff", "red", "#12ggff", "#12345678", "#一二三", "url(x)"] {
            assert!(normalize_note_color(invalid).is_err());
        }
        let db = Connection::open_in_memory().unwrap();
        initialize_database(&db).unwrap();
        let mut settings = read_settings(&db).unwrap();
        settings.note.color = normalize_note_color("#12ABCD").unwrap();
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
        settings.note.text = "**想法**\n- 第一项".into();
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
