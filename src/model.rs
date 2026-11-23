use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Timelike, Weekday};
use serde::{Deserialize, Serialize};

use crate::calendar::Date;

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

const fn default_span_days() -> u8 {
    1
}

/// Task urgency/priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
}

impl Priority {
    pub const fn label_es(self) -> &'static str {
        match self {
            Self::Low => "Baja",
            Self::Normal => "Media",
            Self::High => "Alta",
        }
    }
}

/// Time-of-day scheduling for an item, separated from recurrence rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Schedule {
    #[default]
    AllDay,
    TimeRange {
        start: NaiveTime,
        end: NaiveTime,
    },
}

impl Schedule {
    /// Returns the duration of a `TimeRange`, or `None` for `AllDay`.
    #[allow(dead_code)]
    pub fn duration(&self) -> Option<Duration> {
        match self {
            Self::AllDay => None,
            Self::TimeRange { start, end } => {
                if *end >= *start {
                    Some(*end - *start)
                } else {
                    Some(*end - *start + Duration::hours(24))
                }
            }
        }
    }

    pub fn start_str(&self) -> String {
        match self {
            Self::AllDay => String::new(),
            Self::TimeRange { start, .. } => start.format("%H:%M").to_string(),
        }
    }

    pub fn end_str(&self) -> String {
        match self {
            Self::AllDay => String::new(),
            Self::TimeRange { start, end } if end == start => String::new(),
            Self::TimeRange { end, .. } => end.format("%H:%M").to_string(),
        }
    }
}

/// Repetition rule for an item, independent of its time-of-day schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Recurrence {
    #[default]
    None,
    Daily,
    Weekdays,
    Weekly,
    Monthly,
}

impl Recurrence {
    /// Returns `true` if an item starting on `start` occurs on `target` via this recurrence.
    pub fn matches(&self, start: NaiveDate, target: NaiveDate) -> bool {
        if target < start {
            return false;
        }
        match self {
            Self::None => target == start,
            Self::Daily => true,
            Self::Weekdays => !matches!(target.weekday(), Weekday::Sat | Weekday::Sun),
            Self::Weekly => (target - start).num_days() % 7 == 0,
            Self::Monthly => {
                let last_day = last_day_of_month(target.year(), target.month());
                target.day() == start.day().min(last_day)
            }
        }
    }
}

/// Explicit domain distinction between actionable Tasks and scheduled Events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Task {
        #[serde(default)]
        completed: bool,
        #[serde(default)]
        priority: Priority,
        #[serde(default)]
        auto_rollover: bool,
    },
    Event {
        #[serde(default)]
        location: String,
    },
}

impl Default for ItemKind {
    fn default() -> Self {
        Self::Task {
            completed: false,
            priority: Priority::Normal,
            auto_rollover: false,
        }
    }
}

/// Unified persisted agenda item (Task or Event) using real `chrono` types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgendaItem {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub notes: String,
    pub start_date: NaiveDate,
    #[serde(default = "default_span_days")]
    pub span_days: u8,
    #[serde(default)]
    pub schedule: Schedule,
    #[serde(default)]
    pub recurrence: Recurrence,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub color: Option<[u8; 3]>,
    #[serde(default)]
    pub kind: ItemKind,
}

impl AgendaItem {
    pub fn covers_date(&self, target: NaiveDate) -> bool {
        if self.recurrence != Recurrence::None {
            return self.recurrence.matches(self.start_date, target);
        }
        let span = i64::from(self.span_days.max(1));
        let delta = (target - self.start_date).num_days();
        delta >= 0 && delta < span
    }

    pub fn is_completed(&self) -> bool {
        match self.kind {
            ItemKind::Task { completed, .. } => completed,
            ItemKind::Event { .. } => false,
        }
    }

    pub fn priority(&self) -> Priority {
        match self.kind {
            ItemKind::Task { priority, .. } => priority,
            ItemKind::Event { .. } => Priority::Normal,
        }
    }

