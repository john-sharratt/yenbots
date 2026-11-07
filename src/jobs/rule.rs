//! A repeat rule (§9.2), read in Yen's time zone:
//!
//! | Rule | Runs |
//! |---|---|
//! | `hourly` | on the hour |
//! | `daily 07:30` | every day at 07:30 |
//! | `weekly sun 19:00`, `weekly mon,thu 09:00` | on those days |
//! | `monthly 1st 08:00` | on that day of every month; on the last, in a month too short for it |
//! | a cron line, `*/15 7-21 * * *` | as cron reads it ([`Cron`]) |
//!
//! A time a clock change skips — 02:30 the night the clocks go forward — is
//! run at the same distance past the change, 03:30; a time the clocks pass
//! twice is run the first time.

use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use jiff::Timestamp;

use super::cron::Cron;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    Hourly,
    Daily(u32, u32),
    /// The days, Sunday 0, and the time.
    Weekly(Vec<u32>, u32, u32),
    /// The day of the month, and the time.
    Monthly(u32, u32, u32),
    Cron(Cron),
}

/// How far ahead the next run is looked for: thirty years of days, which
/// finds the rarest a cron line can ask for — a 29th of February — and
/// refuses one that never comes.
const LOOK: u32 = 366 * 30;

const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

const FORMS: &str =
    "`hourly`, `daily 07:30`, `weekly sun 19:00`, `monthly 1st 08:00`, or a cron line";

/// `HH:MM`, in the 24-hour clock.
fn clock(text: &str) -> Result<(u32, u32), String> {
    let bad = || format!("`{text}` is not a time of day: `HH:MM`, as `07:30` or `19:00`");
    let (h, m) = text.split_once(':').ok_or_else(bad)?;
    let h: u32 = h.parse().ok().filter(|h| *h < 24).ok_or_else(bad)?;
    let m: u32 = (m.len() == 2)
        .then(|| m.parse().ok())
        .flatten()
        .filter(|m| *m < 60)
        .ok_or_else(bad)?;
    Ok((h, m))
}

/// `1st`, `2nd`, `3rd`, `4th` … `31st`.
fn nth(text: &str) -> Result<u32, String> {
    let bad = || format!("`{text}` is not a day of the month: `1st` to `31st`");
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    let n: u32 = digits.parse().map_err(|_| bad())?;
    let suffix = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    if !(1..=31).contains(&n) || text != format!("{n}{suffix}") {
        return Err(bad());
    }
    Ok(n)
}

fn weekday_number(w: Weekday) -> u32 {
    w.to_sunday_zero_offset() as u32
}

