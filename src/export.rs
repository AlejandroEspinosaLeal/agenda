use chrono::{Duration, Utc};

use crate::calendar::{month_short_es, Date};
use crate::model::{AgendaItem, Recurrence, Schedule, TaskStore};

const WEEKDAY_SHORT: [&str; 7] = ["LUN", "MAR", "MIÉ", "JUE", "VIE", "SÁB", "DOM"];

/// Generates a GitHub-Flavored Markdown summary for the 7-day week starting at `week_start`.
pub fn export_week_markdown(store: &TaskStore, week_start: Date, week_num: u32) -> String {
    let mut md = format!("# Mi Agenda — Semana {week_num}\n\n");

    if let Some(intention) = store.weekly_intentions.get(&week_start.to_key()) {
        let trimmed = intention.trim();
        if !trimmed.is_empty() {
            md.push_str(&format!("> **Intención de la semana:** {trimmed}\n\n"));
        }
    }

    for offset in 0..7 {
        let day = week_start.add_days(offset);
        let occs = store.tasks_visible_on(day);
        md.push_str(&format!(
            "## {} {} {}\n\n",
            WEEKDAY_SHORT[offset as usize],
            day.day,
            month_short_es(day.month)
        ));
        if occs.is_empty() {
            md.push_str("_Sin bloques programados._\n\n");
        } else {
            for occ in occs {
                let mark = if occ.task.done { "x" } else { " " };
                let time_part = if occ.task.time.is_empty() {
                    String::new()
                } else if occ.task.end_time.is_empty() {
                    format!("`{}` ", occ.task.time)
                } else {
                    format!("`{}–{}` ", occ.task.time, occ.task.end_time)
                };
                let tag_part = if occ.task.tag.is_empty() {
                    String::new()
                } else {
                    format!(" **[{}]**", occ.task.tag)
                };
                md.push_str(&format!(
                    "- [{mark}] {time_part}{}{tag_part}\n",
                    occ.task.title
                ));
            }
            md.push('\n');
        }
    }

    md
}

/// Generates a standard RFC 5545 iCalendar (`.ics`) document for all items in `store`.
pub fn export_ics(store: &TaskStore) -> String {
    let mut ics = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Mi Agenda//Rust Desktop//ES\r\nCALSCALE:GREGORIAN\r\n",
    );
    let dtstamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    for item in &store.items {
        append_vevent(&mut ics, item, &dtstamp);
    }

    ics.push_str("END:VCALENDAR\r\n");
    ics
}

fn append_vevent(out: &mut String, item: &AgendaItem, dtstamp: &str) {
    out.push_str("BEGIN:VEVENT\r\n");
    out.push_str(&format!("UID:miagenda-{}@local\r\n", item.id));
    out.push_str(&format!("DTSTAMP:{dtstamp}\r\n"));

    match item.schedule {
        Schedule::AllDay => {
            let start = item.start_date.format("%Y%m%d");
            let end_date =
                item.start_date + chrono::Duration::days(i64::from(item.span_days.max(1)));
            out.push_str(&format!("DTSTART;VALUE=DATE:{start}\r\n"));
            out.push_str(&format!(
                "DTEND;VALUE=DATE:{}\r\n",
                end_date.format("%Y%m%d")
            ));
        }
        Schedule::TimeRange { start, end } => {
            let d_str = item.start_date.format("%Y%m%d");
            let s_str = start.format("%H%M%S");
            // DTEND must be strictly after DTSTART. Multi-day items end on their
            // last covered day; a night block (end <= start) crosses midnight.
            let span_offset = i64::from(item.span_days.max(1) - 1);
            let mut end_date = item.start_date + Duration::days(span_offset);
            if end <= start {
                end_date += Duration::days(1);
            }
            let e_str = end.format("%H%M%S");
            out.push_str(&format!("DTSTART:{d_str}T{s_str}\r\n"));
            out.push_str(&format!("DTEND:{}T{e_str}\r\n", end_date.format("%Y%m%d")));
        }
    }

    if let Some(rrule) = recurrence_to_rrule(item.recurrence) {
        out.push_str(&format!("RRULE:{rrule}\r\n"));
    }

    out.push_str(&format!("SUMMARY:{}\r\n", ics_escape(&item.title)));
    if !item.notes.is_empty() {
        out.push_str(&format!("DESCRIPTION:{}\r\n", ics_escape(&item.notes)));
    }
    if !item.tag.is_empty() {
        out.push_str(&format!("CATEGORIES:{}\r\n", ics_escape(&item.tag)));
    }
    if !item.location.is_empty() {
        out.push_str(&format!("LOCATION:{}\r\n", ics_escape(&item.location)));
    }
    out.push_str("END:VEVENT\r\n");
}

