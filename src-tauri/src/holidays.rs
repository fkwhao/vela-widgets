use super::*;
use std::{
    collections::HashSet,
    io::Read,
    time::{SystemTime, UNIX_EPOCH},
};

const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/fkwhao/vela-widgets/main/data/holidays/manifest.json";
const DATA_URL: &str =
    "https://raw.githubusercontent.com/fkwhao/vela-widgets/main/data/holidays/china.json";
const DAY_MS: i64 = 86_400_000;
const MAX_BYTES: u64 = 1_048_576;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CalendarSettings {
    pub show_holidays: bool,
    pub show_workdays: bool,
    pub auto_update: bool,
}
impl Default for CalendarSettings {
    fn default() -> Self {
        Self {
            show_holidays: true,
            show_workdays: true,
            auto_update: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DayType {
    Holiday,
    Workday,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HolidayDay {
    pub date: String,
    pub name: String,
    pub r#type: DayType,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HolidayYear {
    pub year: u16,
    pub source: String,
    pub days: Vec<HolidayDay>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HolidayData {
    pub schema_version: u8,
    pub revision: u64,
    pub updated_at: String,
    pub years: Vec<HolidayYear>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u8,
    revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HolidayCache {
    pub data: HolidayData,
    pub last_attempt_at: Option<i64>,
    pub last_checked_at: Option<i64>,
    pub last_updated_at: Option<i64>,
    pub last_error: Option<String>,
}
impl Default for HolidayCache {
    fn default() -> Self {
        Self {
            data: serde_json::from_str(include_str!("../../data/holidays/china.json"))
                .expect("bundled holiday data"),
            last_attempt_at: None,
            last_checked_at: None,
            last_updated_at: None,
            last_error: None,
        }
    }
}
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn valid_date(value: &str) -> Option<u16> {
    if value.len() != 10
        || !value.is_ascii()
        || &value[4..5] != "-"
        || &value[7..8] != "-"
        || !value
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return None;
    }
    let year: u16 = value[..4].parse().ok()?;
    let month: usize = value[5..7].parse().ok()?;
    let day: u8 = value[8..].parse().ok()?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    (year >= 2000 && (1..=12).contains(&month) && day >= 1 && day <= month_days[month - 1])
        .then_some(year)
}
fn validate(data: &HolidayData) -> Result<(), String> {
    if data.schema_version != 1
        || data.revision == 0
        || data.revision > 9_007_199_254_740_991
        || valid_date(&data.updated_at).is_none()
        || data.years.is_empty()
        || data.years.len() > 100
    {
        return Err("节假日数据格式不受支持，已保留原有数据。".into());
    }
    let mut years = HashSet::new();
    for year in &data.years {
        let source = reqwest::Url::parse(&year.source).ok();
        if !(2000..=2199).contains(&year.year)
            || !years.insert(year.year)
            || year.days.is_empty()
            || year.days.len() > 366
            || source.as_ref().is_none_or(|u| {
                u.scheme() != "https"
                    || u.host_str()
                        .is_none_or(|h| !(h == "gov.cn" || h.ends_with(".gov.cn")))
            })
        {
            return Err("节假日年份或官方来源无效，已保留原有数据。".into());
        }
        let mut dates = HashSet::new();
        for day in &year.days {
            if valid_date(&day.date) != Some(year.year)
                || !dates.insert(&day.date)
                || day.name.trim().is_empty()
                || day.name.chars().count() > 24
                || day.name.chars().any(char::is_control)
            {
                return Err("节假日日期重复或无效，已保留原有数据。".into());
            }
        }
    }
    Ok(())
}
// A newer release may include only newly announced years. Keep older years offline.
fn merge_data(current: &HolidayData, incoming: HolidayData) -> HolidayData {
    let mut next = incoming;
    for year in &current.years {
        if !next.years.iter().any(|y| y.year == year.year) {
            next.years.push(year.clone());
        }
    }
    next.years.sort_by_key(|year| year.year);
    next
}
fn write_cache(db: &Connection, cache: &HolidayCache) -> rusqlite::Result<()> {
    let document = serde_json::to_string(cache)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    db.execute("INSERT INTO holiday_cache(id,document) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET document=excluded.document", [document])?;
    Ok(())
}
pub fn read_cache(db: &Connection) -> rusqlite::Result<HolidayCache> {
    let document: Option<String> = db
        .query_row("SELECT document FROM holiday_cache WHERE id=1", [], |row| {
            row.get(0)
        })
        .optional()?;
    let mut cache = document
        .and_then(|s| serde_json::from_str::<HolidayCache>(&s).ok())
        .filter(|c| validate(&c.data).is_ok())
        .unwrap_or_default();
    let bundled = HolidayCache::default().data;
    if bundled.revision > cache.data.revision {
        cache.data = merge_data(&cache.data, bundled);
    }
    Ok(cache)
}
pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS holiday_cache(id INTEGER PRIMARY KEY CHECK(id=1),document TEXT NOT NULL);")?;
    write_cache(db, &read_cache(db)?)
}
fn download<T: serde::de::DeserializeOwned>(
    client: &reqwest::blocking::Client,
    url: &str,
) -> Result<T, String> {
    let response = client
        .get(url)
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .send()
        .map_err(|_| "无法连接节假日更新源，请稍后重试；本地数据仍可使用。".to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("节假日更新源尚未发布，本地数据仍可使用。".into());
    }
    if !response.status().is_success() {
        return Err("节假日更新源暂时不可用，本地数据仍可使用。".into());
    }
    let mut bytes = Vec::new();
    response
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "下载没有完成，已保留原有数据。".to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("节假日数据过大，已保留原有数据。".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "节假日数据格式无效，已保留原有数据。".into())
}
fn fetch_update(current: &HolidayData) -> Result<Option<HolidayData>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(8))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Vela-Widgets/Holidays")
        .build()
        .map_err(|_| "无法创建更新连接。".to_string())?;
    fetch_from(current, &client, MANIFEST_URL, DATA_URL)
}
fn fetch_from(
    current: &HolidayData,
    client: &reqwest::blocking::Client,
    manifest_url: &str,
    data_url: &str,
) -> Result<Option<HolidayData>, String> {
    let manifest: Manifest = download(client, manifest_url)?;
    if manifest.schema_version != 1
        || manifest.revision == 0
        || manifest.revision > 9_007_199_254_740_991
    {
        return Err("节假日更新版本无效，已保留原有数据。".into());
    }
    if manifest.revision < current.revision {
        return Err("更新源版本较旧，已保留原有数据。".into());
    }
    if manifest.revision == current.revision {
        return Ok(None);
    }
    let data: HolidayData = download(client, data_url)?;
    validate(&data)?;
    if data.revision != manifest.revision {
        return Err("更新文件版本尚未同步，请稍后重试。".into());
    }
    Ok(Some(data))
}
fn finish_check(
    db: &Connection,
    result: Result<Option<HolidayData>, String>,
    now: i64,
) -> Result<(), String> {
    let mut latest = read_cache(db).map_err(|e| e.to_string())?;
    match result {
        Ok(next) => {
            if let Some(data) = next {
                latest.data = merge_data(&latest.data, data);
                latest.last_updated_at = Some(now);
            }
            latest.last_checked_at = Some(now);
            latest.last_error = None;
        }
        Err(error) => latest.last_error = Some(error),
    }
    write_cache(db, &latest).map_err(|e| e.to_string())
}
fn check(app: &AppHandle, automatic: bool) -> Result<AppSnapshot, String> {
    let state = app.state::<AppState>();
    let _guard = state
        .holiday_update
        .try_lock()
        .map_err(|_| "正在检查节假日更新，请稍候。".to_string())?;
    let cache = {
        let db = lock_database(&state)?;
        let mut cache = read_cache(&db).map_err(|e| e.to_string())?;
        if automatic {
            let settings = read_settings(&db).map_err(|e| e.to_string())?;
            if !settings.calendar.auto_update || !update_due(cache.last_attempt_at, now_ms()) {
                return read_snapshot(&db).map_err(|e| e.to_string());
            }
        }
        cache.last_attempt_at = Some(now_ms());
        write_cache(&db, &cache).map_err(|e| e.to_string())?;
        cache
    };
    let result = fetch_update(&cache.data);
    // Network I/O happens without the database lock, so desktop edits stay responsive.
    {
        let db = lock_database(&state)?;
        finish_check(&db, result, now_ms())?;
    }
    publish_snapshot(app, &state)
}
fn update_due(last: Option<i64>, now: i64) -> bool {
    last.is_none_or(|last| now < last || now - last >= DAY_MS)
}

#[tauri::command]
pub fn set_calendar_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    calendar: CalendarSettings,
) -> Result<AppSnapshot, String> {
    let enabled = calendar.auto_update;
    update_settings(&state, |s| {
        s.calendar = calendar;
        Ok(())
    })?;
    let snapshot = publish_snapshot(&app, &state)?;
    if enabled {
        thread::spawn(move || {
            let _ = check(&app, true);
        });
    }
    Ok(snapshot)
}
#[tauri::command]
pub async fn check_holiday_updates(app: AppHandle) -> Result<AppSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || check(&app, false))
        .await
        .map_err(|_| "更新检查被中断，请重试。".to_string())?
}
pub fn start_update_worker(app: AppHandle) {
    thread::spawn(move || loop {
        let _ = check(&app, true);
        thread::sleep(Duration::from_secs(60));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_official_schedule_has_exact_days() {
        let data = HolidayCache::default().data;
        validate(&data).unwrap();
        let year = &data.years[0];
        assert_eq!(
            year.days
                .iter()
                .filter(|d| d.r#type == DayType::Holiday)
                .count(),
            33
        );
        assert_eq!(
            year.days
                .iter()
                .filter(|d| d.r#type == DayType::Workday)
                .count(),
            6
        );
        assert_eq!(
            year.days
                .iter()
                .find(|d| d.date == "2026-10-10")
                .unwrap()
                .r#type,
            DayType::Workday
        );
        assert!(!year.days.iter().any(|d| d.date == "2026-10-11"));
    }
    #[test]
    fn malformed_or_duplicate_dates_are_rejected() {
        let mut data = HolidayCache::default().data;
        data.years[0].days[0].date = "2026-02-29".into();
        assert!(validate(&data).is_err());
        data = HolidayCache::default().data;
        let duplicate = data.years[0].days[0].clone();
        data.years[0].days.push(duplicate);
        assert!(validate(&data).is_err());
        assert_eq!(valid_date("2028-02-29"), Some(2028));
        assert_eq!(valid_date("🦀🦀xx"), None);
    }
    #[test]
    fn cache_survives_restart_and_bundle_keeps_newer_data() {
        let db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        let mut cache = read_cache(&db).unwrap();
        cache.data.revision += 1;
        cache.last_error = Some("offline".into());
        write_cache(&db, &cache).unwrap();
        initialize(&db).unwrap();
        let loaded = read_cache(&db).unwrap();
        assert_eq!(loaded.data.revision, cache.data.revision);
        assert_eq!(loaded.last_error.as_deref(), Some("offline"));
        db.execute("UPDATE holiday_cache SET document='invalid'", [])
            .unwrap();
        assert_eq!(
            read_cache(&db).unwrap().data.revision,
            HolidayCache::default().data.revision
        );
    }
    #[test]
    fn updates_preserve_previous_years_and_throttle_failed_attempts() {
        let old = HolidayCache::default().data;
        let mut next = old.clone();
        next.revision += 1;
        next.years[0].year = 2027;
        let merged = merge_data(&old, next);
        assert_eq!(merged.years.len(), 2);
        assert!(!update_due(Some(10), DAY_MS));
        assert!(update_due(Some(10), DAY_MS + 10));
        assert!(update_due(None, 0));
    }
    #[test]
    fn failure_preserves_data_and_success_clears_error_without_losing_timestamps() {
        let db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        let old = read_cache(&db).unwrap().data;
        finish_check(&db, Err("offline".into()), 1).unwrap();
        let failed = read_cache(&db).unwrap();
        assert_eq!(failed.data.revision, old.revision);
        assert_eq!(failed.last_checked_at, None);
        finish_check(&db, Ok(None), 2).unwrap();
        let checked = read_cache(&db).unwrap();
        assert_eq!(checked.last_error, None);
        assert_eq!(checked.last_checked_at, Some(2));
        assert_eq!(checked.last_updated_at, None);
        let mut next = old.clone();
        next.revision += 1;
        finish_check(&db, Ok(Some(next)), 3).unwrap();
        let updated = read_cache(&db).unwrap();
        assert_eq!(updated.last_updated_at, Some(3));
        assert_eq!(updated.data.revision, old.revision + 1);
    }
    fn serve(responses: Vec<String>) -> (String, thread::JoinHandle<()>) {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            for body in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = [0; 4096];
                let _ = stream.read(&mut request).unwrap();
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            }
        });
        (base, worker)
    }
    #[test]
    fn http_manifest_skips_unchanged_payload_and_new_version_downloads() {
        let current = HolidayCache::default().data;
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let (base, worker) = serve(vec![format!(
            "{{\"schemaVersion\":1,\"revision\":{}}}",
            current.revision
        )]);
        assert!(fetch_from(
            &current,
            &client,
            &format!("{base}/manifest.json"),
            &format!("{base}/china.json")
        )
        .unwrap()
        .is_none());
        worker.join().unwrap();
        let mut next = current.clone();
        next.revision += 1;
        let (base, worker) = serve(vec![
            format!("{{\"schemaVersion\":1,\"revision\":{}}}", next.revision),
            serde_json::to_string(&next).unwrap(),
        ]);
        assert_eq!(
            fetch_from(
                &current,
                &client,
                &format!("{base}/manifest.json"),
                &format!("{base}/china.json")
            )
            .unwrap()
            .unwrap()
            .revision,
            next.revision
        );
        worker.join().unwrap();
    }
}
