use crate::{
    lock_database, publish_snapshot, read_settings, write_settings, AppSnapshot, AppState,
};
use chrono::{Datelike, Local, NaiveDate, NaiveTime, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitSettings {
    pub style: String,
    pub selected_ids: Option<Vec<i64>>,
}
impl Default for HabitSettings {
    fn default() -> Self {
        Self {
            style: "card".into(),
            selected_ids: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitItem {
    pub id: i64,
    pub title: String,
    pub encouragement: String,
    pub icon: String,
    pub color: String,
    pub start_date: String,
    pub weekdays: Vec<u32>,
    pub daily_target: i32,
    pub goal_days: Option<i32>,
    pub archived: bool,
    pub sort_order: i32,
    pub track_mood: bool,
    pub track_rating: bool,
    pub track_result: bool,
    #[serde(default)]
    pub reminder_time: Option<String>,
    #[serde(default)]
    pub last_reminded_date: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitRecord {
    pub habit_id: i64,
    pub date: String,
    pub count: i32,
    pub target: i32,
    pub updated_at: String,
    pub mood: Option<String>,
    pub rating: Option<u8>,
    pub result: Option<f64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordInput {
    pub habit_id: i64,
    pub date: String,
    pub delta: i32,
    #[serde(default)]
    pub capture_metadata: bool,
    pub mood: Option<String>,
    pub rating: Option<u8>,
    pub result: Option<f64>,
}

pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS habits (id INTEGER PRIMARY KEY AUTOINCREMENT, document TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS habit_records (habit_id INTEGER NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
        date TEXT NOT NULL, document TEXT NOT NULL, PRIMARY KEY(habit_id,date));")
}
fn decode<T: serde::de::DeserializeOwned>(document: String) -> rusqlite::Result<T> {
    serde_json::from_str(&document).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}
pub fn read_habits(db: &Connection) -> rusqlite::Result<Vec<HabitItem>> {
    let mut stmt = db.prepare("SELECT document FROM habits ORDER BY id")?;
    let mut items: Vec<HabitItem> = stmt
        .query_map([], |r| decode(r.get(0)?))?
        .collect::<rusqlite::Result<_>>()?;
    items.sort_by_key(|h| (h.sort_order, h.id));
    Ok(items)
}
pub fn read_records(db: &Connection) -> rusqlite::Result<Vec<HabitRecord>> {
    let mut stmt = db.prepare("SELECT document FROM habit_records ORDER BY date,habit_id")?;
    let rows = stmt.query_map([], |r| decode(r.get(0)?))?;
    rows.collect()
}
fn parse_date(value: &str) -> Result<NaiveDate, String> {
    let date =
        NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| "请选择有效日期。".to_string())?;
    if date.format("%Y-%m-%d").to_string() != value {
        return Err("请选择有效日期。".into());
    }
    Ok(date)
}
fn validate(item: &HabitItem) -> Result<(), String> {
    if item.title.trim().is_empty() || item.title.trim().chars().count() > 80 {
        return Err("习惯名称需为 1–80 个字符。".into());
    }
    if item.encouragement.chars().count() > 160
        || item.icon.trim().is_empty()
        || item.icon.chars().count() > 16
        || item.color.len() != 7
        || !item.color.starts_with('#')
        || !item.color[1..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("请检查鼓励语、图标和颜色。".into());
    }
    let start = parse_date(&item.start_date)?;
    if !(1900..=2100).contains(&start.year()) {
        return Err("请选择 1900–2100 年之间的开始日期。".into());
    }
    if item.weekdays.is_empty() || item.weekdays.iter().any(|d| *d > 6) {
        return Err("至少选择一个执行日。".into());
    }
    if !(1..=99).contains(&item.daily_target)
        || item.goal_days.is_some_and(|n| !(1..=10000).contains(&n))
    {
        return Err("请检查每日次数和坚持目标。".into());
    }
    Ok(())
}
fn write_habit(db: &Connection, item: &mut HabitItem) -> Result<(), String> {
    validate(item)?;
    item.title = item.title.trim().into();
    if let Some(time) = &item.reminder_time {
        if time.len() != 5 || NaiveTime::parse_from_str(time, "%H:%M").is_err() {
            return Err("请输入有效提醒时间。".into());
        }
    }
    item.last_reminded_date = if item.id == 0 {
        None
    } else {
        read_habits(db)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|h| h.id == item.id)
            .and_then(|h| h.last_reminded_date)
    };
    if item.id == 0 {
        item.sort_order = read_habits(db)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|h| h.sort_order)
            .max()
            .unwrap_or(-1)
            + 1;
        db.execute("INSERT INTO habits(document) VALUES ('{}')", [])
            .map_err(|e| e.to_string())?;
        item.id = db.last_insert_rowid();
    }
    let doc = serde_json::to_string(item).map_err(|e| e.to_string())?;
    if db
        .execute(
            "UPDATE habits SET document=?1 WHERE id=?2",
            params![doc, item.id],
        )
        .map_err(|e| e.to_string())?
        == 0
    {
        return Err("找不到这个习惯。".into());
    }
    Ok(())
}
fn adjust_record(db: &Connection, input: &RecordInput, today: NaiveDate) -> Result<(), String> {
    let doc: String = db
        .query_row(
            "SELECT document FROM habits WHERE id=?1",
            [input.habit_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or("找不到这个习惯。")?;
    let habit: HabitItem = decode(doc).map_err(|e| e.to_string())?;
    let date = parse_date(&input.date)?;
    let existing: Option<String> = db
        .query_row(
            "SELECT document FROM habit_records WHERE habit_id=?1 AND date=?2",
            params![input.habit_id, input.date],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if date > today
        || (existing.is_none()
            && (input.date < habit.start_date
                || !habit
                    .weekdays
                    .contains(&date.weekday().num_days_from_sunday())))
    {
        return Err("只能记录开始日期之后的执行日，不能提前打卡。".into());
    }
    if !(-1..=1).contains(&input.delta)
        || input.rating.is_some_and(|r| !(1..=5).contains(&r))
        || input
            .result
            .is_some_and(|r| !r.is_finite() || r.abs() > 1e9)
        || input
            .mood
            .as_ref()
            .is_some_and(|m| !["开心", "平静", "疲惫", "低落"].contains(&m.as_str()))
    {
        return Err("请检查打卡记录。".into());
    }
    if existing.is_none() && input.delta <= 0 {
        return Ok(());
    }
    let mut record = match existing {
        Some(doc) => decode::<HabitRecord>(doc).map_err(|e| e.to_string())?,
        None => HabitRecord {
            habit_id: input.habit_id,
            date: input.date.clone(),
            count: 0,
            target: habit.daily_target,
            updated_at: String::new(),
            mood: None,
            rating: None,
            result: None,
        },
    };
    record.count = (record.count + input.delta).clamp(0, record.target);
    // Manager check-ins save metadata atomically; widget increments preserve it.
    if input.delta == 0 || input.capture_metadata {
        record.mood = input.mood.clone();
        record.rating = input.rating;
        record.result = input.result;
    }
    record.updated_at = Utc::now().to_rfc3339();
    if record.count == 0 {
        db.execute(
            "DELETE FROM habit_records WHERE habit_id=?1 AND date=?2",
            params![input.habit_id, input.date],
        )
        .map_err(|e| e.to_string())?;
    } else {
        let document = serde_json::to_string(&record).map_err(|e| e.to_string())?;
        db.execute("INSERT INTO habit_records(habit_id,date,document) VALUES (?1,?2,?3) ON CONFLICT(habit_id,date) DO UPDATE SET document=excluded.document", params![input.habit_id, input.date, document]).map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
pub fn save_habit(
    app: AppHandle,
    state: State<'_, AppState>,
    mut item: HabitItem,
) -> Result<AppSnapshot, String> {
    {
        let db = lock_database(state.inner())?;
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
        write_habit(&tx, &mut item)?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn delete_habit(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<AppSnapshot, String> {
    {
        let db = lock_database(state.inner())?;
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM habits WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        let mut settings = read_settings(&tx).map_err(|e| e.to_string())?;
        if let Some(ids) = &mut settings.habit.selected_ids {
            ids.retain(|i| *i != id);
        }
        write_settings(&tx, &settings).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn adjust_habit_record(
    app: AppHandle,
    state: State<'_, AppState>,
    input: RecordInput,
) -> Result<AppSnapshot, String> {
    {
        let db = lock_database(state.inner())?;
        adjust_record(&db, &input, Local::now().date_naive())?;
    }
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn set_habit_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: HabitSettings,
) -> Result<AppSnapshot, String> {
    {
        let db = lock_database(state.inner())?;
        if !["card", "list", "report"].contains(&settings.style.as_str())
            || settings.selected_ids.as_ref().is_some_and(|ids| {
                read_habits(&db).map_or(true, |habits| {
                    ids.iter().any(|id| !habits.iter().any(|h| h.id == *id))
                })
            })
        {
            return Err("无效的组件设置。".into());
        }
        let mut current = read_settings(&db).map_err(|e| e.to_string())?;
        current.habit = settings;
        write_settings(&db, &current).map_err(|e| e.to_string())?;
    }
    publish_snapshot(&app, state.inner())
}
#[tauri::command]
pub fn reorder_habits(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<i64>,
) -> Result<AppSnapshot, String> {
    {
        let db = lock_database(state.inner())?;
        let mut habits = read_habits(&db).map_err(|e| e.to_string())?;
        if ids.len() != habits.len()
            || habits
                .iter()
                .any(|h| ids.iter().filter(|id| **id == h.id).count() != 1)
        {
            return Err("习惯列表已变化，请重试。".into());
        }
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
        for h in &mut habits {
            h.sort_order = ids.iter().position(|id| *id == h.id).unwrap() as i32;
            write_habit(&tx, h)?;
        }
        tx.commit().map_err(|e| e.to_string())?;
    }
    publish_snapshot(&app, state.inner())
}

// Reuse the clock's native reminder window. The worker checks once per minute;
// only the exact scheduled minute fires, never a backlog of old reminders.
pub fn tick_reminders(
    db: &Connection,
    tools: &mut crate::clock_tools::ClockTools,
    now: i64,
) -> Result<bool, String> {
    let Some(local) = Local.timestamp_millis_opt(now).single() else {
        return Ok(false);
    };
    let day = local.format("%Y-%m-%d").to_string();
    let time = local.format("%H:%M").to_string();
    let habits = read_habits(db).map_err(|e| e.to_string())?;
    let records = read_records(db).map_err(|e| e.to_string())?;
    let mut changed = false;
    for mut h in habits {
        if h.archived
            || h.reminder_time.as_deref() != Some(&time)
            || h.last_reminded_date.as_deref() == Some(&day)
            || h.start_date > day
            || !h.weekdays.contains(&local.weekday().num_days_from_sunday())
            || records
                .iter()
                .any(|r| r.habit_id == h.id && r.date == day && r.count >= r.target)
        {
            continue;
        }
        h.last_reminded_date = Some(day.clone());
        let document = serde_json::to_string(&h).map_err(|e| e.to_string())?;
        db.execute(
            "UPDATE habits SET document=?1 WHERE id=?2",
            params![document, h.id],
        )
        .map_err(|e| e.to_string())?;
        let id = now.max(tools.alerts.iter().map(|a| a.id + 1).max().unwrap_or(0));
        tools.alerts.push(crate::clock_tools::ClockAlert {
            id,
            alarm_id: None,
            label: format!("习惯提醒 · {}", h.title),
            fired_at: now,
            missed: false,
        });
        changed = true;
    }
    if tools.alerts.len() > 32 {
        tools.alerts.drain(..tools.alerts.len() - 32);
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (Connection, HabitItem) {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        initialize(&db).unwrap();
        let mut h = HabitItem {
            id: 0,
            title: "喝水".into(),
            encouragement: "坚持".into(),
            icon: "💧".into(),
            color: "#444fb0".into(),
            start_date: "2026-10-01".into(),
            weekdays: (0..7).collect(),
            daily_target: 3,
            goal_days: None,
            archived: false,
            sort_order: 0,
            track_mood: false,
            track_rating: false,
            track_result: false,
            reminder_time: None,
            last_reminded_date: None,
        };
        write_habit(&db, &mut h).unwrap();
        (db, h)
    }
    #[test]
    fn records_persist_cap_and_keep_original_target() {
        let (db, mut h) = setup();
        let today = parse_date("2026-10-04").unwrap();
        let mut input = RecordInput {
            habit_id: h.id,
            date: "2026-10-04".into(),
            delta: 1,
            capture_metadata: false,
            mood: None,
            rating: None,
            result: None,
        };
        for _ in 0..5 {
            adjust_record(&db, &input, today).unwrap();
        }
        assert_eq!(read_records(&db).unwrap()[0].count, 3);
        h.daily_target = 5;
        write_habit(&db, &mut h).unwrap();
        adjust_record(&db, &input, today).unwrap();
        assert_eq!(read_records(&db).unwrap()[0].target, 3);
        input.delta = 0;
        input.mood = Some("开心".into());
        input.rating = Some(5);
        input.result = Some(12.5);
        adjust_record(&db, &input, today).unwrap();
        let saved = read_records(&db).unwrap().remove(0);
        assert_eq!(saved.mood.as_deref(), Some("开心"));
        assert_eq!(saved.rating, Some(5));
        assert_eq!(saved.result, Some(12.5));
        h.weekdays = vec![1];
        h.start_date = "2026-10-06".into();
        write_habit(&db, &mut h).unwrap();
        input.delta = -1;
        adjust_record(&db, &input, today).unwrap();
        assert_eq!(read_records(&db).unwrap()[0].count, 2);
        assert_eq!(read_records(&db).unwrap()[0].mood.as_deref(), Some("开心"));
        db.execute("DELETE FROM habits WHERE id=?1", [h.id])
            .unwrap();
        assert!(read_records(&db).unwrap().is_empty());
    }
    #[test]
    fn manager_checkin_saves_metadata_atomically_and_widget_preserves_it() {
        let (db, h) = setup();
        let today = parse_date("2026-10-04").unwrap();
        let mut input = RecordInput {
            habit_id: h.id,
            date: "2026-10-04".into(),
            delta: 1,
            capture_metadata: true,
            mood: Some("平静".into()),
            rating: Some(4),
            result: Some(0.0),
        };
        adjust_record(&db, &input, today).unwrap();
        let record = read_records(&db).unwrap().remove(0);
        assert_eq!(record.count, 1);
        assert_eq!(record.mood.as_deref(), Some("平静"));
        assert_eq!(record.rating, Some(4));
        assert_eq!(record.result, Some(0.0));
        input.capture_metadata = false;
        input.mood = None;
        input.rating = None;
        input.result = None;
        adjust_record(&db, &input, today).unwrap();
        let record = read_records(&db).unwrap().remove(0);
        assert_eq!(record.count, 2);
        assert_eq!(record.rating, Some(4));
        assert_eq!(record.result, Some(0.0));
        input.capture_metadata = true;
        input.rating = Some(9);
        assert!(adjust_record(&db, &input, today).is_err());
        assert_eq!(read_records(&db).unwrap()[0].count, 2);
    }
    #[test]
    fn future_rest_days_and_invalid_input_are_rejected() {
        let (db, mut h) = setup();
        h.weekdays = vec![1];
        write_habit(&db, &mut h).unwrap();
        let input = RecordInput {
            habit_id: h.id,
            date: "2026-10-04".into(),
            delta: 1,
            capture_metadata: false,
            mood: None,
            rating: None,
            result: None,
        };
        assert!(adjust_record(&db, &input, parse_date("2026-10-04").unwrap()).is_err());
        let input = RecordInput {
            date: "2026-10-05".into(),
            ..input
        };
        assert!(adjust_record(&db, &input, parse_date("2026-10-04").unwrap()).is_err());
        h.daily_target = 0;
        assert!(validate(&h).is_err());
        assert!(parse_date("2026-02-30").is_err());
    }
    #[test]
    fn reminders_fire_once_and_skip_paused_or_completed_days() {
        let (db, mut h) = setup();
        h.reminder_time = Some("20:00".into());
        write_habit(&db, &mut h).unwrap();
        let now = Local
            .with_ymd_and_hms(2026, 10, 4, 20, 0, 0)
            .unwrap()
            .timestamp_millis();
        let mut tools = crate::clock_tools::ClockTools::default();
        assert!(tick_reminders(&db, &mut tools, now).unwrap());
        assert_eq!(tools.alerts.len(), 1);
        assert!(!tick_reminders(&db, &mut tools, now + 30000).unwrap());
        h.title = "水".into();
        write_habit(&db, &mut h).unwrap();
        assert!(!tick_reminders(&db, &mut tools, now).unwrap());
        let input = RecordInput {
            habit_id: h.id,
            date: "2026-10-05".into(),
            delta: 1,
            capture_metadata: false,
            mood: None,
            rating: None,
            result: None,
        };
        for _ in 0..3 {
            adjust_record(&db, &input, parse_date("2026-10-05").unwrap()).unwrap();
        }
        let next = Local
            .with_ymd_and_hms(2026, 10, 5, 20, 0, 0)
            .unwrap()
            .timestamp_millis();
        assert!(!tick_reminders(&db, &mut tools, next).unwrap());
        h.archived = true;
        write_habit(&db, &mut h).unwrap();
        assert!(!tick_reminders(&db, &mut tools, next + 86400000).unwrap());
    }
}
