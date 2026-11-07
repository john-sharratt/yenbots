//! A time as Yen reads it, in her zone: `Sunday 8 November, 7:30 pm` —
//! what a workflow's `ctx.time(ms)` answers (§10.2).

use jiff::tz::TimeZone;
use jiff::Timestamp;

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// `ms` in the zone `tz`, as a person says it; `None` for an instant out of
/// the calendar's range.
pub fn spoken(ms: i64, tz: &TimeZone) -> Option<String> {
    let z = Timestamp::from_millisecond(ms).ok()?.to_zoned(tz.clone());
    let weekday = WEEKDAYS[z.weekday().to_sunday_zero_offset() as usize];
    let month = MONTHS[z.month() as usize - 1];
    let (h, m) = (z.hour(), z.minute());
    let half = if h < 12 { "am" } else { "pm" };
    let h12 = match h % 12 {
        0 => 12,
        h => h,
    };
    Some(format!(
        "{weekday} {} {month}, {h12}:{m:02} {half}",
        z.day()
    ))
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    #[test]
    fn a_time_is_said_in_yens_zone() {
        let tz = TimeZone::get("Australia/Sydney").unwrap();
        let ms = |h, m| {
            tz.to_ambiguous_zoned(date(2026, 11, 8).at(h, m, 0, 0))
                .compatible()
                .unwrap()
                .timestamp()
                .as_millisecond()
        };
        assert_eq!(
            spoken(ms(19, 30), &tz).unwrap(),
            "Sunday 8 November, 7:30 pm"
        );
        assert_eq!(
            spoken(ms(0, 5), &tz).unwrap(),
            "Sunday 8 November, 12:05 am"
        );
        assert_eq!(
            spoken(ms(12, 0), &tz).unwrap(),
            "Sunday 8 November, 12:00 pm"
        );
        assert_eq!(
            spoken(ms(9, 0), &TimeZone::UTC).unwrap(),
            "Saturday 7 November, 10:00 pm"
        );
        assert_eq!(spoken(i64::MAX, &tz), None);
    }
}
