pub fn format_uptime(started_at: &str) -> String {
    let started = chrono::DateTime::parse_from_rfc3339(started_at);
    match started {
        Ok(t) => {
            let now = chrono::Utc::now();
            let diff = now.signed_duration_since(t);
            let total_minutes = diff.num_minutes();
            let hours = total_minutes / 60;
            let minutes = total_minutes % 60;
            let days = hours / 24;
            let hours = hours % 24;
            if days > 0 {
                format!("{}d {}h", days, hours)
            } else if hours > 0 {
                format!("{}h {:02}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            }
        }
        Err(_) => String::new(),
    }
}

pub fn format_viewers(count: u32) -> String {
    if count >= 1_000_000 {
        format!("{:.1}m", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

pub fn format_viewers_full(count: u32) -> String {
    let s = count.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_viewers() {
        assert_eq!(format_viewers(0), "0");
        assert_eq!(format_viewers(42), "42");
        assert_eq!(format_viewers(999), "999");
        assert_eq!(format_viewers(1000), "1.0k");
        assert_eq!(format_viewers(23400), "23.4k");
        assert_eq!(format_viewers(42391), "42.4k");
        assert_eq!(format_viewers(1000000), "1.0m");
        assert_eq!(format_viewers(2500000), "2.5m");
    }

    #[test]
    fn test_format_viewers_full() {
        assert_eq!(format_viewers_full(0), "0");
        assert_eq!(format_viewers_full(42), "42");
        assert_eq!(format_viewers_full(42391), "42,391");
        assert_eq!(format_viewers_full(1000000), "1,000,000");
    }

    #[test]
    fn test_format_uptime_minutes() {
        let five_min_ago = (chrono::Utc::now() - chrono::Duration::minutes(5))
            .to_rfc3339();
        assert_eq!(format_uptime(&five_min_ago), "5m");
    }

    #[test]
    fn test_format_uptime_hours_minutes() {
        let two_hours_ago = (chrono::Utc::now() - chrono::Duration::hours(2) - chrono::Duration::minutes(15))
            .to_rfc3339();
        assert_eq!(format_uptime(&two_hours_ago), "2h 15m");
    }

    #[test]
    fn test_format_uptime_days() {
        let one_day_ago = (chrono::Utc::now() - chrono::Duration::hours(27))
            .to_rfc3339();
        assert_eq!(format_uptime(&one_day_ago), "1d 3h");
    }

    #[test]
    fn test_format_uptime_invalid() {
        assert_eq!(format_uptime("not-a-date"), "");
    }
}