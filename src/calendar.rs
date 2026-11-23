use std::time::{SystemTime, UNIX_EPOCH};

/// Simple calendar date (year, month 1..=12, day 1..=31).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub const fn new(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    /// Formats the date as `YYYY-MM-DD` for stable JSON storage keys.
    pub fn to_key(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Returns a new `Date` shifted by `delta` days (positive or negative).
    pub fn add_days(self, delta: i64) -> Self {
        let days = days_from_civil(self.year, self.month, self.day) + delta;
        civil_from_days(days)
    }

    /// Returns signed number of days from `self` to `other`.
    pub fn days_until(self, other: Self) -> i64 {
        days_from_civil(other.year, other.month, other.day)
            - days_from_civil(self.year, self.month, self.day)
    }

    /// Returns today's date (UTC-based via `SystemTime`).
    pub fn today() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let days = secs.div_euclid(86_400);
        civil_from_days(days)
    }
}

/// Returns `true` if `year` is a Gregorian leap year.
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Returns the number of days in `(year, month)` where `month` is `1..=12`.
pub const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Returns the weekday index (`0 = Monday .. 6 = Sunday`) for `1`st of `(year, month)`.
pub fn first_weekday_monday_zero(year: i32, month: u8) -> u8 {
    let days = days_from_civil(year, month, 1);
    // 1970-01-01 was a Thursday (index 3 in Mon=0..Sun=6).
    (days + 3).rem_euclid(7) as u8
}

/// Returns the Spanish name of a month (`1..=12`).
pub const fn month_name_es(month: u8) -> &'static str {
    match month {
        1 => "Enero",
        2 => "Febrero",
        3 => "Marzo",
        4 => "Abril",
        5 => "Mayo",
        6 => "Junio",
        7 => "Julio",
        8 => "Agosto",
        9 => "Septiembre",
        10 => "Octubre",
        11 => "Noviembre",
        12 => "Diciembre",
        _ => "Mes",
    }
}

/// Returns the 3-letter Spanish abbreviation of a month (`1..=12`).
pub const fn month_short_es(month: u8) -> &'static str {
    match month {
        1 => "Ene",
        2 => "Feb",
        3 => "Mar",
        4 => "Abr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Ago",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dic",
        _ => "Mes",
    }
}

/// Returns the Spanish weekday name for `0 = Monday .. 6 = Sunday`.
pub const fn weekday_name_es(weekday_mon_zero: u8) -> &'static str {
    match weekday_mon_zero {
        0 => "Lunes",
        1 => "Martes",
        2 => "Miércoles",
        3 => "Jueves",
        4 => "Viernes",
        5 => "Sábado",
        6 => "Domingo",
        _ => "Día",
    }
}

impl Date {
    /// Returns weekday index (`0 = Monday .. 6 = Sunday`).
    pub fn weekday_monday_zero(self) -> u8 {
        let days = days_from_civil(self.year, self.month, self.day);
        (days + 3).rem_euclid(7) as u8
    }

    /// Returns the Monday of the week containing `self`.
    pub fn week_monday(self) -> Self {
        let offset = i64::from(self.weekday_monday_zero());
        self.add_days(-offset)
    }

    /// Returns approximate/ISO week number (`1..=53`).
    pub fn iso_week(self) -> u32 {
        let thursday = self.week_monday().add_days(3);
        let jan1 = days_from_civil(thursday.year, 1, 1);
        let thu_days = days_from_civil(thursday.year, thursday.month, thursday.day);
        ((thu_days - jan1) / 7 + 1) as u32
    }
}

/// Advances `(year, month)` by `delta` months (`+1` or `-1`).
pub fn shift_month(year: i32, month: u8, delta: i32) -> (i32, u8) {
    let total = year * 12 + (month as i32 - 1) + delta;
    let new_year = total.div_euclid(12);
    let new_month = (total.rem_euclid(12) + 1) as u8;
    (new_year, new_month)
}

/// Howard Hinnant's civil-from-days algorithm (days since 1970-01-01).
fn civil_from_days(z: i64) -> Date {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = (y + if m <= 2 { 1 } else { 0 }) as i32;
    Date::new(year, m as u8, d as u8)
}

/// Howard Hinnant's days-from-civil algorithm (days since 1970-01-01).
fn days_from_civil(y: i32, m: u8, d: u8) -> i64 {
    let y = y as i64 - if m <= 2 { 1 } else { 0 };
    let m = m as i64;
    let d = d as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as u64 + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_year_should_handle_century_rules() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2025));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
    }

    #[test]
    fn days_in_month_should_return_29_for_leap_february() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 11), 30);
    }

    #[test]
    fn first_weekday_should_match_known_dates() {
        // 2026-11-01 was a Sunday (6 in Mon=0..Sun=6)
        assert_eq!(first_weekday_monday_zero(2026, 11), 6);
        // 2026-06-01 was a Monday (0)
        assert_eq!(first_weekday_monday_zero(2026, 6), 0);
    }

    #[test]
    fn shift_month_should_wrap_year_boundaries() {
        assert_eq!(shift_month(2026, 1, -1), (2025, 12));
        assert_eq!(shift_month(2026, 12, 1), (2027, 1));
    }

    #[test]
    fn add_days_should_cross_month_and_year_boundaries() {
        let d = Date::new(2026, 12, 29);
        let next_week = d.add_days(6);
        assert_eq!(next_week, Date::new(2027, 1, 4));
    }

    #[test]
    fn week_monday_and_iso_week_should_match_nov_23_2026() {
        let d = Date::new(2026, 11, 23);
        assert_eq!(d.weekday_monday_zero(), 0);
        assert_eq!(d.week_monday(), d);
        assert_eq!(d.iso_week(), 48);
    }
}
