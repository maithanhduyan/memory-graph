//! Time and timestamp utilities

use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// Get current Unix timestamp in seconds
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Get current user from git config or OS environment
pub fn get_current_user() -> String {
    use std::env;
    use std::process::Command;

    // 1. Try Git Config (preferred for project context)
    if let Ok(output) = Command::new("git").args(["config", "user.name"]).output() {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }

    // 2. Try OS Environment Variable
    env::var("USER") // Linux/Mac
        .or_else(|_| env::var("USERNAME")) // Windows
        .unwrap_or_else(|_| "anonymous".to_string())
}

/// Get current time information as JSON
pub fn get_current_time() -> Value {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap();
    let timestamp = duration.as_secs();
    let millis = duration.as_millis() as u64;

    // Calculate datetime components
    let secs = timestamp as i64;

    // Days since epoch
    let days = secs / 86400;
    let remaining = secs % 86400;

    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Calculate year, month, day
    let (year, month, day) = days_to_ymd(days);

    // Format ISO 8601
    let iso8601 = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    );

    // Format readable
    let weekday = get_weekday(days);
    let month_name = get_month_name(month);
    let readable = format!(
        "{}, {} {} {} {:02}:{:02}:{:02} UTC",
        weekday, day, month_name, year, hours, minutes, seconds
    );

    json!({
        "timestamp": timestamp,
        "timestamp_ms": millis,
        "iso8601": iso8601,
        "readable": readable,
        "components": {
            "year": year,
            "month": month,
            "day": day,
            "hour": hours,
            "minute": minutes,
            "second": seconds,
            "weekday": weekday
        }
    })
}

/// Convert days since epoch to year/month/day
pub fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm to convert days since epoch to year/month/day
    let remaining_days = days + 719468; // Days from year 0 to 1970

    let era = remaining_days / 146097;
    let doe = remaining_days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { year + 1 } else { year };

    (year, month, day)
}

/// Get weekday name from days since epoch
pub fn get_weekday(days: i64) -> &'static str {
    match (days + 4) % 7 {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}

/// Get month name from month number
pub fn get_month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}
