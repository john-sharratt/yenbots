//! A cron line, for a repeat no named rule says (§9.2): five fields —
//! minute, hour, day of the month, month, day of the week — each `*`, a
//! number, a range `a-b`, a step `*/n` or `a-b/n`, or a list of those joined
//! by commas. Months and days of the week may be named (`jan`, `mon`); Sunday
//! is 0 or 7. As in every cron, when both day fields are given a day matching
//! either one is a day it runs.

/// A field's values, as a set of bits.
type Bits = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cron {
    minutes: Bits,
    hours: Bits,
    days: Bits,
    months: Bits,
    weekdays: Bits,
    /// Whether the day of the month was given — not `*`.
    days_given: bool,
    /// Whether the day of the week was given.
    weekdays_given: bool,
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const WEEKDAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/// One value of a field: a number, or a name from `names` counted from `lo`.
fn value(s: &str, lo: u32, names: &[&str]) -> Option<u32> {
    if let Ok(n) = s.parse() {
        return Some(n);
    }
    let at = names.iter().position(|n| n.eq_ignore_ascii_case(s))?;
    Some(lo + at as u32)
}

/// The field `text`, its values from `lo` to `hi`.
fn field(text: &str, lo: u32, hi: u32, names: &[&str], what: &str) -> Result<Bits, String> {
    let bad = || {
        format!("`{text}` is not a cron {what}: from {lo} to {hi}, as `*`, `5`, `1-5`, `*/15` or a list")
    };
    let mut bits: Bits = 0;
    for part in text.split(',') {
        let (range, step) = match part.split_once('/') {
            Some((r, s)) => (r, s.parse::<u32>().ok().filter(|s| *s > 0).ok_or_else(bad)?),
            None => (part, 1),
        };
        let (from, to) = if range == "*" {
            (lo, hi)
        } else if let Some((a, b)) = range.split_once('-') {
            (
                value(a, lo, names).ok_or_else(bad)?,
                value(b, lo, names).ok_or_else(bad)?,
            )
        } else {
            let v = value(range, lo, names).ok_or_else(bad)?;
            // `5/15` runs from 5 to the end, every 15.
            (v, if part.contains('/') { hi } else { v })
        };
        if from < lo || to > hi || from > to {
            return Err(bad());
        }
        let mut v = from;
        while v <= to {
            bits |= 1 << v;
            v += step;
        }
    }
    Ok(bits)
}

fn has(bits: Bits, v: u32) -> bool {
    bits & (1 << v) != 0
}

impl Cron {
    pub fn parse(line: &str) -> Result<Cron, String> {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [minute, hour, day, month, weekday] = fields[..] else {
            return Err(format!(
                "`{line}` is not a cron line: five fields, minute hour day month weekday"
            ));
        };
        let mut weekdays = field(weekday, 0, 7, &WEEKDAYS, "day of the week")?;
        if has(weekdays, 7) {
            weekdays = (weekdays | 1) & !(1 << 7);
        }
        Ok(Cron {
            minutes: field(minute, 0, 59, &[], "minute")?,
            hours: field(hour, 0, 23, &[], "hour")?,
            days: field(day, 1, 31, &[], "day of the month")?,
            months: field(month, 1, 12, &MONTHS, "month")?,
            weekdays,
            days_given: !day.starts_with('*'),
            weekdays_given: !weekday.starts_with('*'),
        })
    }

    /// Whether it runs on the day `day` of month `month` (1–12), a
    /// `weekday` (0 Sunday – 6 Saturday).
    pub fn on_day(&self, month: u32, day: u32, weekday: u32) -> bool {
        if !has(self.months, month) {
            return false;
        }
        let by_day = has(self.days, day);
        let by_weekday = has(self.weekdays, weekday);
        match (self.days_given, self.weekdays_given) {
            (true, true) => by_day || by_weekday,
            (true, false) => by_day,
            (false, true) => by_weekday,
            (false, false) => true,
        }
    }

    /// The times of a day it runs at, as `(hour, minute)`, earliest first.
    pub fn times(&self) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for h in 0..24 {
            if has(self.hours, h) {
                for m in 0..60 {
                    if has(self.minutes, m) {
                        out.push((h, m));
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cron_line_reads_its_five_fields() {
        let c = Cron::parse("*/20 7-9 * * mon-fri").unwrap();
        assert_eq!(
            c.times(),
            [
                (7, 0),
                (7, 20),
                (7, 40),
                (8, 0),
                (8, 20),
                (8, 40),
                (9, 0),
                (9, 20),
                (9, 40)
            ]
        );
        assert!(c.on_day(11, 9, 1), "Monday the 9th");
        assert!(!c.on_day(11, 8, 0), "Sunday the 8th");
        let c = Cron::parse("30 18 1,15 jan,JUL *").unwrap();
        assert_eq!(c.times(), [(18, 30)]);
        assert!(c.on_day(1, 15, 3));
        assert!(!c.on_day(2, 15, 3));
        assert!(!c.on_day(7, 2, 3));
        let c = Cron::parse("5/30 0 * * *").unwrap();
        assert_eq!(c.times(), [(0, 5), (0, 35)]);
    }

    /// **Sunday is 0 or 7, and two day fields match either**, as in every
    /// cron.
    #[test]
    fn the_day_fields_are_cron_s() {
        let sunday = Cron::parse("0 9 * * 7").unwrap();
        assert!(sunday.on_day(11, 8, 0));
        let either = Cron::parse("0 9 12 * fri").unwrap();
        assert!(either.on_day(11, 12, 4), "Thursday the 12th");
        assert!(either.on_day(11, 20, 5), "Friday the 20th");
        assert!(!either.on_day(11, 19, 4), "Thursday the 19th");
    }

    #[test]
    fn a_line_that_is_not_cron_is_refused_with_why() {
        assert_eq!(
            Cron::parse("0 9 * *").unwrap_err(),
            "`0 9 * *` is not a cron line: five fields, minute hour day month weekday"
        );
        assert_eq!(
            Cron::parse("60 9 * * *").unwrap_err(),
            "`60` is not a cron minute: from 0 to 59, as `*`, `5`, `1-5`, `*/15` or a list"
        );
        for bad in [
            "0 24 * * *",
            "0 9 0 * *",
            "0 9 * 13 *",
            "0 9 * * 8",
            "*/0 9 * * *",
            "0 9-7 * * *",
            "0 9 * * funday",
        ] {
            assert!(Cron::parse(bad).is_err(), "{bad}");
        }
    }
}
