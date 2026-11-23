use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::calendar::Date;

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

const fn default_span_days() -> u8 {
    1
}

/// A single task attached to a start calendar day (optionally spanning multiple days).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub end_time: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default = "default_span_days")]
    pub span_days: u8,
    #[serde(default)]
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

/// Borrowed input fields for creating a new [`Task`].
pub struct TaskDraft<'a> {
    pub title: &'a str,
    pub notes: &'a str,
    pub time: &'a str,
    pub end_time: &'a str,
    pub location: &'a str,
    pub tag: &'a str,
    pub span_days: u8,
    pub color: Option<[u8; 3]>,
    pub priority: Priority,
}

/// Persistent collection of tasks keyed by `YYYY-MM-DD`, plus daily notes and weekly intentions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskStore {
    pub next_id: u64,
    pub by_date: BTreeMap<String, Vec<Task>>,
    #[serde(default)]
    pub daily_notes: BTreeMap<String, String>,
    #[serde(default)]
    pub weekly_intentions: BTreeMap<String, String>,
}

const DATA_MAGIC: &[u8; 4] = b"OTSK";
const DATA_KEY: &[u8; 8] = b"rU5t_T4s";

impl TaskStore {
    /// Returns the app-data storage path (`%LOCALAPPDATA%\organizador_tareas\tareas.dat`)
    /// so no visible `.json` file is created on the user's Desktop.
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