const fn recurrence_to_rrule(rec: Recurrence) -> Option<&'static str> {
    match rec {
        Recurrence::None => None,
        Recurrence::Daily => Some("FREQ=DAILY"),
        Recurrence::Weekdays => Some("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR"),
        Recurrence::Weekly => Some("FREQ=WEEKLY"),
        Recurrence::Monthly => Some("FREQ=MONTHLY"),
    }
}

/// Escapes special characters per RFC 5545 Section 3.3.11 (`\`, `;`, `,`, newlines).
pub fn ics_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Priority, TaskDraft};

    #[test]
    fn ics_escape_should_escape_rfc5545_special_chars() {
        assert_eq!(
            ics_escape("Reunión, notas; plan\\2026\nLínea 2"),
            "Reunión\\, notas\\; plan\\\\2026\\nLínea 2"
        );
    }

    #[test]
    fn export_ics_should_produce_valid_vcalendar_with_vevent_and_rrule() {
        let mut store = TaskStore::default();
        store.add_task(
            Date::new(2026, 11, 23),
            TaskDraft {
                title: "Deep Work, Rust",
                notes: "Repaso",
                time: "09:00",
                end_time: "10:30",
                location: "Estudio",
                tag: "Trabajo",
                span_days: 1,
                recurrence: Recurrence::Weekdays,
                color: None,
                priority: Priority::High,
            },
        );
        let ics = export_ics(&store);
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(ics.contains("BEGIN:VEVENT\r\n"));
        assert!(ics.contains("DTSTART:20261123T090000\r\n"));
        assert!(ics.contains("DTEND:20261123T103000\r\n"));
        assert!(ics.contains("RRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR\r\n"));
        assert!(ics.contains("SUMMARY:Deep Work\\, Rust\r\n"));
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
    }

    #[test]
    fn export_ics_should_roll_dtend_to_next_day_when_crossing_midnight() {
        let mut store = TaskStore::default();
        store.add_task(
            Date::new(2026, 11, 23),
            TaskDraft {
                title: "Guardia nocturna",
                notes: "",
                time: "23:30",
                end_time: "00:30",
                location: "",
                tag: "",
                span_days: 1,
                recurrence: Recurrence::None,
                color: None,
                priority: Priority::Normal,
            },
        );
        let ics = export_ics(&store);
        assert!(ics.contains("DTSTART:20261123T233000\r\n"));
        assert!(
            ics.contains("DTEND:20261124T003000\r\n"),
            "el fin nocturno debe caer al día siguiente: {ics}"
        );
    }

    #[test]
    fn export_ics_should_keep_dtend_after_dtstart_for_equal_and_multi_day_ranges() {
        let mut store = TaskStore::default();
        // Zero-length-looking range must still produce DTEND > DTSTART.
        store.add_task(
            Date::new(2026, 11, 23),
            TaskDraft {
                title: "Rango igual",
                notes: "",
                time: "08:00",
                end_time: "08:00",
                location: "",
                tag: "",
                span_days: 1,
                recurrence: Recurrence::None,
                color: None,
                priority: Priority::Normal,
            },
        );
        // Multi-day timed item ends on its last covered day.
        store.add_task(
            Date::new(2026, 11, 23),
            TaskDraft {
                title: "Congreso",
                notes: "",
                time: "09:00",
                end_time: "18:00",
                location: "",
                tag: "",
                span_days: 3,
                recurrence: Recurrence::None,
                color: None,
                priority: Priority::Normal,
            },
        );
        let ics = export_ics(&store);
        assert!(ics.contains("DTSTART:20261123T080000\r\n"));
        assert!(ics.contains("DTEND:20261124T080000\r\n"));
        assert!(ics.contains("DTSTART:20261123T090000\r\n"));
        assert!(ics.contains("DTEND:20261125T180000\r\n"));

        // Global invariant: every DTEND timestamp is strictly after its DTSTART.
        let mut start: Option<&str> = None;
        for line in ics.split("\r\n") {
            if let Some(value) = line.strip_prefix("DTSTART:") {
                start = Some(value);
            } else if let Some(value) = line.strip_prefix("DTEND:") {
                let s = start.expect("DTSTART before DTEND");
                assert!(value > s, "DTEND {value} debe ser posterior a DTSTART {s}");
            }
        }
    }

    #[test]
    fn export_week_markdown_should_include_tasks_and_checkboxes() {
        let mut store = TaskStore::default();
        let mon = Date::new(2026, 11, 23);
        store.add_task(
            mon,
            TaskDraft {
                title: "Sprint",
                notes: "",
                time: "10:00",
                end_time: "11:00",
                location: "",
                tag: "Trabajo",
                span_days: 1,
                recurrence: Recurrence::None,
                color: None,
                priority: Priority::Normal,
            },
        );
        let md = export_week_markdown(&store, mon, 48);
        assert!(md.contains("# Mi Agenda — Semana 48"));
        assert!(md.contains("- [ ] `10:00–11:00` Sprint **[Trabajo]**"));
    }
}
