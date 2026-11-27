use chrono::{Local, Timelike};

/// Input interval for time-blocking layout (in minutes from midnight `0..=1440`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeInterval {
    pub id: u64,
    pub start_min: u32,
    pub end_min: u32,
}

/// Output placement of a block within parallel overlap columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedBlock {
    pub id: u64,
    pub start_min: u32,
    pub end_min: u32,
    pub col: usize,
    pub total_cols: usize,
}

/// Returns current local time in minutes since midnight (`0..1440`).
pub fn current_local_minutes() -> u32 {
    let now = Local::now();
    now.hour() * 60 + now.minute()
}

/// Snaps a minute-of-day value to the nearest 15-minute increment (`0..=1440`).
pub const fn snap_to_15(minutes: u32) -> u32 {
    let clamped = if minutes > 1440 { 1440 } else { minutes };
    let snapped = ((clamped + 7) / 15) * 15;
    if snapped > 1440 {
        1440
    } else {
        snapped
    }
}

/// Partitions overlapping time intervals into connected clusters and assigns parallel columns
/// so simultaneous blocks never cover each other.
pub fn layout_day_blocks(mut intervals: Vec<TimeInterval>) -> Vec<PlacedBlock> {
    if intervals.is_empty() {
        return Vec::new();
    }

    // Normalize zero/inverted durations to a 30-minute visual minimum
    for iv in &mut intervals {
        if iv.end_min <= iv.start_min {
            iv.end_min = (iv.start_min + 30).min(1440);
        }
    }

    intervals.sort_by_key(|iv| (iv.start_min, iv.end_min, iv.id));

    let mut placed = Vec::with_capacity(intervals.len());
    let mut cluster_start_idx = 0usize;

    while cluster_start_idx < intervals.len() {
        let mut cluster_max_end = intervals[cluster_start_idx].end_min;
        let mut cluster_end_idx = cluster_start_idx + 1;

        while cluster_end_idx < intervals.len()
            && intervals[cluster_end_idx].start_min < cluster_max_end
        {
            cluster_max_end = cluster_max_end.max(intervals[cluster_end_idx].end_min);
            cluster_end_idx += 1;
        }

        // Greedy column assignment within this connected overlap cluster
        let mut col_ends: Vec<u32> = Vec::new();
        let mut cluster_placed: Vec<(TimeInterval, usize)> =
            Vec::with_capacity(cluster_end_idx - cluster_start_idx);

        for iv in &intervals[cluster_start_idx..cluster_end_idx] {
            let assigned = col_ends
                .iter()
                .position(|&end| end <= iv.start_min)
                .unwrap_or_else(|| {
                    col_ends.push(0);
                    col_ends.len() - 1
                });
            col_ends[assigned] = iv.end_min;
            cluster_placed.push((*iv, assigned));
        }

        let total_cols = col_ends.len().max(1);
        for (iv, col) in cluster_placed {
            placed.push(PlacedBlock {
                id: iv.id,
                start_min: iv.start_min,
                end_min: iv.end_min,
                col,
                total_cols,
            });
        }

        cluster_start_idx = cluster_end_idx;
    }

    placed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_overlapping_blocks_should_use_single_column() {
        let out = layout_day_blocks(vec![
            TimeInterval {
                id: 1,
                start_min: 540,
                end_min: 600,
            },
            TimeInterval {
                id: 2,
                start_min: 600,
                end_min: 660,
            },
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].total_cols, 1);
        assert_eq!(out[1].total_cols, 1);
    }

    #[test]
    fn simultaneous_blocks_should_split_into_parallel_columns() {
        let out = layout_day_blocks(vec![
            TimeInterval {
                id: 1,
                start_min: 540,
                end_min: 630,
            },
            TimeInterval {
                id: 2,
                start_min: 570,
                end_min: 660,
            },
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].col, 0);
        assert_eq!(out[1].col, 1);
        assert_eq!(out[0].total_cols, 2);
        assert_eq!(out[1].total_cols, 2);
    }

    #[test]
    fn chained_overlap_should_reuse_freed_column() {
        // A: 9-11, B: 10-12, C: 11-13 -> A and C share col 0, B gets col 1
        let out = layout_day_blocks(vec![
            TimeInterval {
                id: 1,
                start_min: 540,
                end_min: 660,
            },
            TimeInterval {
                id: 2,
                start_min: 600,
                end_min: 720,
            },
            TimeInterval {
                id: 3,
                start_min: 660,
                end_min: 780,
            },
        ]);
        assert_eq!(out[0].col, 0);
        assert_eq!(out[1].col, 1);
        assert_eq!(out[2].col, 0);
        assert_eq!(out[0].total_cols, 2);
    }

    #[test]
    fn snap_to_15_should_round_to_quarter_hour_boundaries() {
        assert_eq!(snap_to_15(546), 540);
        assert_eq!(snap_to_15(548), 555);
        assert_eq!(snap_to_15(1450), 1440);
    }
}