    /// Converts to the UI view projection [`Task`] used by Phase 1 views.
    pub fn to_view_task(&self) -> Task {
        let loc = match &self.kind {
            ItemKind::Event { location } if !location.is_empty() => location.clone(),
            _ => self.location.clone(),
        };
        Task {
            id: self.id,
            title: self.title.clone(),
            notes: self.notes.clone(),
            time: self.schedule.start_str(),
            end_time: self.schedule.end_str(),
            location: loc,
            tag: self.tag.clone(),
            span_days: self.span_days,
            color: self.color,
            done: self.is_completed(),
            priority: self.priority(),
        }
    }
}

impl Task {
    /// Returns `(start_min, end_min)` in minutes since midnight if timed, or `None` if AllDay.
    pub fn time_bounds_minutes(&self) -> Option<(u32, u32)> {
        let start = NaiveTime::parse_from_str(self.time.trim(), "%H:%M").ok()?;
        let s_min = start.hour() * 60 + start.minute();
        let e_min = NaiveTime::parse_from_str(self.end_time.trim(), "%H:%M")
            .map(|e| e.hour() * 60 + e.minute())
            .unwrap_or_else(|_| (s_min + 60).min(1440));
        let end = if e_min > s_min {
            e_min
        } else {
            (s_min + 30).min(1440)
        };
        Some((s_min, end))
    }
}

/// View-layer projection preserved for existing Phase 1 views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub notes: String,
    pub time: String,
    pub end_time: String,
    pub location: String,
    pub tag: String,
    pub span_days: u8,
    pub color: Option<[u8; 3]>,
    pub done: bool,
    pub priority: Priority,
}

/// A task as seen on a particular calendar day, preserving its `start_date`.
#[derive(Debug, Clone)]
pub struct TaskOccurrence {
    pub start_date: Date,
    pub task: Task,
}

/// Borrowed input fields for creating a new task or event.
pub struct TaskDraft<'a> {
    pub title: &'a str,
    pub notes: &'a str,
    pub time: &'a str,
    pub end_time: &'a str,
    pub location: &'a str,
    pub tag: &'a str,
    pub span_days: u8,
    pub recurrence: Recurrence,
    pub color: Option<[u8; 3]>,
    pub priority: Priority,
}

/// Persistent v2 store with schema versioning, items, daily notes, and weekly intentions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStore {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default = "default_next_id")]
    pub next_id: u64,
    #[serde(default)]
    pub items: Vec<AgendaItem>,
    #[serde(default)]
    pub daily_notes: BTreeMap<String, String>,
    #[serde(default)]
    pub weekly_intentions: BTreeMap<String, String>,
    #[serde(default)]
    pub last_notified_date: Option<String>,
}

const fn default_next_id() -> u64 {
    1
}

impl Default for TaskStore {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            next_id: 1,
            items: Vec::new(),
            daily_notes: BTreeMap::new(),
            weekly_intentions: BTreeMap::new(),
            last_notified_date: None,
        }
    }
}

const DATA_MAGIC: &[u8; 4] = b"OTSK";
const DATA_KEY: &[u8; 8] = b"rU5t_T4s";

impl TaskStore {
    /// Returns the app-data storage path (`%LOCALAPPDATA%\organizador_tareas\tareas.dat`).
    pub fn default_storage_path() -> PathBuf {
        if let Some(base) = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .or_else(|| std::env::var_os("HOME"))
        {
            return PathBuf::from(base)
                .join("organizador_tareas")
                .join("tareas.dat");
        }
        PathBuf::from("tareas.dat")
    }

