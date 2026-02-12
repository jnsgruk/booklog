use chrono::{DateTime, Datelike, Utc};

/// Format a datetime as a human-readable relative time string.
///
/// Examples: "Just now", "5m ago", "3h ago", "Yesterday", "4d ago", "2w ago",
/// "Mar 15" (same year), "Mar 15, 2024" (different year).
pub fn format_relative_time(dt: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let delta = now.signed_duration_since(dt);
    let secs = delta.num_seconds();

    if secs < 60 {
        return "Just now".to_string();
    }

    let mins = delta.num_minutes();
    if mins < 60 {
        return format!("{mins}m ago");
    }

    let hours = delta.num_hours();
    if hours < 24 {
        return format!("{hours}h ago");
    }

    let days = delta.num_days();
    if days == 1 {
        return "Yesterday".to_string();
    }
    if days < 7 {
        return format!("{days}d ago");
    }

    let weeks = days / 7;
    if days < 30 {
        return format!("{weeks}w ago");
    }

    if dt.year() == now.year() {
        dt.format("%b %d").to_string()
    } else {
        dt.format("%b %d, %Y").to_string()
    }
}

/// Format a page count for display.
pub fn format_pages(pages: i32) -> String {
    if pages == 1 {
        "1 page".to_string()
    } else {
        format!("{pages} pages")
    }
}

/// Format a star rating for display (0.5-5.0 in half-star increments).
pub fn format_rating(rating: f64) -> String {
    if rating.fract() == 0.0 {
        format!("{}/5", rating as i32)
    } else {
        format!("{rating}/5")
    }
}

/// Validate that a rating is a valid half-star value (0.5 to 5.0 in 0.5 increments).
pub fn is_valid_rating(rating: f64) -> bool {
    (0.5..=5.0).contains(&rating) && (rating * 2.0).fract() == 0.0
}

/// Em dash constant for use as a placeholder when a value is absent.
pub const EM_DASH: &str = "\u{2014}";

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, s).unwrap()
    }

    #[test]
    fn just_now_zero_seconds() {
        let now = utc(2025, 6, 1, 12, 0, 0);
        assert_eq!(format_relative_time(now, now), "Just now");
    }

    #[test]
    fn just_now_59_seconds() {
        let dt = utc(2025, 6, 1, 12, 0, 0);
        let now = utc(2025, 6, 1, 12, 0, 59);
        assert_eq!(format_relative_time(dt, now), "Just now");
    }

    #[test]
    fn minutes_boundary_60_seconds() {
        let dt = utc(2025, 6, 1, 12, 0, 0);
        let now = utc(2025, 6, 1, 12, 1, 0);
        assert_eq!(format_relative_time(dt, now), "1m ago");
    }

    #[test]
    fn minutes_ago() {
        let dt = utc(2025, 6, 1, 12, 0, 0);
        let now = utc(2025, 6, 1, 12, 45, 0);
        assert_eq!(format_relative_time(dt, now), "45m ago");
    }

    #[test]
    fn hours_boundary_60_minutes() {
        let dt = utc(2025, 6, 1, 12, 0, 0);
        let now = utc(2025, 6, 1, 13, 0, 0);
        assert_eq!(format_relative_time(dt, now), "1h ago");
    }

    #[test]
    fn hours_ago() {
        let dt = utc(2025, 6, 1, 6, 0, 0);
        let now = utc(2025, 6, 1, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "6h ago");
    }

    #[test]
    fn yesterday_exactly_24h() {
        let dt = utc(2025, 6, 1, 12, 0, 0);
        let now = utc(2025, 6, 2, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "Yesterday");
    }

    #[test]
    fn yesterday_36h() {
        let dt = utc(2025, 6, 1, 0, 0, 0);
        let now = utc(2025, 6, 2, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "Yesterday");
    }

    #[test]
    fn days_ago_2() {
        let dt = utc(2025, 6, 1, 12, 0, 0);
        let now = utc(2025, 6, 3, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "2d ago");
    }

    #[test]
    fn days_ago_6() {
        let dt = utc(2025, 6, 1, 12, 0, 0);
        let now = utc(2025, 6, 7, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "6d ago");
    }

    #[test]
    fn weeks_ago_1() {
        let dt = utc(2025, 6, 1, 12, 0, 0);
        let now = utc(2025, 6, 8, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "1w ago");
    }

    #[test]
    fn weeks_ago_3() {
        let dt = utc(2025, 6, 1, 12, 0, 0);
        let now = utc(2025, 6, 22, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "3w ago");
    }

    #[test]
    fn same_year_absolute_date() {
        let dt = utc(2025, 3, 15, 10, 0, 0);
        let now = utc(2025, 6, 1, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "Mar 15");
    }

    #[test]
    fn different_year_absolute_date() {
        let dt = utc(2024, 3, 15, 10, 0, 0);
        let now = utc(2025, 6, 1, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "Mar 15, 2024");
    }

    #[test]
    fn boundary_at_30_days() {
        let dt = utc(2025, 5, 2, 12, 0, 0);
        let now = utc(2025, 6, 1, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "May 02");
    }

    #[test]
    fn future_timestamp_returns_just_now() {
        let dt = utc(2025, 6, 1, 13, 0, 0);
        let now = utc(2025, 6, 1, 12, 0, 0);
        assert_eq!(format_relative_time(dt, now), "Just now");
    }

    // --- format_pages tests ---

    #[test]
    fn pages_singular() {
        assert_eq!(format_pages(1), "1 page");
    }

    #[test]
    fn pages_plural() {
        assert_eq!(format_pages(350), "350 pages");
    }

    // --- format_rating tests ---

    #[test]
    fn rating_display_whole() {
        assert_eq!(format_rating(4.0), "4/5");
    }

    #[test]
    fn rating_display_half() {
        assert_eq!(format_rating(3.5), "3.5/5");
    }

    #[test]
    fn rating_display_half_low() {
        assert_eq!(format_rating(0.5), "0.5/5");
    }

    #[test]
    fn valid_rating_accepts_half_stars() {
        for i in 1..=10 {
            let r = i as f64 * 0.5;
            assert!(is_valid_rating(r), "{r} should be valid");
        }
    }

    #[test]
    fn valid_rating_rejects_invalid() {
        assert!(!is_valid_rating(0.0));
        assert!(!is_valid_rating(0.3));
        assert!(!is_valid_rating(5.5));
        assert!(!is_valid_rating(-1.0));
        assert!(!is_valid_rating(1.7));
    }
}
