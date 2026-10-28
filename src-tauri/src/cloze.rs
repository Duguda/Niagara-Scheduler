//! Cloze study: **bold** text in a note or memo becomes a blank. The user recalls it, reveals it
//! and marks themselves ○ (recalled) or ✕ (missed). Each blank keeps its own record and a review
//! date: a ○ moves it up one box (longer gap), a ✕ sends it back to box 1 (tomorrow).
//!
//! A blank is identified by its source (`ob:<vault path>` or `memo:<id>`), its text, and which
//! occurrence of that text in the source it is — so editing other parts of a note keeps records.

use std::collections::HashMap;

use chrono::{Duration, Local, NaiveDate};
use rusqlite::{OptionalExtension, Row, params};
use serde::Serialize;

use crate::db::{D_FMT, Db, DbResult, now_ts, parse_d};

/// Days until the next review for each box (box 1 = first gap). One place to tune the schedule.
pub const BOX_INTERVALS: [i64; 5] = [1, 2, 4, 8, 16];
const TOP_BOX: i64 = BOX_INTERVALS.len() as i64;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ClozeStat {
    /// `<source>|<answer>|<occurrence>`.
    pub key: String,
    pub right: i64,
    pub wrong: i64,
    /// Whether the latest mark was ○.
    pub last_right: bool,
    /// 1..=5, see `BOX_INTERVALS`.
    pub box_no: i64,
    /// Next review day.
    pub due: NaiveDate,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SourceDue {
    pub source: String,
    /// Blanks whose review day has come.
    pub due: i64,
    /// Blanks whose latest mark was ✕.
    pub weak: i64,
    /// Blanks with any record.
    pub seen: i64,
}

fn stat_from_row(r: &Row) -> rusqlite::Result<ClozeStat> {
    Ok(ClozeStat {
        key: r.get(0)?,
        right: r.get(1)?,
        wrong: r.get(2)?,
        last_right: r.get(3)?,
        box_no: r.get(4)?,
        due: parse_d(r.get::<_, String>(5)?),
    })
}

/// Box and review day after a mark.
pub fn next_box(current: Option<i64>, right: bool, today: NaiveDate) -> (i64, NaiveDate) {
    let box_no = match (current, right) {
        (_, false) => 1,
        (None, true) => 2,
        (Some(b), true) => (b + 1).min(TOP_BOX),
    };
    // A ✕ is reviewed again tomorrow; a ○ waits for its box's gap.
    let days = if right { BOX_INTERVALS[(box_no - 1) as usize] } else { BOX_INTERVALS[0] };
    (box_no, today + Duration::days(days))
}

impl Db {
    /// Every record for one source.
    pub fn cloze_stats(&self, source: &str) -> DbResult<Vec<ClozeStat>> {
        let mut st = self.conn.prepare(
            "SELECT key, right, wrong, last_right, box, due FROM cloze_stats WHERE source = ?1",
        )?;
        st.query_map([source], stat_from_row)?.collect()
    }

    /// Saves a ○ (`right`) or ✕ for one blank and returns its new record.
    pub fn cloze_record(&self, source: &str, key: &str, right: bool, today: NaiveDate) -> DbResult<ClozeStat> {
        let current: Option<i64> =
            self.conn.query_row("SELECT box FROM cloze_stats WHERE key = ?1", [key], |r| r.get(0)).optional()?;
        let (box_no, due) = next_box(current, right, today);
        self.conn.execute(
            "INSERT INTO cloze_stats(key, source, right, wrong, last_right, box, due, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(key) DO UPDATE SET
                 right = right + ?3, wrong = wrong + ?4, last_right = ?5, box = ?6, due = ?7, updated_at = ?8",
            params![key, source, right as i64, (!right) as i64, right, box_no, due.format(D_FMT).to_string(), now_ts()],
        )?;
        self.conn.query_row(
            "SELECT key, right, wrong, last_right, box, due FROM cloze_stats WHERE key = ?1",
            [key],
            stat_from_row,
        )
    }

    /// Forgets the records of one source (start over).
    pub fn cloze_reset(&self, source: &str) -> DbResult<()> {
        self.conn.execute("DELETE FROM cloze_stats WHERE source = ?1", [source])?;
        Ok(())
    }

    /// Per source: how many blanks are due today, weak, and seen. Sources with no record are left out.
    pub fn cloze_overview(&self, today: NaiveDate) -> DbResult<Vec<SourceDue>> {
        let mut st = self.conn.prepare(
            "SELECT source,
                    SUM(due <= ?1),
                    SUM(last_right = 0),
                    COUNT(*)
             FROM cloze_stats GROUP BY source",
        )?;
        st.query_map([today.format(D_FMT).to_string()], |r| {
            Ok(SourceDue { source: r.get(0)?, due: r.get(1)?, weak: r.get(2)?, seen: r.get(3)? })
        })?
        .collect()
    }

    /// Blanks due today across every source (for the brief and the sidebar count).
    pub fn cloze_due_total(&self, today: NaiveDate) -> DbResult<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM cloze_stats WHERE due <= ?1",
            [today.format(D_FMT).to_string()],
            |r| r.get(0),
        )
    }
}

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// How many `**bold**` spans a Markdown text has (code is ignored). Used to list study sources
/// without rendering them.
pub fn count_bold(text: &str) -> usize {
    let mut n = 0;
    let mut in_fence = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        // Drop inline code, then count `**…**` pairs with something inside.
        let mut plain = String::new();
        let mut in_code = false;
        for c in line.chars() {
            if c == '`' {
                in_code = !in_code;
            } else if !in_code {
                plain.push(c);
            }
        }
        let parts: Vec<&str> = plain.split("**").collect();
        n += parts.iter().skip(1).step_by(2).zip(parts.iter().skip(2).step_by(2)).filter(|(inner, _)| !inner.trim().is_empty()).count();
    }
    n
}

