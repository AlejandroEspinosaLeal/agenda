use crate::model::Priority;

/// Result of parsing a quick natural-language command palette entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEntry {
    pub title: String,
    pub date_offset_days: i64,
    pub start_time: String,
    pub end_time: String,
    pub tag: String,
    pub priority: Priority,
}

/// Parses natural-language task/event input such as:
/// `"Estudiar Rust 18:00-19:30 #Estudio"` or `"Reunión mañana 10:00 1h #Trabajo"`.
pub fn parse_quick_entry(input: &str) -> Option<ParsedEntry> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with('>') {
        return None;
    }

    let mut date_offset_days: i64 = 0;
    let mut start_min: Option<u32> = None;
    let mut end_min: Option<u32> = None;
    let mut duration_mins: Option<u32> = None;
    let mut tag = String::new();
    let mut priority = Priority::Normal;
    let mut title_parts: Vec<&str> = Vec::new();

    for token in trimmed.split_whitespace() {
        let lower = token.to_lowercase();

        if let Some(raw_tag) = token.strip_prefix('#') {
            tag = normalize_tag(raw_tag);
            continue;
        }
        if lower == "!alta" {
            priority = Priority::High;
            continue;
        }
        if lower == "!media" {
            priority = Priority::Normal;
            continue;
        }
        if lower == "!baja" {
            priority = Priority::Low;
            continue;
        }
        if lower == "hoy" {
            date_offset_days = 0;
            continue;
        }
        if lower == "mañana" || lower == "manana" {
            date_offset_days = 1;
            continue;
        }
        if lower == "pasado" {
            date_offset_days = 2;
            continue;
        }
        if let Some((s, e)) = parse_time_range(&lower) {
            start_min = Some(s);
            end_min = Some(e);
            continue;
        }
        if start_min.is_none() {
            if let Some(s) = parse_hh_mm(&lower) {
                start_min = Some(s);
                continue;
            }
        }
        if let Some(d) = parse_duration_token(&lower) {
            duration_mins = Some(d);
            continue;
        }

        title_parts.push(token);
    }

    let title = title_parts.join(" ");
    if title.is_empty() {
        return None;
    }

    let (start_time, end_time) = match (start_min, end_min, duration_mins) {
        (Some(s), Some(e), _) => (fmt_min(s), fmt_min(e)),
        (Some(s), None, Some(d)) => (fmt_min(s), fmt_min((s + d).min(1439))),
        (Some(s), None, None) => (fmt_min(s), fmt_min((s + 60).min(1439))),
        _ => (String::new(), String::new()),
    };

    Some(ParsedEntry {
        title,
        date_offset_days,
        start_time,
        end_time,
        tag,
        priority,
    })
}

fn normalize_tag(raw: &str) -> String {
    match raw.to_lowercase().as_str() {
        "trabajo" => "Trabajo".to_owned(),
        "personal" => "Personal".to_owned(),
        "estudio" => "Estudio".to_owned(),
        "salud" => "Salud".to_owned(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

fn parse_time_range(tok: &str) -> Option<(u32, u32)> {
    let (a, b) = tok.split_once('-')?;
    let s = parse_hh_mm(a)?;
    let e = parse_hh_mm(b)?;
    if e > s {
        Some((s, e))
    } else {
        Some((s, (s + 30).min(1439)))
    }
}

fn parse_hh_mm(tok: &str) -> Option<u32> {
    let (h_str, m_str) = tok.split_once(':')?;
    let h: u32 = h_str.parse().ok()?;
    let m: u32 = m_str.parse().ok()?;
    if h < 24 && m < 60 {
        Some(h * 60 + m)
    } else {
        None
    }
}

fn parse_duration_token(tok: &str) -> Option<u32> {
    if let Some(m) = tok.strip_suffix('m') {
        let mins: u32 = m.parse().ok()?;
        return Some(mins.clamp(15, 720));
    }
    if let Some(h) = tok.strip_suffix('h') {
        let hrs: f32 = h.parse().ok()?;
        if hrs > 0.0 && hrs <= 12.0 {
            return Some((hrs * 60.0).round() as u32);
        }
    }
    None
}

fn fmt_min(min: u32) -> String {
    format!("{:02}:{:02}", (min / 60).min(23), min % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_should_extract_time_range_and_hashtag() {
        let parsed = parse_quick_entry("Estudiar Rust 18:00-19:30 #Estudio")
            .expect("should parse quick entry");
        assert_eq!(parsed.title, "Estudiar Rust");
        assert_eq!(parsed.date_offset_days, 0);
        assert_eq!(parsed.start_time, "18:00");
        assert_eq!(parsed.end_time, "19:30");
        assert_eq!(parsed.tag, "Estudio");
    }

    #[test]
    fn parser_should_extract_relative_day_and_duration() {
        let parsed = parse_quick_entry("Reunión mañana 10:00 1h #Trabajo !alta")
            .expect("should parse quick entry");
        assert_eq!(parsed.title, "Reunión");
        assert_eq!(parsed.date_offset_days, 1);
        assert_eq!(parsed.start_time, "10:00");
        assert_eq!(parsed.end_time, "11:00");
        assert_eq!(parsed.tag, "Trabajo");
        assert_eq!(parsed.priority, Priority::High);
    }

    #[test]
    fn parser_should_handle_plain_all_day_title() {
        let parsed = parse_quick_entry("Llamar al banco").expect("should parse");
        assert_eq!(parsed.title, "Llamar al banco");
        assert!(parsed.start_time.is_empty());
        assert!(parsed.end_time.is_empty());
    }
}