impl Rule {
    pub fn parse(text: &str) -> Result<Rule, String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let rule = match words[..] {
            ["hourly"] => Rule::Hourly,
            ["daily", at] => {
                let (h, m) = clock(at)?;
                Rule::Daily(h, m)
            }
            ["weekly", days, at] => {
                let (h, m) = clock(at)?;
                let mut on = Vec::new();
                for d in days.split(',') {
                    let n = DAYS
                        .iter()
                        .position(|x| x.eq_ignore_ascii_case(d))
                        .ok_or_else(|| format!("`{d}` is not a day: `sun`, `mon`, `tue`, `wed`, `thu`, `fri` or `sat`"))?;
                    if !on.contains(&(n as u32)) {
                        on.push(n as u32);
                    }
                }
                on.sort_unstable();
                Rule::Weekly(on, h, m)
            }
            ["monthly", day, at] => {
                let (h, m) = clock(at)?;
                Rule::Monthly(nth(day)?, h, m)
            }
            [_, _, _, _, _] => Rule::Cron(Cron::parse(text)?),
            _ => return Err(format!("`{text}` is not a repeat rule: {FORMS}")),
        };
        if rule.next(0, &TimeZone::UTC).is_none() {
            return Err(format!("`{text}` never comes"));
        }
        Ok(rule)
    }

    /// Whether it runs on `d`.
    fn on(&self, d: Date) -> bool {
        match self {
            Rule::Hourly | Rule::Daily(..) => true,
            Rule::Weekly(days, ..) => days.contains(&weekday_number(d.weekday())),
            Rule::Monthly(day, ..) => d.day() as u32 == (*day).min(d.days_in_month() as u32),
            Rule::Cron(c) => c.on_day(
                d.month() as u32,
                d.day() as u32,
                weekday_number(d.weekday()),
            ),
        }
    }

    /// The times of a day it runs, earliest first.
    fn times(&self) -> Vec<(u32, u32)> {
        match self {
            Rule::Hourly => (0..24).map(|h| (h, 0)).collect(),
            Rule::Daily(h, m) | Rule::Weekly(_, h, m) | Rule::Monthly(_, h, m) => vec![(*h, *m)],
            Rule::Cron(c) => c.times(),
        }
    }

    /// The first time after `after_ms` it runs, in the zone `tz`; `None` if
    /// none comes in thirty years.
    pub fn next(&self, after_ms: i64, tz: &TimeZone) -> Option<i64> {
        let mut day = Timestamp::from_millisecond(after_ms)
            .ok()?
            .to_zoned(tz.clone())
            .date();
        let times = self.times();
        for _ in 0..LOOK {
            if self.on(day) {
                for &(h, m) in &times {
                    let local = day.at(h as i8, m as i8, 0, 0);
                    let at = tz.to_ambiguous_zoned(local).compatible().ok()?;
                    let ms = at.timestamp().as_millisecond();
                    if ms > after_ms {
                        return Some(ms);
                    }
                }
            }
            day = day.tomorrow().ok()?;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    fn sydney() -> TimeZone {
        TimeZone::get("Australia/Sydney").unwrap()
    }

    /// The instant of a Sydney wall-clock time, in milliseconds.
    fn at(y: i16, mo: i8, d: i8, h: i8, mi: i8) -> i64 {
        sydney()
            .to_ambiguous_zoned(date(y, mo, d).at(h, mi, 0, 0))
            .compatible()
            .unwrap()
            .timestamp()
            .as_millisecond()
    }

    fn next(rule: &str, after: i64) -> i64 {
        Rule::parse(rule).unwrap().next(after, &sydney()).unwrap()
    }

    /// **Each rule runs next when Yen's clock says**, strictly after the
    /// time asked from.
    #[test]
    fn each_rule_runs_next_at_its_time() {
        // Saturday 7 November 2026, 10:15 in Sydney.
        let now = at(2026, 11, 7, 10, 15);
        assert_eq!(next("hourly", now), at(2026, 11, 7, 11, 0));
        assert_eq!(next("daily 07:30", now), at(2026, 11, 8, 7, 30));
        assert_eq!(next("daily 10:15", now), at(2026, 11, 8, 10, 15), "not now");
        assert_eq!(next("daily 10:16", now), at(2026, 11, 7, 10, 16));
        assert_eq!(next("weekly sun 19:00", now), at(2026, 11, 8, 19, 0));
        assert_eq!(next("weekly mon,thu 09:00", now), at(2026, 11, 9, 9, 0));
        assert_eq!(next("monthly 1st 08:00", now), at(2026, 12, 1, 8, 0));
        assert_eq!(next("*/20 10 * * *", now), at(2026, 11, 7, 10, 20));
        assert_eq!(next("0 9 * * sat", now), at(2026, 11, 14, 9, 0));
    }

    /// **A month too short for the day runs on its last.**
    #[test]
    fn a_short_month_runs_on_its_last_day() {
        let jan31 = at(2027, 1, 31, 9, 0);
        assert_eq!(next("monthly 31st 08:00", jan31), at(2027, 2, 28, 8, 0));
        assert_eq!(
            next("monthly 31st 08:00", at(2027, 2, 28, 8, 0)),
            at(2027, 3, 31, 8, 0)
        );
    }

    /// **A time the clocks skip runs just after; one they pass twice runs
    /// once.** Sydney's clocks go forward at 02:00 on 4 October 2026 and
    /// back at 03:00 on 4 April 2027.
    #[test]
    fn clock_changes_are_run_through() {
        let skipped = next("daily 02:30", at(2026, 10, 3, 12, 0));
        assert_eq!(
            skipped - at(2026, 10, 4, 1, 59),
            31 * 60_000,
            "03:30, the new time"
        );
        let twice = next("daily 02:30", at(2027, 4, 3, 12, 0));
        let after = next("daily 02:30", twice);
        assert_eq!(after - twice, 25 * 3_600_000, "the next day's, once");
    }

    /// **The rarest cron line is found; one that never comes is refused.**
    #[test]
    fn a_rule_that_never_comes_is_refused() {
        let rare = Rule::parse("0 0 29 feb *").unwrap();
        assert!(rare.next(0, &TimeZone::UTC).is_some());
        assert_eq!(
            Rule::parse("0 0 31 2 *").unwrap_err(),
            "`0 0 31 2 *` never comes"
        );
    }

    #[test]
    fn a_rule_that_is_not_one_is_refused_with_why() {
        let told = |s: &str| Rule::parse(s).unwrap_err();
        assert_eq!(
            told("every morning"),
            "`every morning` is not a repeat rule: `hourly`, `daily 07:30`, `weekly sun 19:00`, `monthly 1st 08:00`, or a cron line"
        );
        assert_eq!(
            told("daily 7.30"),
            "`7.30` is not a time of day: `HH:MM`, as `07:30` or `19:00`"
        );
        assert_eq!(
            told("daily 24:00"),
            "`24:00` is not a time of day: `HH:MM`, as `07:30` or `19:00`"
        );
        assert_eq!(
            told("weekly sunday 19:00"),
            "`sunday` is not a day: `sun`, `mon`, `tue`, `wed`, `thu`, `fri` or `sat`"
        );
        assert_eq!(
            told("monthly 1th 08:00"),
            "`1th` is not a day of the month: `1st` to `31st`"
        );
        assert_eq!(
            told("monthly 32nd 08:00"),
            "`32nd` is not a day of the month: `1st` to `31st`"
        );
        assert_eq!(
            Rule::parse("monthly 22nd 08:00").unwrap(),
            Rule::Monthly(22, 8, 0)
        );
        assert_eq!(
            Rule::parse("monthly 11th 08:00").unwrap(),
            Rule::Monthly(11, 8, 0)
        );
        assert_eq!(Rule::parse("daily 7:05").unwrap(), Rule::Daily(7, 5));
    }
}