/// The tag that opts a note or memo into recall practice (`#암기`, or `암기` in front-matter tags).
pub const STUDY_TAG: &str = "암기";

/// True when the text carries the study tag: `#암기` (or a nested `#암기/과목`) in the body outside
/// code, or `암기` in the front matter's `tags`. Bold text alone is often just emphasis.
pub fn has_study_tag(text: &str) -> bool {
    let mut lines = text.lines().peekable();
    // Front matter: `tags: [암기, …]`, `tags: 암기`, or a `tags:` block list.
    if lines.peek().is_some_and(|l| l.trim() == "---") {
        lines.next();
        let mut in_tags = false;
        for line in lines.by_ref() {
            let t = line.trim();
            if t == "---" {
                break;
            }
            let tag_ok = |v: &str| {
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'').trim_start_matches('#');
                v == STUDY_TAG || v.starts_with(&format!("{STUDY_TAG}/"))
            };
            if let Some(rest) = t.strip_prefix("tags:").or_else(|| t.strip_prefix("tag:")) {
                let rest = rest.trim().trim_start_matches('[').trim_end_matches(']');
                if rest.split(',').any(tag_ok) {
                    return true;
                }
                in_tags = rest.is_empty();
            } else if in_tags && t.starts_with('-') {
                if tag_ok(&t[1..]) {
                    return true;
                }
            } else {
                in_tags = false;
            }
        }
    }
    let mut in_fence = false;
    for line in lines {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut in_code = false;
        let chars: Vec<char> = line.chars().collect();
        let tag: Vec<char> = format!("#{STUDY_TAG}").chars().collect();
        for i in 0..chars.len() {
            if chars[i] == '`' {
                in_code = !in_code;
            }
            if in_code || chars[i..].len() < tag.len() || chars[i..i + tag.len()] != tag[..] {
                continue;
            }
            let before_ok = i == 0 || chars[i - 1].is_whitespace() || "([".contains(chars[i - 1]);
            let after = chars.get(i + tag.len());
            let after_ok = after.is_none_or(|c| c.is_whitespace() || *c == '/' || ",.;:!?)]".contains(*c));
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

/// Records grouped by source, for callers that need a lookup.
pub fn by_source(rows: Vec<SourceDue>) -> HashMap<String, SourceDue> {
    rows.into_iter().map(|r| (r.source.clone(), r)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{d, temp_db};

    #[test]
    fn boxes_climb_on_right_and_reset_on_wrong() {
        let t = d("2026-10-28");
        assert_eq!(next_box(None, true, t), (2, d("2026-10-30")));
        assert_eq!(next_box(None, false, t), (1, d("2026-10-29")));
        assert_eq!(next_box(Some(2), true, t), (3, d("2026-11-01")));
        assert_eq!(next_box(Some(5), true, t), (5, d("2026-11-13")));
        assert_eq!(next_box(Some(4), false, t), (1, d("2026-10-29")));
    }

    #[test]
    fn records_accumulate_per_blank_and_overview_counts_due_and_weak() {
        let (db, _) = temp_db();
        let t = d("2026-10-28");
        let s = db.cloze_record("ob:SQL.md", "ob:SQL.md|JOIN|0", false, t).unwrap();
        assert_eq!((s.right, s.wrong, s.last_right, s.box_no), (0, 1, false, 1));
        let s = db.cloze_record("ob:SQL.md", "ob:SQL.md|JOIN|0", true, t).unwrap();
        assert_eq!((s.right, s.wrong, s.last_right, s.box_no), (1, 1, true, 2));
        db.cloze_record("ob:SQL.md", "ob:SQL.md|HAVING|0", false, t).unwrap();
        db.cloze_record("memo:3", "memo:3|OSI|0", true, t).unwrap();

        assert_eq!(db.cloze_stats("ob:SQL.md").unwrap().len(), 2);
        let tomorrow = d("2026-10-29");
        let by = by_source(db.cloze_overview(tomorrow).unwrap());
        assert_eq!(by["ob:SQL.md"], SourceDue { source: "ob:SQL.md".into(), due: 1, weak: 1, seen: 2 });
        assert_eq!(db.cloze_due_total(d("2026-10-30")).unwrap(), 3);

        db.cloze_reset("ob:SQL.md").unwrap();
        assert!(db.cloze_stats("ob:SQL.md").unwrap().is_empty());
        assert_eq!(db.cloze_stats("memo:3").unwrap().len(), 1);
    }

    #[test]
    fn the_study_tag_opts_a_note_in() {
        assert!(has_study_tag("# SQL #암기\n**JOIN**"));
        assert!(has_study_tag("#암기/네트워크관리사 **53**"));
        assert!(has_study_tag("---\ntags: [sql, 암기]\n---\n**JOIN**"));
        assert!(has_study_tag("---\ntags:\n  - 공부\n  - \"#암기\"\n---\nbody"));
        assert!(!has_study_tag("**JOIN** just emphasis"));
        assert!(!has_study_tag("#암기장 is another tag"));
        assert!(!has_study_tag("`#암기` in code"));
        assert!(!has_study_tag("```\n#암기\n```"));
        assert!(!has_study_tag("url#암기"));
    }

    #[test]
    fn bold_counting_ignores_code_and_empty_pairs() {
        assert_eq!(count_bold("**JOIN** and **HAVING**"), 2);
        assert_eq!(count_bold("no bold here"), 0);
        assert_eq!(count_bold("`**not**` but **yes**"), 1);
        assert_eq!(count_bold("```\n**code**\n```\n**text**"), 1);
        assert_eq!(count_bold("**** empty"), 0);
        assert_eq!(count_bold("**open only"), 0);
    }
}
