use super::*;
use chrono::{Datelike, Local, NaiveDate, NaiveTime, TimeZone};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockTools {
    pub mode: String,
    pub alarms: Vec<Alarm>,
    pub timer: Timer,
    pub stopwatch: Stopwatch,
    pub alerts: Vec<ClockAlert>,
}
impl Default for ClockTools {
    fn default() -> Self {
        Self {
            mode: "clock".into(),
            alarms: vec![],
            timer: Timer::default(),
            stopwatch: Stopwatch::default(),
            alerts: vec![],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alarm {
    pub id: i64,
    pub label: String,
    pub time: String,
    pub date: Option<String>,
    pub weekdays: Vec<u32>,
    pub enabled: bool,
    pub next_at: Option<i64>,
    pub snooze_at: Option<i64>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmInput {
    pub id: Option<i64>,
    pub label: String,
    pub time: String,
    pub date: Option<String>,
    pub weekdays: Vec<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timer {
    pub duration_seconds: u32,
    pub remaining_ms: i64,
    pub deadline: Option<i64>,
}
impl Default for Timer {
    fn default() -> Self {
        Self {
            duration_seconds: 300,
            remaining_ms: 300000,
            deadline: None,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stopwatch {
    pub elapsed_ms: i64,
    pub started_at: Option<i64>,
    pub laps: Vec<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockAlert {
    pub id: i64,
    pub alarm_id: Option<i64>,
    pub label: String,
    pub fired_at: i64,
    pub missed: bool,
}
pub fn now_ms() -> i64 {
    Local::now().timestamp_millis()
}
fn next_alarm(time: &str, date: Option<&str>, weekdays: &[u32], now: i64) -> Result<i64, String> {
    if time.len() != 5 {
        return Err("请输入有效时间。".into());
    }
    let time = NaiveTime::parse_from_str(time, "%H:%M").map_err(|_| "请输入有效时间。")?;
    let today = Local
        .timestamp_millis_opt(now)
        .single()
        .ok_or("无效日期。")?
        .date_naive();
    let fixed = date
        .map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| "请输入有效日期。"))
        .transpose()?;
    for offset in 0..9 {
        let day = fixed.unwrap_or(today + chrono::Duration::days(offset));
        if weekdays.is_empty() || weekdays.contains(&day.weekday().num_days_from_monday()) {
            if let Some(candidate) = Local.from_local_datetime(&day.and_time(time)).earliest() {
                if candidate.timestamp_millis() > now {
                    return Ok(candidate.timestamp_millis());
                }
            }
        }
        if fixed.is_some() {
            break;
        }
    }
    Err("请选择未来的日期和时间。".into())
}
impl ClockTools {
    pub fn tick(&mut self, now: i64) -> bool {
        let mut fired = vec![];
        if let Some(deadline) = self.timer.deadline {
            if now >= deadline {
                self.timer.deadline = None;
                self.timer.remaining_ms = 0;
                fired.push((None, "计时结束".to_string(), deadline));
            }
        }
        for alarm in &mut self.alarms {
            let due = if alarm.enabled {
                alarm.next_at.filter(|t| *t <= now)
            } else {
                None
            };
            let snoozed = alarm.snooze_at.filter(|t| *t <= now);
            if due.is_some() || snoozed.is_some() {
                fired.push((
                    Some(alarm.id),
                    alarm.label.clone(),
                    due.or(snoozed).unwrap(),
                ));
                if snoozed.is_some() {
                    alarm.snooze_at = None;
                }
                if due.is_some() {
                    if alarm.weekdays.is_empty() {
                        alarm.enabled = false;
                        alarm.next_at = None;
                    } else {
                        alarm.next_at = next_alarm(&alarm.time, None, &alarm.weekdays, now).ok();
                    }
                }
            }
        }
        let changed = !fired.is_empty();
        for (alarm_id, label, due) in fired {
            let id = now.max(self.alerts.iter().map(|a| a.id + 1).max().unwrap_or(0));
            self.alerts.push(ClockAlert {
                id,
                alarm_id,
                label,
                fired_at: now,
                missed: now - due > 60000,
            });
        }
        if self.alerts.len() > 32 {
            self.alerts.drain(..self.alerts.len() - 32);
        }
        changed
    }
    fn act(
        &mut self,
        action: &str,
        id: Option<i64>,
        seconds: Option<u32>,
        mode: Option<String>,
        alarm: Option<AlarmInput>,
        now: i64,
    ) -> Result<(), String> {
        match action {
            "mode" => {
                let mode = mode.ok_or("请选择模式。")?;
                if !["clock", "alarm", "stopwatch", "timer"].contains(&mode.as_str()) {
                    return Err("无效模式。".into());
                }
                self.mode = mode;
            }
            "timer-set" => {
                let seconds = seconds
                    .filter(|s| *s >= 1 && *s <= 86400)
                    .ok_or("时长范围为 1 秒至 24 小时。")?;
                self.timer = Timer {
                    duration_seconds: seconds,
                    remaining_ms: i64::from(seconds) * 1000,
                    deadline: None,
                };
            }
            "timer-toggle" => {
                if let Some(deadline) = self.timer.deadline.take() {
                    self.timer.remaining_ms = (deadline - now).max(0);
                } else {
                    if self.timer.remaining_ms <= 0 {
                        self.timer.remaining_ms = i64::from(self.timer.duration_seconds) * 1000;
                    }
                    self.timer.deadline = Some(now + self.timer.remaining_ms);
                }
            }
            "timer-reset" => {
                self.timer.deadline = None;
                self.timer.remaining_ms = i64::from(self.timer.duration_seconds) * 1000;
            }
            "stopwatch-toggle" => {
                if let Some(started) = self.stopwatch.started_at.take() {
                    self.stopwatch.elapsed_ms += (now - started).max(0);
                } else {
                    self.stopwatch.started_at = Some(now);
                }
            }
            "stopwatch-reset" => {
                self.stopwatch = Stopwatch::default();
            }
            "stopwatch-lap" => {
                let started = self.stopwatch.started_at.ok_or("开始秒表后才能计次。")?;
                if self.stopwatch.laps.len() >= 100 {
                    return Err("最多记录 100 次。".into());
                }
                self.stopwatch
                    .laps
                    .push(self.stopwatch.elapsed_ms + (now - started).max(0));
            }
            "alarm-save" => {
                let input = alarm.ok_or("请填写闹钟。")?;
                if input.label.trim().chars().count() > 40 || input.weekdays.iter().any(|d| *d > 6)
                {
                    return Err("闹钟名称最多 40 字，请选择有效重复日期。".into());
                }
                let date = input.date.filter(|d| !d.is_empty());
                if date.is_some() && !input.weekdays.is_empty() {
                    return Err("指定日期不能与每周重复同时使用。".into());
                }
                let next = next_alarm(&input.time, date.as_deref(), &input.weekdays, now)?;
                let id = input.id.unwrap_or_else(|| {
                    now.max(self.alarms.iter().map(|a| a.id + 1).max().unwrap_or(0))
                });
                if input.id.is_some() && !self.alarms.iter().any(|a| a.id == id) {
                    return Err("这个闹钟已被删除。".into());
                }
                if input.id.is_none() && self.alarms.len() >= 32 {
                    return Err("最多保存 32 个闹钟。".into());
                }
                let label = if input.label.trim().is_empty() {
                    "闹钟".into()
                } else {
                    input.label.trim().into()
                };
                let mut weekdays = input.weekdays;
                weekdays.sort();
                weekdays.dedup();
                let item = Alarm {
                    id,
                    label,
                    time: input.time,
                    date,
                    weekdays,
                    enabled: true,
                    next_at: Some(next),
                    snooze_at: None,
                };
                if let Some(old) = self.alarms.iter_mut().find(|a| a.id == id) {
                    *old = item;
                } else {
                    self.alarms.push(item);
                }
            }
            "alarm-toggle" => {
                let alarm = self
                    .alarms
                    .iter_mut()
                    .find(|a| Some(a.id) == id)
                    .ok_or("这个闹钟已被删除。")?;
                if !alarm.enabled {
                    alarm.next_at = Some(next_alarm(
                        &alarm.time,
                        alarm.date.as_deref(),
                        &alarm.weekdays,
                        now,
                    )?);
                } else {
                    alarm.next_at = None;
                    alarm.snooze_at = None;
                }
                alarm.enabled = !alarm.enabled;
            }
            "alarm-delete" => {
                self.alarms.retain(|a| Some(a.id) != id);
                self.alerts.retain(|a| a.alarm_id != id);
            }
            "dismiss" => {
                if id.is_some() {
                    self.alerts.retain(|a| Some(a.id) != id);
                } else {
                    self.alerts.clear();
                }
            }
            "snooze" => {
                let alert = self
                    .alerts
                    .iter()
                    .find(|a| Some(a.id) == id)
                    .ok_or("这条提醒已结束。")?
                    .clone();
                if let Some(alarm_id) = alert.alarm_id {
                    let alarm = self
                        .alarms
                        .iter_mut()
                        .find(|a| a.id == alarm_id)
                        .ok_or("这个闹钟已被删除。")?;
                    alarm.snooze_at = Some(now + 300000);
                } else {
                    self.timer = Timer {
                        duration_seconds: 300,
                        remaining_ms: 300000,
                        deadline: Some(now + 300000),
                    };
                }
                self.alerts.retain(|a| Some(a.id) != id);
            }
            _ => return Err("不支持这个操作。".into()),
        }
        Ok(())
    }
}
#[tauri::command]
pub fn clock_action(
    app: AppHandle,
    state: State<'_, AppState>,
    action: String,
    id: Option<i64>,
    seconds: Option<u32>,
    mode: Option<String>,
    alarm: Option<AlarmInput>,
) -> Result<AppSnapshot, String> {
    update_settings(state.inner(), |s| {
        s.clock_tools.tick(now_ms());
        s.clock_tools
            .act(&action, id, seconds, mode, alarm, now_ms())
    })?;
    let snapshot = publish_snapshot(&app, state.inner())?;
    if snapshot.settings.clock_tools.alerts.is_empty() {
        if let Some(window) = app.get_webview_window("clock-reminder") {
            let _ = window.hide();
        }
    }
    if !snapshot.settings.clock_tools.alerts.is_empty() {
        show_reminder(&app);
    }
    Ok(snapshot)
}
fn show_reminder(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("clock-reminder") {
            let _ = window.show();
            let _ = window.set_focus();
            return;
        }
        let Ok(directory) = webview_data_path(&handle) else {
            return;
        };
        let _ = WebviewWindowBuilder::new(
            &handle,
            "clock-reminder",
            WebviewUrl::App("index.html?view=clock-reminder".into()),
        )
        .data_directory(directory)
        .title("Vela 时间提醒")
        .inner_size(360.0, 270.0)
        .resizable(false)
        .always_on_top(true)
        .build();
    });
}
pub fn dismiss_all(app: &AppHandle) {
    let state = app.state::<AppState>();
    if update_settings(state.inner(), |s| {
        s.clock_tools.alerts.clear();
        Ok(())
    })
    .is_ok()
    {
        let _ = publish_snapshot(app, state.inner());
    }
}
pub fn start_worker(app: AppHandle) {
    std::thread::spawn(move || {
        let mut first = true;
        let mut last_beep = 0;
        loop {
            let now = now_ms();
            let state = app.state::<AppState>();
            let result = (|| -> Result<(bool, bool), String> {
                let db = lock_database(state.inner())?;
                // Inspect clock deadlines without silently cleaning other widgets' data.
                let document: String = db
                    .query_row("SELECT document FROM app_settings WHERE id=1", [], |row| {
                        row.get(0)
                    })
                    .map_err(|e| e.to_string())?;
                let mut settings: Settings =
                    serde_json::from_str(&document).map_err(|e| e.to_string())?;
                let changed = settings.clock_tools.tick(now);
                if changed {
                    write_settings(&db, &settings).map_err(|e| e.to_string())?;
                }
                let pending = !settings.clock_tools.alerts.is_empty();
                let audible = settings
                    .clock_tools
                    .alerts
                    .iter()
                    .any(|a| now - a.fired_at < 60000 && !a.missed);
                if changed {
                    let snapshot = read_snapshot(&db).map_err(|e| e.to_string())?;
                    let _ = app.emit(SNAPSHOT_UPDATED_EVENT, snapshot);
                }
                Ok((pending && (changed || first), audible))
            })();
            if let Ok((show, audible)) = result {
                if show {
                    show_reminder(&app);
                }
                if audible && now - last_beep >= 5000 {
                    #[cfg(target_os = "windows")]
                    unsafe {
                        let _ = windows::Win32::System::Diagnostics::Debug::MessageBeep(
                            windows::Win32::UI::WindowsAndMessaging::MB_ICONEXCLAMATION,
                        );
                    }
                    last_beep = now;
                }
            }
            first = false;
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tools_persist_separately_from_clock_display_settings() {
        let db = Connection::open_in_memory().unwrap();
        initialize_database(&db).unwrap();
        let mut settings = read_settings(&db).unwrap();
        settings
            .clock_tools
            .act("timer-set", None, Some(25), None, None, 1000)
            .unwrap();
        settings
            .clock_tools
            .act("timer-toggle", None, None, None, None, 1000)
            .unwrap();
        settings
            .clock_tools
            .act("stopwatch-toggle", None, None, None, None, 1000)
            .unwrap();
        settings.clock.show_seconds = true;
        write_settings(&db, &settings).unwrap();
        let restored = read_settings(&db).unwrap();
        assert_eq!(restored.clock_tools.timer.deadline, Some(26000));
        assert_eq!(restored.clock_tools.stopwatch.started_at, Some(1000));
        assert!(restored.clock.show_seconds);
        assert!(!restored.widgets["clock"].enabled);
    }
    #[test]
    fn timer_and_stopwatch_survive_switches_and_pause_without_drift() {
        let mut t = ClockTools::default();
        t.act("timer-set", None, Some(10), None, None, 1000)
            .unwrap();
        t.act("timer-toggle", None, None, None, None, 1000).unwrap();
        t.act("stopwatch-toggle", None, None, None, None, 1000)
            .unwrap();
        t.act("mode", None, None, Some("clock".into()), None, 2000)
            .unwrap();
        t.act("timer-toggle", None, None, None, None, 4000).unwrap();
        assert_eq!(t.timer.remaining_ms, 7000);
        t.act("stopwatch-lap", None, None, None, None, 4500)
            .unwrap();
        assert_eq!(t.stopwatch.laps, vec![3500]);
        t.act("stopwatch-toggle", None, None, None, None, 5000)
            .unwrap();
        assert_eq!(t.stopwatch.elapsed_ms, 4000);
        t.act("timer-toggle", None, None, None, None, 6000).unwrap();
        assert!(!t.tick(12999));
        assert!(t.tick(13000));
        assert!(!t.tick(14000));
        assert_eq!(t.alerts.len(), 1);
        let restored: ClockTools =
            serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        assert_eq!(restored.stopwatch.elapsed_ms, 4000);
    }
    #[test]
    fn recurring_alarms_skip_missed_occurrences_and_snooze_independently() {
        let now = Local
            .with_ymd_and_hms(2026, 10, 5, 7, 0, 0)
            .unwrap()
            .timestamp_millis();
        let mut t = ClockTools::default();
        t.act(
            "alarm-save",
            None,
            None,
            None,
            Some(AlarmInput {
                id: None,
                label: "起床".into(),
                time: "08:00".into(),
                date: None,
                weekdays: vec![0, 1, 2, 3, 4],
            }),
            now,
        )
        .unwrap();
        let due = t.alarms[0].next_at.unwrap();
        assert!(t.tick(due));
        assert!(t.alarms[0].next_at.unwrap() > due);
        let next = t.alarms[0].next_at;
        t.act("snooze", Some(t.alerts[0].id), None, None, None, due)
            .unwrap();
        assert_eq!(t.alarms[0].next_at, next);
        assert!(t.tick(due + 300000));
        assert!(!t.tick(due + 300001));
    }
    #[test]
    fn restart_catches_overdue_timer_once_and_rejects_invalid_input() {
        let mut t = ClockTools::default();
        t.timer.deadline = Some(1000);
        assert!(t.tick(100000));
        assert!(t.alerts[0].missed);
        assert!(!t.tick(100001));
        assert!(t.act("timer-set", None, Some(0), None, None, 1).is_err());
        assert!(next_alarm("25:00", None, &[], now_ms()).is_err());
        assert!(next_alarm("08:00", Some("2000-01-01"), &[], now_ms()).is_err());
    }
}
