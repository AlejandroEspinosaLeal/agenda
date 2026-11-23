use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPreset {
    Pomodoro25,
    DeepWork50,
    BlockRemaining(u64),
}

impl FocusPreset {
    pub const fn work_secs(self) -> u64 {
        match self {
            Self::Pomodoro25 => 25 * 60,
            Self::DeepWork50 => 50 * 60,
            Self::BlockRemaining(secs) => {
                if secs == 0 {
                    15 * 60
                } else {
                    secs
                }
            }
        }
    }

    pub const fn break_secs(self) -> u64 {
        match self {
            Self::Pomodoro25 => 5 * 60,
            Self::DeepWork50 => 10 * 60,
            Self::BlockRemaining(_) => 5 * 60,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Pomodoro25 => "Pomodoro 25/5",
            Self::DeepWork50 => "Deep Work 50/10",
            Self::BlockRemaining(_) => "Restante del bloque",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPhase {
    Idle,
    Running { is_break: bool, remaining_secs: u64 },
    Paused { is_break: bool, remaining_secs: u64 },
}

/// State machine for the Focus / Pomodoro timer.
/// Ticks only while `is_running()` is true (zero CPU when idle or paused).
pub struct FocusTimer {
    pub preset: FocusPreset,
    pub phase: FocusPhase,
    pub active_item_id: Option<u64>,
    last_tick: Option<Instant>,
}

impl FocusTimer {
    pub fn new() -> Self {
        Self {
            preset: FocusPreset::Pomodoro25,
            phase: FocusPhase::Idle,
            active_item_id: None,
            last_tick: None,
        }
    }

    pub const fn is_running(&self) -> bool {
        matches!(self.phase, FocusPhase::Running { .. })
    }

    pub fn display_secs(&self) -> u64 {
        match self.phase {
            FocusPhase::Idle => self.preset.work_secs(),
            FocusPhase::Running { remaining_secs, .. }
            | FocusPhase::Paused { remaining_secs, .. } => remaining_secs,
        }
    }

    pub fn is_break(&self) -> bool {
        matches!(
            self.phase,
            FocusPhase::Running { is_break: true, .. }
                | FocusPhase::Paused { is_break: true, .. }
        )
    }

    pub fn start_or_resume(&mut self) {
        match self.phase {
            FocusPhase::Idle => {
                self.phase = FocusPhase::Running {
                    is_break: false,
                    remaining_secs: self.preset.work_secs(),
                };
                self.last_tick = Some(Instant::now());
            }
            FocusPhase::Paused {
                is_break,
                remaining_secs,
            } => {
                self.phase = FocusPhase::Running {
                    is_break,
                    remaining_secs,
                };
                self.last_tick = Some(Instant::now());
            }
            FocusPhase::Running { .. } => {}
        }
    }

    pub fn pause(&mut self) {
        if let FocusPhase::Running {
            is_break,
            remaining_secs,
        } = self.phase
        {
            self.phase = FocusPhase::Paused {
                is_break,
                remaining_secs,
            };
            self.last_tick = None;
        }
    }

    pub fn reset(&mut self) {
        self.phase = FocusPhase::Idle;
        self.last_tick = None;
    }

    pub fn set_preset(&mut self, preset: FocusPreset) {
        self.preset = preset;
        self.reset();
    }

    /// Advances the timer using elapsed wall-clock seconds only when running.
    pub fn tick(&mut self) {
        let FocusPhase::Running {
            is_break,
            remaining_secs,
        } = self.phase
        else {
            return;
        };
        let now = Instant::now();
        let Some(prev) = self.last_tick else {
            self.last_tick = Some(now);
            return;
        };
        let elapsed = now.duration_since(prev).as_secs();
        if elapsed == 0 {
            return;
        }
        self.last_tick = Some(now);

        if elapsed >= remaining_secs {
            let next_break = !is_break;
            let next_secs = if next_break {
                self.preset.break_secs()
            } else {
                self.preset.work_secs()
            };
            self.phase = FocusPhase::Running {
                is_break: next_break,
                remaining_secs: next_secs,
            };
        } else {
            self.phase = FocusPhase::Running {
                is_break,
                remaining_secs: remaining_secs - elapsed,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_timer_state_transitions() {
        let mut t = FocusTimer::new();
        assert!(!t.is_running());
        assert_eq!(t.display_secs(), 25 * 60);

        t.start_or_resume();
        assert!(t.is_running());

        t.pause();
        assert!(!t.is_running());
        assert!(matches!(t.phase, FocusPhase::Paused { .. }));

        t.set_preset(FocusPreset::BlockRemaining(900));
        assert!(!t.is_running());
        assert_eq!(t.display_secs(), 900);
    }
}