    /// Loads tasks from `path`, migrating legacy v1 stores or `tareas.json` without panicking.
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read(path) {
            Ok(bytes) => decode_store(&bytes),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if let Some(migrated) = Self::try_migrate_legacy_json(path) {
                    return Ok(migrated);
                }
                Ok(Self::default())
            }
            Err(err) => Err(err),
        }
    }

    fn try_migrate_legacy_json(target_path: &Path) -> Option<Self> {
        let exe = std::env::current_exe().ok()?;
        let legacy = exe.parent()?.join("tareas.json");
        let raw = fs::read_to_string(&legacy).ok()?;
        let store = parse_any_store_json(raw.as_bytes()).ok()?;
        if store.save(target_path).is_ok() {
            let _ = fs::remove_file(&legacy);
        }
        Some(store)
    }

    /// Saves tasks to `path` in a compact non-plaintext binary format.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let encoded = encode_store(self)?;
        fs::write(path, encoded)
    }

    /// Moves incomplete tasks with `auto_rollover = true` from past dates up to `today`.
    #[allow(dead_code)]
    pub fn rollover_overdue(&mut self, today: NaiveDate) -> bool {
        let mut changed = false;
        for item in &mut self.items {
            if let ItemKind::Task {
                completed: false,
                auto_rollover: true,
                ..
            } = item.kind
            {
                if item.start_date < today && item.recurrence == Recurrence::None {
                    item.start_date = today;
                    changed = true;
                }
            }
        }
        changed
    }

    /// Returns pending `AllDay` tasks (backlog without assigned time).
    pub fn unscheduled_backlog(&self) -> Vec<TaskOccurrence> {
        self.items
            .iter()
            .filter(|item| {
                !item.is_completed() && matches!(item.schedule, Schedule::AllDay)
            })
            .map(|item| TaskOccurrence {
                start_date: from_naive_date(item.start_date),
                task: item.to_view_task(),
            })
            .collect()
    }

    /// Returns all items visible on `date`.
    pub fn tasks_visible_on(&self, date: Date) -> Vec<TaskOccurrence> {
        let Some(target) = to_naive_date(date) else {
            return Vec::new();
        };
        self.items
            .iter()
            .filter(|item| item.covers_date(target))
            .map(|item| TaskOccurrence {
                start_date: from_naive_date(item.start_date),
                task: item.to_view_task(),
            })
            .collect()
    }

    /// Adds a new task to `date`, trimming whitespace and ignoring empty titles.
    pub fn add_task(&mut self, date: Date, draft: TaskDraft<'_>) -> bool {
        let trimmed = draft.title.trim();
        let Some(start_date) = to_naive_date(date) else {
            return false;
        };
        if trimmed.is_empty() {
            return false;
        }
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        let schedule = parse_schedule(draft.time, draft.end_time);

        self.items.push(AgendaItem {
            id,
            title: trimmed.to_owned(),
            notes: draft.notes.trim().to_owned(),
            start_date,
            span_days: draft.span_days.clamp(1, 14),
            schedule,
            recurrence: draft.recurrence,
            tag: draft.tag.trim().to_owned(),
            location: draft.location.trim().to_owned(),
            color: draft.color,
            kind: ItemKind::Task {
                completed: false,
                priority: draft.priority,
                auto_rollover: false,
            },
        });
        true
    }

    /// Adds an item directly (used for Events or recurring Tasks).
    #[allow(dead_code)]
    pub fn add_item(&mut self, mut item: AgendaItem) -> u64 {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        item.id = id;
        self.items.push(item);
        id
    }

    /// Toggles completion state for task `id`.
    pub fn toggle_task(&mut self, _date: Date, id: u64) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            if let ItemKind::Task { completed, .. } = &mut item.kind {
                *completed = !*completed;
                return true;
            }
        }
        false
    }

    /// Updates the custom RGB color for item `id`.
    #[allow(dead_code)]
    pub fn set_task_color(&mut self, _date: Date, id: u64, color: Option<[u8; 3]>) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            if item.color != color {
                item.color = color;
                return true;
            }
        }
        false
    }

    /// Deletes item `id`.
    pub fn delete_task(&mut self, _date: Date, id: u64) -> bool {
        let before = self.items.len();
        self.items.retain(|i| i.id != id);
        self.items.len() != before
    }

    /// Updates an item's date and `TimeRange` schedule (in minutes since midnight).
    pub fn update_item_schedule(
        &mut self,
        id: u64,
        new_date: Option<Date>,
        start_min: u32,
        end_min: u32,
    ) -> bool {
        let Some(item) = self.items.iter_mut().find(|i| i.id == id) else {
            return false;
        };
        let s_sec = (start_min.min(1425)) * 60;
        let e_sec = (end_min.max(start_min + 15).min(1439)) * 60;
        let (Some(start), Some(end)) = (
            NaiveTime::from_num_seconds_from_midnight_opt(s_sec, 0),
            NaiveTime::from_num_seconds_from_midnight_opt(e_sec, 0),
        ) else {
            return false;
        };
        let mut changed = false;
        if let Some(d) = new_date.and_then(to_naive_date) {
            if item.start_date != d {
                item.start_date = d;
                changed = true;
            }
        }
        let next_sched = Schedule::TimeRange { start, end };
        if item.schedule != next_sched {
            item.schedule = next_sched;
            changed = true;
        }
        changed
    }

    /// Moves item `id` to the following calendar day.
    #[allow(dead_code)]
    pub fn move_task_to_next_day(&mut self, _date: Date, id: u64) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            if let Some(next) = item.start_date.succ_opt() {
                item.start_date = next;
                return true;
            }
        }
        false
    }

    /// Removes all completed tasks visible on `date`.
    #[allow(dead_code)]
    pub fn clear_completed(&mut self, date: Date) -> bool {
        let Some(target) = to_naive_date(date) else {
            return false;
        };
        let before = self.items.len();
        self.items
            .retain(|item| !(item.covers_date(target) && item.is_completed()));
        self.items.len() != before
    }

    /// Returns `(total_minutes, category -> (count, minutes))` for the 7-day week starting at `week_monday`.
    pub fn week_category_stats(
        &self,
        week_monday: Date,
    ) -> (i64, BTreeMap<String, (usize, i64)>) {
        let mut total_mins = 0_i64;
        let mut stats: BTreeMap<String, (usize, i64)> = BTreeMap::new();
        for offset in 0..7 {
            let day = week_monday.add_days(offset);
            for occ in self.tasks_visible_on(day) {
                let mins = occ
                    .task
                    .time_bounds_minutes()
                    .map(|(s, e)| i64::from(e - s))
                    .unwrap_or(60);
                total_mins += mins;
                let cat = if occ.task.tag.is_empty() {
                    "General".to_owned()
                } else {
                    occ.task.tag
                };
                let entry = stats.entry(cat).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += mins;
            }
        }
        (total_mins, stats)
    }

    /// Counts `(pending, completed)` items visible on `date`.
    pub fn counts_for(&self, date: Date) -> (usize, usize) {
        let tasks = self.tasks_visible_on(date);
        let done = tasks.iter().filter(|occ| occ.task.done).count();
        (tasks.len() - done, done)
    }

    /// Counts `(pending, completed)` items starting in `(year, month)`.
    pub fn month_counts(&self, year: i32, month: u8) -> (usize, usize) {
        let mut pending = 0;
        let mut done = 0;
        for item in &self.items {
            if item.start_date.year() == year && item.start_date.month() == u32::from(month) {
                if item.is_completed() {
                    done += 1;
                } else {
                    pending += 1;
                }
            }
        }
        (pending, done)
    }
}

