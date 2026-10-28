//! Calendar import from iCalendar (`.ics`) files, such as the export of Google Calendar
//! (Settings → Import & export → Export gives a `.zip` of `.ics` files; a single calendar's
//! "Export calendar" gives one `.ics`).
//!
//! Supported: one-off and all-day events, DTEND or DURATION, LOCATION, DESCRIPTION, URL,
//! STATUS:CANCELLED, EXDATE, and RRULE with FREQ / INTERVAL / weekly BYDAY / UNTIL / COUNT.
//! Moved occurrences (RECURRENCE-ID) are skipped in their series and kept as their own events.
//! Rules Niagara cannot repeat exactly (e.g. "every 2nd Tuesday") become a single event and are
//! counted in the report.
//!
//! Times in UTC (`…Z`) are converted to this computer's time zone. Times with a TZID are taken
//! as wall-clock times, which is right when the calendar's zone is this computer's zone.

use std::io::Read;
use std::path::Path;

use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

use crate::recurrence::{Freq, Repeat};

#[derive(Clone, Debug, PartialEq)]
pub struct IcsEvent {
    pub uid: String,
    /// For a moved occurrence of a series: the date it originally fell on.
    pub recurrence_id: Option<NaiveDate>,
    pub title: String,
    pub start: NaiveDateTime,
    /// For all-day events, 00:00 of the last day (inclusive), as Niagara stores them.
    pub end: NaiveDateTime,
    pub all_day: bool,
    pub location: String,
    pub memo: String,
    pub url: Option<String>,
    pub repeat: Option<Repeat>,
    pub exdates: Vec<NaiveDate>,
    pub cancelled: bool,
    /// The file had a repeat rule Niagara cannot express; only the first occurrence is kept.
    pub rule_dropped: bool,
}

/// Joins folded lines (a line starting with a space or tab continues the previous one).
fn unfold(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(rest) = raw.strip_prefix(' ').or_else(|| raw.strip_prefix('\t')) {
            if let Some(last) = lines.last_mut() {
                last.push_str(rest);
                continue;
            }
        }
        lines.push(raw.to_string());
    }
    lines
}

/// `NAME;P1=a;P2="b:c":value` → (NAME, params, value).
fn split_prop(line: &str) -> Option<(String, Vec<(String, String)>, String)> {
    let mut quoted = false;
    let colon = line.char_indices().find(|&(_, c)| {
        if c == '"' {
            quoted = !quoted;
        }
        c == ':' && !quoted
    })?;
    let (head, value) = (&line[..colon.0], &line[colon.0 + 1..]);
    let mut parts = head.split(';');
    let name = parts.next()?.trim().to_ascii_uppercase();
    let params = parts
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.trim().to_ascii_uppercase(), v.trim_matches('"').to_string()))
        .collect();
    Some((name, params, value.to_string()))
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

enum When {
    Date(NaiveDate),
    Time(NaiveDateTime),
}

/// `20261028`, `20261028T090000`, `20261028T000000Z` (UTC → local).
fn parse_when(value: &str) -> Option<When> {
    let v = value.trim();
    if v.len() == 8 {
        return NaiveDate::parse_from_str(v, "%Y%m%d").ok().map(When::Date);
    }
    let (body, utc) = match v.strip_suffix('Z') {
        Some(b) => (b, true),
        None => (v, false),
    };
    let t = NaiveDateTime::parse_from_str(body, "%Y%m%dT%H%M%S").ok()?;
    Some(When::Time(if utc { Utc.from_utc_datetime(&t).with_timezone(&Local).naive_local() } else { t }))
}

fn when_date(w: &When) -> NaiveDate {
    match w {
        When::Date(d) => *d,
        When::Time(t) => t.date(),
    }
}

/// `P1D`, `PT1H30M`, `P2W`, `-PT15M` (sign ignored).
fn parse_duration(v: &str) -> Option<Duration> {
    let v = v.trim().trim_start_matches(['+', '-']).strip_prefix('P')?;
    let mut total = Duration::zero();
    let mut num = String::new();
    let mut in_time = false;
    for c in v.chars() {
        match c {
            'T' => in_time = true,
            '0'..='9' => num.push(c),
            _ => {
                let n: i64 = num.parse().ok()?;
                num.clear();
                total += match (c, in_time) {
                    ('W', _) => Duration::weeks(n),
                    ('D', _) => Duration::days(n),
                    ('H', true) => Duration::hours(n),
                    ('M', true) => Duration::minutes(n),
                    ('S', true) => Duration::seconds(n),
                    _ => return None,
                };
            }
        }
    }
    Some(total)
}

const WEEKDAYS: [&str; 7] = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"];

