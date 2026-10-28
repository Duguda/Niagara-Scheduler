//! Morning brief feed.
//!
//! A small JSON summary of today, written to `brief.json` in the data folder at start-up and
//! every few minutes. Other tools (a morning brief, scripts) read it instead of the database,
//! so they never have to understand the schema or expand repeating events themselves.
//!
//! Only planning data goes in: events, due todos, D-Days and focus time. The diary, expenses,
//! commissions and activity tracking never do.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration as StdDuration;

use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::Serialize;

use crate::db::{self, Db, DbResult, Event};

pub const FILE_NAME: &str = "brief.json";
/// D-Days further away than this are left out.
const DDAY_HORIZON_DAYS: i64 = 60;
const REFRESH: StdDuration = StdDuration::from_secs(5 * 60);

#[derive(Debug, Serialize, PartialEq)]
pub struct BriefEvent {
    pub title: String,
    /// `HH:MM`, or `None` for all-day events and ones that started on an earlier day.
    pub start: Option<String>,
    /// `HH:MM`, or `None` for all-day events and ones that end on a later day.
    pub end: Option<String>,
    pub all_day: bool,
    pub location: String,
    pub cancelled: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct BriefTodo {
    pub title: String,
    pub due: NaiveDate,
    pub due_time: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct BriefDDay {
    pub title: String,
    /// The date being counted down to (next anniversary for yearly D-Days).
    pub date: NaiveDate,
    pub days_left: i64,
}

#[derive(Debug, Serialize)]
pub struct Brief {
    pub version: u32,
    /// Local time this file was written, `YYYY-MM-DDTHH:MM`.
    pub generated_at: String,
    pub date: NaiveDate,
    pub today: Vec<BriefEvent>,
    pub tomorrow: Vec<BriefEvent>,
    pub todos_overdue: Vec<BriefTodo>,
    pub todos_today: Vec<BriefTodo>,
    pub ddays: Vec<BriefDDay>,
    pub focus_minutes_yesterday: i64,
    pub focus_sessions_yesterday: usize,
}

fn day_events(db: &Db, day: NaiveDate) -> DbResult<Vec<BriefEvent>> {
    let mut events: Vec<Event> = db.events_between(day, day)?;
    events.sort_by_key(|e| (!e.all_day, e.start));
    Ok(events
        .into_iter()
        .map(|e| {
            let start = (!e.all_day && e.start.date() == day).then(|| e.start.format("%H:%M").to_string());
            let end = (!e.all_day && e.end.date() == day).then(|| e.end.format("%H:%M").to_string());
            BriefEvent { title: e.title, start, end, all_day: e.all_day, location: e.location, cancelled: e.cancelled }
        })
        .collect())
}

/// The day a D-Day counts toward from `today`: its date, or for yearly ones the next anniversary.
fn target_date(date: NaiveDate, yearly: bool, today: NaiveDate) -> NaiveDate {
    if !yearly {
        return date;
    }
    // 29 February falls on the 28th in other years.
    let in_year = |y: i32| NaiveDate::from_ymd_opt(y, date.month(), date.day()).or_else(|| NaiveDate::from_ymd_opt(y, date.month(), 28));
    match in_year(today.year()) {
        Some(d) if d >= today => d,
        _ => in_year(today.year() + 1).unwrap_or(date),
    }
}

pub fn build(db: &Db, today: NaiveDate) -> DbResult<Brief> {
    let groups: std::collections::HashMap<i64, String> = db.todo_groups()?.into_iter().map(|g| (g.id, g.name)).collect();
    let (mut todos_overdue, mut todos_today) = (Vec::new(), Vec::new());
    for t in db.todos()? {
        let Some(due) = t.due.filter(|_| !t.done) else { continue };
        let item = BriefTodo { title: t.title, due, due_time: t.due_time, group: t.group_id.and_then(|id| groups.get(&id).cloned()) };
        if due < today {
            todos_overdue.push(item);
        } else if due == today {
            todos_today.push(item);
        }
    }
    todos_overdue.sort_by(|a, b| a.due.cmp(&b.due));
    todos_today.sort_by(|a, b| a.due_time.is_none().cmp(&b.due_time.is_none()).then_with(|| a.due_time.cmp(&b.due_time)));

    let mut ddays: Vec<BriefDDay> = db
        .ddays()?
        .into_iter()
        .map(|d| {
            let date = target_date(d.date, d.yearly, today);
            BriefDDay { title: d.title, date, days_left: (date - today).num_days() }
        })
        .filter(|d| (0..=DDAY_HORIZON_DAYS).contains(&d.days_left))
        .collect();
    ddays.sort_by_key(|d| d.days_left);

    let yesterday = today - Duration::days(1);
    let sessions = db.pomodoro_sessions(db::day_start_ts(yesterday), db::day_start_ts(today))?;
    let focus_minutes_yesterday = sessions.iter().map(|s| s.focused_secs).sum::<i64>() / 60;

    Ok(Brief {
        version: 1,
        generated_at: Local::now().format("%Y-%m-%dT%H:%M").to_string(),
        date: today,
        today: day_events(db, today)?,
        tomorrow: day_events(db, today + Duration::days(1))?,
        todos_overdue,
        todos_today,
        ddays,
        focus_minutes_yesterday,
        focus_sessions_yesterday: sessions.len(),
    })
}

/// Writes the brief next to the database. A temporary file plus rename means a reader never
/// sees a half-written file.
pub fn write(dir: &Path, brief: &Brief) -> io::Result<PathBuf> {
    let json = serde_json::to_string_pretty(brief).map_err(io::Error::other)?;
    let path = dir.join(FILE_NAME);
    let tmp = dir.join(format!("{FILE_NAME}.tmp"));
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &path)?;
    Ok(path)
}

pub fn refresh(db: &Db, dir: &Path) -> Result<PathBuf, String> {
    let brief = build(db, Local::now().date_naive()).map_err(|e| e.to_string())?;
    write(dir, &brief).map_err(|e| e.to_string())
}

/// Keeps `brief.json` current while the app runs.
pub fn start(db_path: PathBuf, dir: PathBuf) {
    let spawned = std::thread::Builder::new().name("brief".into()).spawn(move || {
        let db = match db::open_conn(&db_path) {
            Ok(conn) => Db { conn },
            Err(e) => return eprintln!("brief could not open database: {e}"),
        };
        loop {
            if let Err(e) = refresh(&db, &dir) {
                eprintln!("could not write the brief: {e}");
            }
            std::thread::sleep(REFRESH);
        }
    });
    if let Err(e) = spawned {
        eprintln!("could not start the brief thread: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{d, temp_db};

    #[test]
    fn yearly_ddays_count_to_the_next_anniversary() {
        let today = d("2026-10-28");
        assert_eq!(target_date(d("2000-11-03"), true, today), d("2026-11-03"));
        assert_eq!(target_date(d("2000-10-28"), true, today), d("2026-10-28"));
        assert_eq!(target_date(d("2000-01-05"), true, today), d("2027-01-05"));
        assert_eq!(target_date(d("2024-02-29"), true, d("2026-03-01")), d("2027-02-28"));
        assert_eq!(target_date(d("2026-12-01"), false, today), d("2026-12-01"));
    }

    #[test]
    fn todos_are_split_into_overdue_and_today_and_done_ones_are_left_out() {
        let (db, _) = temp_db();
        let today = d("2026-10-28");
        let g = db.add_todo_group("자격증", 0).unwrap();
        db.add_todo(Some(g), None, "SQL 복습", Some(d("2026-10-27")), None).unwrap();
        db.add_todo(None, None, "네트워크 필기", Some(today), Some("09:00")).unwrap();
        let done = db.add_todo(None, None, "끝난 일", Some(today), None).unwrap();
        db.set_todo_done(done, true).unwrap();
        db.add_todo(None, None, "나중 일", Some(d("2026-11-10")), None).unwrap();
        db.add_todo(None, None, "기한 없음", None, None).unwrap();

        let brief = build(&db, today).unwrap();
        assert_eq!(brief.todos_overdue.len(), 1);
        assert_eq!(brief.todos_overdue[0].title, "SQL 복습");
        assert_eq!(brief.todos_overdue[0].group.as_deref(), Some("자격증"));
        assert_eq!(brief.todos_today.len(), 1);
        assert_eq!(brief.todos_today[0].due_time.as_deref(), Some("09:00"));
    }

    #[test]
    fn writes_valid_json_next_to_the_database() {
        let (db, path) = temp_db();
        let dir = path.with_extension("briefdir");
        fs::create_dir_all(&dir).unwrap();
        let written = refresh(&db, &dir).unwrap();
        let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&written).unwrap()).unwrap();
        assert_eq!(value["version"], 1);
        assert!(value["today"].is_array());
        assert!(!dir.join("brief.json.tmp").exists());
    }
}