    /// Loads tasks from `path`, migrating a legacy `tareas.json` next to the executable if present.
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read(path) {
            Ok(bytes) => decode_store(&bytes),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if let Some(migrated) = Self::try_migrate_legacy_json(path) {
                    return Ok(migrated);
                }
                Ok(Self {
                    next_id: 1,
                    by_date: BTreeMap::new(),
                    daily_notes: BTreeMap::new(),
                    weekly_intentions: BTreeMap::new(),
                })
            }
            Err(err) => Err(err),
        }
    }

    fn try_migrate_legacy_json(target_path: &Path) -> Option<Self> {
        let exe = std::env::current_exe().ok()?;
        let legacy = exe.parent()?.join("tareas.json");
        let raw = fs::read_to_string(&legacy).ok()?;
        let store: Self = serde_json::from_str(&raw).ok()?;
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

    /// Returns tasks whose start date is `date`.
    pub fn tasks_for(&self, date: Date) -> &[Task] {
        self.by_date
            .get(&date.to_key())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Returns all tasks visible on `date`, including multi-day tasks started up to 13 days earlier.
    pub fn tasks_visible_on(&self, date: Date) -> Vec<TaskOccurrence> {
        let mut out = Vec::new();
        for offset in (0..=13).rev() {
            let start = date.add_days(-offset);
            for task in self.tasks_for(start) {
                let span = i64::from(task.span_days.max(1));
                if offset < span {
                    out.push(TaskOccurrence {
                        start_date: start,
                        task: task.clone(),
                    });
                }
            }
        }
        out
    }

    /// Adds a new task to `date`, trimming whitespace and ignoring empty titles.
    pub fn add_task(&mut self, date: Date, draft: TaskDraft<'_>) -> bool {
        let trimmed = draft.title.trim();
        if trimmed.is_empty() {
            return false;
        }
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        let task = Task {
            id,
            title: trimmed.to_owned(),
            notes: draft.notes.trim().to_owned(),
            time: draft.time.trim().to_owned(),
            end_time: draft.end_time.trim().to_owned(),
            location: draft.location.trim().to_owned(),
            tag: draft.tag.trim().to_owned(),
            span_days: draft.span_days.clamp(1, 14),
            color: draft.color,
            done: false,
            priority: draft.priority,
        };
        self.by_date.entry(date.to_key()).or_default().push(task);
        true
    }

    /// Toggles completion state for task `id` on `date`.
    pub fn toggle_task(&mut self, date: Date, id: u64) -> bool {
        let Some(list) = self.by_date.get_mut(&date.to_key()) else {
            return false;
        };
        if let Some(task) = list.iter_mut().find(|t| t.id == id) {
            task.done = !task.done;
            return true;
        }
        false
    }

    /// Updates the custom RGB color for task `id` on `date`.
    pub fn set_task_color(&mut self, date: Date, id: u64, color: Option<[u8; 3]>) -> bool {
        let Some(list) = self.by_date.get_mut(&date.to_key()) else {
            return false;
        };
        if let Some(task) = list.iter_mut().find(|t| t.id == id) {
            if task.color != color {
                task.color = color;
                return true;
            }
        }
        false
    }

    /// Deletes task `id` from `date`, removing the date entry if it becomes empty.
    pub fn delete_task(&mut self, date: Date, id: u64) -> bool {
        let key = date.to_key();
        let Some(list) = self.by_date.get_mut(&key) else {
            return false;
        };
        let before = list.len();
        list.retain(|t| t.id != id);
        let removed = list.len() != before;
        if list.is_empty() {
            self.by_date.remove(&key);
        }
        removed
    }

    /// Moves task `id` from `date` to the following calendar day.
    pub fn move_task_to_next_day(&mut self, date: Date, id: u64) -> bool {
        let key = date.to_key();
        let Some(list) = self.by_date.get_mut(&key) else {
            return false;
        };
        let Some(pos) = list.iter().position(|t| t.id == id) else {
            return false;
        };
        let task = list.remove(pos);
        if list.is_empty() {
            self.by_date.remove(&key);
        }
        let next = date.add_days(1);
        self.by_date.entry(next.to_key()).or_default().push(task);
        true
    }

    /// Removes all completed tasks on `date`.
    pub fn clear_completed(&mut self, date: Date) -> bool {
        let key = date.to_key();
        let Some(list) = self.by_date.get_mut(&key) else {
            return false;
        };
        let before = list.len();
        list.retain(|t| !t.done);
        let removed = list.len() != before;
        if list.is_empty() {
            self.by_date.remove(&key);
        }
        removed
    }

    /// Returns `(total_tasks, category_counts)` for the 7-day week starting at `week_monday`.
    pub fn week_category_counts(&self, week_monday: Date) -> (usize, BTreeMap<String, usize>) {
        let mut total = 0usize;
        let mut counts = BTreeMap::new();
        for offset in 0..7 {
            let day = week_monday.add_days(offset);
            for occ in self.tasks_visible_on(day) {
                total += 1;
                let cat = if occ.task.tag.is_empty() {
                    "General".to_owned()
                } else {
                    occ.task.tag
                };
                *counts.entry(cat).or_insert(0) += 1;
            }
        }
        (total, counts)
    }

    /// Counts `(pending, completed)` tasks visible on `date`.
    pub fn counts_for(&self, date: Date) -> (usize, usize) {
        let tasks = self.tasks_visible_on(date);
        let done = tasks.iter().filter(|occ| occ.task.done).count();
        (tasks.len() - done, done)
    }

    /// Counts `(pending, completed)` tasks across a given `(year, month)`.
    pub fn month_counts(&self, year: i32, month: u8) -> (usize, usize) {
        let prefix = format!("{:04}-{:02}-", year, month);
        let mut pending = 0;
        let mut done = 0;
        for (key, list) in &self.by_date {
            if key.starts_with(&prefix) {
                for task in list {
                    if task.done {
                        done += 1;
                    } else {
                        pending += 1;
                    }
                }
            }
        }
        (pending, done)
    }
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
    serde_json::from_slice(&decoded)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
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
                color: None,
                priority: Priority::Normal,
            }
        ));
        assert!(store.tasks_for(date).is_empty());
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
                color: None,
                priority: Priority::High,
            }
        ));
        assert_eq!(store.counts_for(date), (1, 0));

        let id = store.tasks_for(date)[0].id;
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
                color: None,
                priority: Priority::High,
            }
        ));

        let bytes = encode_store(&store).expect("encode should succeed");
        assert!(bytes.starts_with(DATA_MAGIC));
        assert!(!bytes.windows(7).any(|w| w == b"Secreta"));

        let decoded = decode_store(&bytes).expect("decode should succeed");
        assert_eq!(decoded.tasks_for(date).len(), 1);
        assert_eq!(decoded.tasks_for(date)[0].title, "Secreta");
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
}