/// RRULE → Niagara's rule, or `None` when it cannot be repeated exactly.
fn parse_rrule(v: &str, start: NaiveDate) -> Option<Repeat> {
    let mut freq = None;
    let mut rule = Repeat { freq: Freq::Daily, interval: 1, weekdays: vec![], until: None, count: None };
    for part in v.split(';') {
        let Some((k, val)) = part.split_once('=') else { continue };
        match k.to_ascii_uppercase().as_str() {
            "FREQ" => {
                freq = Some(match val.to_ascii_uppercase().as_str() {
                    "DAILY" => Freq::Daily,
                    "WEEKLY" => Freq::Weekly,
                    "MONTHLY" => Freq::Monthly,
                    "YEARLY" => Freq::Yearly,
                    _ => return None,
                })
            }
            "INTERVAL" => rule.interval = val.parse().ok().filter(|n| *n > 0)?,
            "COUNT" => rule.count = Some(val.parse().ok()?),
            "UNTIL" => rule.until = parse_when(val).map(|w| when_date(&w)),
            "BYDAY" => {
                for d in val.split(',') {
                    // A position ("2TU", "-1FR") is a monthly/yearly pattern Niagara cannot repeat.
                    let i = WEEKDAYS.iter().position(|w| w.eq_ignore_ascii_case(d.trim()))?;
                    rule.weekdays.push(i as u8);
                }
            }
            "BYMONTHDAY" => {
                if val.split(',').count() != 1 || val.parse::<u32>().ok()? != chrono::Datelike::day(&start) {
                    return None;
                }
            }
            "BYMONTH" => {
                if val.split(',').count() != 1 || val.parse::<u32>().ok()? != chrono::Datelike::month(&start) {
                    return None;
                }
            }
            "WKST" => {}
            _ => return None,
        }
    }
    rule.freq = freq?;
    if !rule.weekdays.is_empty() {
        if rule.freq == Freq::Daily && rule.interval == 1 {
            // "Every weekday" style: daily limited to some days = weekly on those days.
            rule.freq = Freq::Weekly;
        } else if rule.freq != Freq::Weekly {
            return None;
        }
        rule.weekdays.sort_unstable();
        rule.weekdays.dedup();
    }
    Some(rule)
}

/// Every VEVENT in an iCalendar text. Events without a start are skipped.
pub fn parse(text: &str) -> Vec<IcsEvent> {
    let mut out = Vec::new();
    let mut cur: Option<Vec<(String, Vec<(String, String)>, String)>> = None;
    let mut depth = 0usize; // nested components (VALARM) inside a VEVENT
    for line in unfold(text) {
        let Some((name, params, value)) = split_prop(&line) else { continue };
        match (name.as_str(), value.trim().to_ascii_uppercase().as_str()) {
            ("BEGIN", "VEVENT") if cur.is_none() => cur = Some(Vec::new()),
            ("BEGIN", _) if cur.is_some() => depth += 1,
            ("END", "VEVENT") if depth == 0 => {
                if let Some(props) = cur.take()
                    && let Some(e) = build(&props)
                {
                    out.push(e);
                }
            }
            ("END", _) if cur.is_some() => depth = depth.saturating_sub(1),
            _ => {
                if depth == 0
                    && let Some(props) = cur.as_mut()
                {
                    props.push((name, params, value));
                }
            }
        }
    }
    out
}

fn build(props: &[(String, Vec<(String, String)>, String)]) -> Option<IcsEvent> {
    let get = |n: &str| props.iter().find(|(k, _, _)| k == n);
    let start = parse_when(&get("DTSTART")?.2)?;
    let all_day = matches!(start, When::Date(_));
    let start_dt = match &start {
        When::Date(d) => d.and_time(NaiveTime::MIN),
        When::Time(t) => *t,
    };
    let end_dt = if let Some(end) = get("DTEND").and_then(|p| parse_when(&p.2)) {
        match end {
            // DTEND of an all-day event is the day after the last day.
            When::Date(d) => (d - Duration::days(1)).max(start_dt.date()).and_time(NaiveTime::MIN),
            When::Time(t) => t.max(start_dt),
        }
    } else if let Some(dur) = get("DURATION").and_then(|p| parse_duration(&p.2)) {
        if all_day {
            (start_dt + dur - Duration::days(1)).max(start_dt)
        } else {
            start_dt + dur
        }
    } else if all_day {
        start_dt
    } else {
        start_dt + Duration::hours(1)
    };
    let mut rule_dropped = false;
    let repeat = get("RRULE").and_then(|p| {
        let r = parse_rrule(&p.2, start_dt.date());
        rule_dropped = r.is_none();
        r
    });
    let mut exdates: Vec<NaiveDate> = props
        .iter()
        .filter(|(k, _, _)| k == "EXDATE")
        .flat_map(|(_, _, v)| v.split(',').filter_map(|x| parse_when(x).map(|w| when_date(&w))).collect::<Vec<_>>())
        .collect();
    exdates.sort_unstable();
    exdates.dedup();
    let text = |n: &str| get(n).map(|p| unescape(&p.2).trim().to_string()).unwrap_or_default();
    let uid = text("UID");
    Some(IcsEvent {
        uid: if uid.is_empty() { format!("{}|{}", text("SUMMARY"), start_dt) } else { uid },
        recurrence_id: get("RECURRENCE-ID").and_then(|p| parse_when(&p.2)).map(|w| when_date(&w)),
        title: text("SUMMARY"),
        start: start_dt,
        end: end_dt,
        all_day,
        location: text("LOCATION"),
        memo: text("DESCRIPTION"),
        url: get("URL").map(|p| p.2.trim().to_string()).filter(|u| !u.is_empty()),
        repeat,
        exdates,
        cancelled: get("STATUS").is_some_and(|p| p.2.trim().eq_ignore_ascii_case("CANCELLED")),
        rule_dropped,
    })
}

