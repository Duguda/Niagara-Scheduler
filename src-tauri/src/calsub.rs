//! Calendar subscriptions: a calendar's iCal address (for Google Calendar, "Secret address in
//! iCal format") is read every 30 minutes and on demand, and its events follow the feed
//! (added, changed, removed). One way only: nothing is sent back. The address is kept in this
//! computer's database only, since anyone with it can read the calendar.

use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::db::{self, Db, DbResult, IcsReport};

const KEY: &str = "cal.subscriptions";
const EVERY: Duration = Duration::from_secs(30 * 60);
const MAX_BYTES: u64 = 20 * 1024 * 1024;
/// Events that ended more than this many days ago are left alone (history is not rewritten).
const KEEP_DAYS: i64 = 30;
/// Colors handed to new subscriptions in turn (from the app's palette).
const COLORS: [u32; 6] = [0x22b5bf, 0x4aa8ff, 0x34c38f, 0xf5b83d, 0xb164e8, 0xff8a5c];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub url: String,
    pub color: u32,
    /// Unix seconds of the last successful read.
    #[serde(default)]
    pub last_sync: Option<i64>,
    #[serde(default)]
    pub last_error: Option<String>,
    /// Events in the feed at the last read.
    #[serde(default)]
    pub events: usize,
}

pub fn list(db: &Db) -> DbResult<Vec<Subscription>> {
    Ok(db.setting(KEY)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
}

fn save(db: &Db, subs: &[Subscription]) -> DbResult<()> {
    db.set_setting(KEY, &serde_json::to_string(subs).unwrap_or_else(|_| "[]".into()))
}

/// `webcal://` is the same address over https.
pub fn normalize(url: &str) -> Result<String, String> {
    let url = url.trim();
    let url = match url.strip_prefix("webcal://") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_string(),
    };
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("bad url".into());
    }
    Ok(url)
}

/// Downloads a feed and reads its events.
pub fn fetch(url: &str) -> Result<(Vec<crate::ics::IcsEvent>, Option<String>), String> {
    let resp = ureq::get(url)
        .timeout(Duration::from_secs(30))
        .set("User-Agent", "Niagara-Scheduler")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => format!("HTTP {code}"),
            other => other.to_string(),
        })?;
    let mut text = String::new();
    resp.into_reader().take(MAX_BYTES).read_to_string(&mut text).map_err(|e| e.to_string())?;
    if !text.contains("BEGIN:VCALENDAR") {
        return Err("not a calendar".into());
    }
    Ok((crate::ics::parse(&text), crate::ics::calendar_name(&text)))
}

fn cutoff() -> NaiveDate {
    Local::now().date_naive() - chrono::Duration::days(KEEP_DAYS)
}

/// Adds a subscription from a feed already read once (so a wrong address is caught right away).
pub fn add(db: &Db, name: &str, url: &str, events: &[crate::ics::IcsEvent], cal_name: Option<String>) -> Result<(Subscription, IcsReport), String> {
    let url = url.to_string();
    let mut subs = list(db).map_err(|e| e.to_string())?;
    if subs.iter().any(|s| s.url == url) {
        return Err("already subscribed".into());
    }
    let id = format!("{:x}", chrono::Utc::now().timestamp_millis());
    let name = Some(name.trim()).filter(|n| !n.is_empty()).map(str::to_string).or(cal_name).unwrap_or_else(|| "Calendar".into());
    let color = COLORS[subs.len() % COLORS.len()];
    let report = db.ics_sync(events, &id, color, cutoff()).map_err(|e| e.to_string())?;
    let sub = Subscription { id, name, url, color, last_sync: Some(Local::now().timestamp()), last_error: None, events: events.len() };
    subs.push(sub.clone());
    save(db, &subs).map_err(|e| e.to_string())?;
    Ok((sub, report))
}

/// Stops a subscription; its events are removed unless `keep_events`.
pub fn remove(db: &Db, id: &str, keep_events: bool) -> DbResult<()> {
    let subs: Vec<Subscription> = list(db)?.into_iter().filter(|s| s.id != id).collect();
    if !keep_events {
        db.ics_unsubscribe(id)?;
    } else {
        // The events stay as ordinary events; only the link to the feed goes.
        db.conn.execute("DELETE FROM ics_imports WHERE substr(uid, 1, length(?1)) = ?1", [format!("sub:{id}:")])?;
    }
    save(db, &subs)
}

/// Reads one subscription (or all) now. The network part runs without the database; `apply`
/// gets the database for the short write. Returns whether any event changed.
pub fn sync(read_subs: impl Fn() -> DbResult<Vec<Subscription>>, apply: impl Fn(&dyn Fn(&Db) -> DbResult<bool>) -> DbResult<bool>, only: Option<&str>) -> DbResult<bool> {
    let subs = read_subs()?;
    let mut changed = false;
    for sub in subs.iter().filter(|s| only.is_none_or(|id| s.id == id)) {
        let result = fetch(&sub.url);
        changed |= apply(&|db: &Db| {
            let mut all = list(db)?;
            let Some(stored) = all.iter_mut().find(|s| s.id == sub.id) else { return Ok(false) };
            let mut changed = false;
            match &result {
                Ok((events, _)) => {
                    let r = db.ics_sync(events, &stored.id, stored.color, cutoff())?;
                    changed = r.added + r.updated + r.removed > 0;
                    stored.last_sync = Some(Local::now().timestamp());
                    stored.last_error = None;
                    stored.events = events.len();
                }
                Err(e) => stored.last_error = Some(e.clone()),
            }
            save(db, &all)?;
            Ok(changed)
        })?;
    }
    Ok(changed)
}

/// Tells the windows to reload (same message the windows send each other, see sync.svelte.ts).
pub fn notify(app: &AppHandle) {
    let _ = app.emit("nora://data", serde_json::json!({ "from": "calsub" }));
}

/// Reads every subscription shortly after start and then every 30 minutes.
pub fn start(app: AppHandle, db_path: PathBuf) {
    let spawned = std::thread::Builder::new().name("calsub".into()).spawn(move || {
        let db = match db::open_conn(&db_path) {
            Ok(conn) => Db { conn },
            Err(e) => return eprintln!("calendar subscriptions could not open the database: {e}"),
        };
        std::thread::sleep(Duration::from_secs(20));
        loop {
            match sync(|| list(&db), |f| f(&db), None) {
                Ok(true) => notify(&app),
                Ok(false) => {}
                Err(e) => eprintln!("calendar subscriptions: {e}"),
            }
            std::thread::sleep(EVERY);
        }
    });
    if let Err(e) = spawned {
        eprintln!("could not start calendar subscriptions: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(normalize(" webcal://calendar.google.com/x/basic.ics ").unwrap(), "https://calendar.google.com/x/basic.ics");
        assert!(normalize("file:///etc/passwd").is_err());
        assert!(normalize("calendar.google.com").is_err());
    }
}