/// Legacy v1 persistence schema for zero-panic migration.
#[derive(Deserialize)]
struct LegacyTask {
    id: u64,
    title: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    time: String,
    #[serde(default)]
    end_time: String,
    #[serde(default)]
    location: String,
    #[serde(default)]
    tag: String,
    #[serde(default = "default_span_days")]
    span_days: u8,
    #[serde(default)]
    color: Option<[u8; 3]>,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    priority: Priority,
}

#[derive(Deserialize)]
struct LegacyTaskStore {
    #[serde(default = "default_next_id")]
    next_id: u64,
    #[serde(default)]
    by_date: BTreeMap<String, Vec<LegacyTask>>,
    #[serde(default)]
    daily_notes: BTreeMap<String, String>,
    #[serde(default)]
    weekly_intentions: BTreeMap<String, String>,
}

fn parse_any_store_json(raw: &[u8]) -> io::Result<TaskStore> {
    if let Ok(v2) = serde_json::from_slice::<TaskStore>(raw) {
        if v2.schema_version >= CURRENT_SCHEMA_VERSION {
            return Ok(v2);
        }
    }

    let legacy: LegacyTaskStore = serde_json::from_slice(raw)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let mut items = Vec::new();
    for (date_key, list) in legacy.by_date {
        let Ok(start_date) = NaiveDate::parse_from_str(&date_key, "%Y-%m-%d") else {
            continue;
        };
        for t in list {
            items.push(AgendaItem {
                id: t.id,
                title: t.title,
                notes: t.notes,
                start_date,
                span_days: t.span_days.clamp(1, 14),
                schedule: parse_schedule(&t.time, &t.end_time),
                recurrence: Recurrence::None,
                tag: t.tag,
                location: t.location,
                color: t.color,
                kind: ItemKind::Task {
                    completed: t.done,
                    priority: t.priority,
                    auto_rollover: false,
                },
            });
        }
    }

    Ok(TaskStore {
        schema_version: CURRENT_SCHEMA_VERSION,
        next_id: legacy.next_id.max(1),
        items,
        daily_notes: legacy.daily_notes,
        weekly_intentions: legacy.weekly_intentions,
        last_notified_date: None,
    })
}