/// Reads a `.ics` file, or every `.ics` inside a `.zip` (Google Calendar's full export).
/// Returns the events and the calendars' names.
pub fn read_file(path: &Path) -> Result<(Vec<IcsEvent>, Vec<String>), String> {
    const MAX: u64 = 50 * 1024 * 1024;
    let is_zip = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("zip"));
    let mut texts = Vec::new();
    if is_zip {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipArchive::new(file).map_err(|_| "not a zip".to_string())?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
            if !entry.name().to_ascii_lowercase().ends_with(".ics") || entry.size() > MAX {
                continue;
            }
            let mut text = String::new();
            entry.by_ref().take(MAX).read_to_string(&mut text).map_err(|e| e.to_string())?;
            texts.push(text);
        }
    } else {
        if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX {
            return Err("file too large".into());
        }
        texts.push(std::fs::read_to_string(path).map_err(|e| e.to_string())?);
    }
    if texts.iter().all(|t| !t.contains("BEGIN:VCALENDAR")) {
        return Err("not a calendar file".into());
    }
    let mut events = Vec::new();
    let mut names = Vec::new();
    for text in &texts {
        events.extend(parse(text));
        if let Some(name) = unfold(text).iter().filter_map(|l| split_prop(l)).find(|(n, _, _)| n == "X-WR-CALNAME") {
            names.push(unescape(&name.2));
        }
    }
    Ok((events, names))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "BEGIN:VCALENDAR\r\nX-WR-CALNAME:개인\r\nBEGIN:VEVENT\r\nUID:a1@google.com\r\nDTSTART;TZID=Asia/Seoul:20261030T090000\r\nDTEND;TZID=Asia/Seoul:20261030T103000\r\nSUMMARY:네트워크관리사 스터디\\, 2주차\r\nDESCRIPTION:첫 줄\\n둘째 줄 이어서 아주 긴 설명이 접\r\n 혀서 이어집니다\r\nLOCATION:도서관\r\nRRULE:FREQ=WEEKLY;BYDAY=FR,MO;UNTIL=20261231T145959Z\r\nEXDATE;TZID=Asia/Seoul:20261106T090000\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nDESCRIPTION:알림\r\nEND:VALARM\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b2\r\nDTSTART;VALUE=DATE:20261101\r\nDTEND;VALUE=DATE:20261103\r\nSUMMARY:휴가\r\nSTATUS:CANCELLED\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:c3\r\nDTSTART:20261105T120000\r\nDURATION:PT45M\r\nSUMMARY:점심\r\nRRULE:FREQ=MONTHLY;BYDAY=2TU\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    #[test]
    fn reads_events_rules_and_all_day_ends() {
        let ev = parse(SAMPLE);
        assert_eq!(ev.len(), 3);
        let a = &ev[0];
        assert_eq!(a.title, "네트워크관리사 스터디, 2주차");
        assert_eq!(a.memo, "첫 줄\n둘째 줄 이어서 아주 긴 설명이 접혀서 이어집니다");
        assert_eq!((a.start, a.end), (dt("2026-10-30 09:00"), dt("2026-10-30 10:30")));
        let r = a.repeat.as_ref().unwrap();
        assert_eq!((r.freq, r.weekdays.clone()), (Freq::Weekly, vec![1, 5]));
        assert!(r.until.is_some());
        assert_eq!(a.exdates, vec![NaiveDate::from_ymd_opt(2026, 11, 6).unwrap()]);

        let b = &ev[1];
        assert!(b.all_day && b.cancelled);
        assert_eq!((b.start, b.end), (dt("2026-11-01 00:00"), dt("2026-11-02 00:00")));

        let c = &ev[2];
        assert_eq!(c.end, dt("2026-11-05 12:45"));
        assert!(c.repeat.is_none() && c.rule_dropped);
    }

    #[test]
    fn durations_and_rules() {
        assert_eq!(parse_duration("P1DT2H"), Some(Duration::hours(26)));
        assert_eq!(parse_duration("P2W"), Some(Duration::weeks(2)));
        let d = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        assert_eq!(parse_rrule("FREQ=YEARLY;BYMONTH=10;BYMONTHDAY=30", d).map(|r| r.freq), Some(Freq::Yearly));
        assert!(parse_rrule("FREQ=MONTHLY;BYMONTHDAY=1,15", d).is_none());
        let weekdays = parse_rrule("FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR", d).unwrap();
        assert_eq!((weekdays.freq, weekdays.weekdays.len()), (Freq::Weekly, 5));
    }
}