fn parse_schedule(start_raw: &str, end_raw: &str) -> Schedule {
    let Some(start) = parse_hm(start_raw) else {
        return Schedule::AllDay;
    };
    let end = parse_hm(end_raw).unwrap_or(start);
    Schedule::TimeRange { start, end }
}

fn parse_hm(raw: &str) -> Option<NaiveTime> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    NaiveTime::parse_from_str(trimmed, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(trimmed, "%-H:%M"))
        .ok()
}

fn to_naive_date(date: Date) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(date.year, u32::from(date.month), u32::from(date.day))
}

fn from_naive_date(d: NaiveDate) -> Date {
    Date::new(d.year(), d.month() as u8, d.day() as u8)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next.and_then(|d| d.pred_opt()).map(|d| d.day()).unwrap_or(28)
}

fn encode_store(store: &TaskStore) -> io::Result<Vec<u8>> {
    let json = serde_json::to_vec(store)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let mut out = Vec::with_capacity(DATA_MAGIC.len() + json.len());
    out.extend_from_slice(DATA_MAGIC);
    for (idx, byte) in json.iter().enumerate() {
        out.push(byte ^ DATA_KEY[idx % DATA_KEY.len()]);
    }
    Ok(out)
}

fn decode_store(bytes: &[u8]) -> io::Result<TaskStore> {
    if bytes.len() < DATA_MAGIC.len() || &bytes[..DATA_MAGIC.len()] != DATA_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Formato de datos inválido",
        ));
    }
    let decoded: Vec<u8> = bytes[DATA_MAGIC.len()..]
        .iter()
        .enumerate()
        .map(|(idx, byte)| byte ^ DATA_KEY[idx % DATA_KEY.len()])
        .collect();
    parse_any_store_json(&decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_task_should_ignore_empty_title() {
        let mut store = TaskStore::default();
        let date = Date::new(2026, 11, 17);
        assert!(!store.add_task(
            date,
            TaskDraft {
                title: "   ",
                notes: "",
                time: "",
                end_time: "",
                location: "",
                tag: "",
                span_days: 1,
                recurrence: Recurrence::None,
                color: None,
                priority: Priority::Normal,
            }
        ));
        assert!(store.tasks_visible_on(date).is_empty());
    }

    #[test]
    fn toggle_move_and_delete_should_update_counts() {
        let mut store = TaskStore::default();
        let date = Date::new(2026, 11, 30);
        assert!(store.add_task(
            date,
            TaskDraft {
                title: "Revisar Rust",
                notes: "LTO + strip",
                time: "10:00",
                end_time: "11:30",
                location: "Oficina",
                tag: "Trabajo",
                span_days: 1,
                recurrence: Recurrence::None,
                color: None,
                priority: Priority::High,
            }
        ));
        assert_eq!(store.counts_for(date), (1, 0));

        let id = store.tasks_visible_on(date)[0].task.id;
        assert!(store.move_task_to_next_day(date, id));
        assert_eq!(store.counts_for(date), (0, 0));

        let dec_1 = Date::new(2026, 12, 1);
        assert_eq!(store.counts_for(dec_1), (1, 0));
        assert!(store.toggle_task(dec_1, id));
        assert_eq!(store.counts_for(dec_1), (0, 1));
        assert!(store.clear_completed(dec_1));
        assert_eq!(store.counts_for(dec_1), (0, 0));
    }

    #[test]
    fn encode_decode_roundtrip_should_not_expose_plaintext_json() {
        let mut store = TaskStore::default();
        let date = Date::new(2026, 11, 17);
        assert!(store.add_task(
            date,
            TaskDraft {
                title: "Secreta",
                notes: "",
                time: "09:00",
                end_time: "",
                location: "",
                tag: "Personal",
                span_days: 1,
                recurrence: Recurrence::None,
                color: None,
                priority: Priority::High,
            }
        ));

        let bytes = encode_store(&store).expect("encode should succeed");
        assert!(bytes.starts_with(DATA_MAGIC));
        assert!(!bytes.windows(7).any(|w| w == b"Secreta"));

        let decoded = decode_store(&bytes).expect("decode should succeed");
        assert_eq!(decoded.tasks_visible_on(date).len(), 1);
        assert_eq!(decoded.tasks_visible_on(date)[0].task.title, "Secreta");
    }

    #[test]
    fn weekly_task_should_be_visible_across_all_seven_days() {
        let mut store = TaskStore::default();
        let start = Date::new(2026, 11, 17);
        assert!(store.add_task(
            start,
            TaskDraft {
                title: "Sprint",
                notes: "",
                time: "",
                end_time: "",
                location: "",
                tag: "Trabajo",
                span_days: 7,
                recurrence: Recurrence::None,
                color: Some([59, 130, 246]),
                priority: Priority::Normal,
            }
        ));

        for offset in 0..7 {
            let day = start.add_days(offset);
            assert_eq!(store.tasks_visible_on(day).len(), 1);
        }
        assert!(store.tasks_visible_on(start.add_days(7)).is_empty());
    }

    #[test]
    fn legacy_v1_payload_should_migrate_cleanly_without_panic() {
        let v1_json = br#"{
            "next_id": 5,
            "by_date": {
                "2026-11-23": [
                    {
                        "id": 4,
                        "title": "Planificar semana",
                        "notes": "Notas v1",
                        "time": "09:30",
                        "end_time": "10:45",
                        "location": "Estudio",
                        "tag": "Trabajo",
                        "span_days": 1,
                        "done": false,
                        "priority": "High"
                    }
                ]
            },
            "daily_notes": { "2026-11-23": "Recordatorio" }
        }"#;

        let migrated = parse_any_store_json(v1_json).expect("v1 migration should succeed");
        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.items.len(), 1);
        assert_eq!(migrated.items[0].title, "Planificar semana");
        assert_eq!(
            migrated.items[0].schedule.duration(),
            Some(Duration::minutes(75))
        );
        assert_eq!(
            migrated.daily_notes.get("2026-11-23").map(String::as_str),
            Some("Recordatorio")
        );
    }

    #[test]
    fn recurrence_and_auto_rollover_should_behave_accurately() {
        let mut store = TaskStore::default();
        let monday = NaiveDate::from_ymd_opt(2026, 11, 23).expect("valid date");
        let saturday = NaiveDate::from_ymd_opt(2026, 11, 28).expect("valid date");

        store.add_item(AgendaItem {
            id: 0,
            title: "Standup".to_owned(),
            notes: String::new(),
            start_date: monday,
            span_days: 1,
            schedule: Schedule::AllDay,
            recurrence: Recurrence::Weekdays,
            tag: "Trabajo".to_owned(),
            location: String::new(),
            color: None,
            kind: ItemKind::Event {
                location: "Meet".to_owned(),
            },
        });

        assert_eq!(store.tasks_visible_on(Date::new(2026, 11, 24)).len(), 1);
        assert!(!Recurrence::Weekdays.matches(monday, saturday));

        let overdue_id = store.add_item(AgendaItem {
            id: 0,
            title: "Pendiente ayer".to_owned(),
            notes: String::new(),
            start_date: monday,
            span_days: 1,
            schedule: Schedule::AllDay,
            recurrence: Recurrence::None,
            tag: "Personal".to_owned(),
            location: String::new(),
            color: None,
            kind: ItemKind::Task {
                completed: false,
                priority: Priority::High,
                auto_rollover: true,
            },
        });

        let wednesday = NaiveDate::from_ymd_opt(2026, 11, 25).expect("valid date");
        assert!(store.rollover_overdue(wednesday));
        let rolled = store.items.iter().find(|i| i.id == overdue_id).expect("exists");
        assert_eq!(rolled.start_date, wednesday);
    }
}
