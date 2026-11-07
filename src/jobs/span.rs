//! A length of time as a manifest writes one: a whole number and its unit —
//! `30s`, `10m`, `2h`, `1d` — for a schedule's `before:` and an app's
//! `deadline:` (§9.2, §10.3).

/// The span `text` names, in milliseconds.
pub fn parse(text: &str) -> Result<i64, String> {
    let t = text.trim();
    let unit = t.chars().last().unwrap_or(' ');
    let ms = match unit {
        's' => 1_000,
        'm' => 60_000,
        'h' => 3_600_000,
        'd' => 86_400_000,
        _ => {
            return Err(format!(
                "`{text}` is not a span: a number and `s`, `m`, `h` or `d`, as `10m`"
            ))
        }
    };
    let n: i64 = t[..t.len() - 1]
        .parse()
        .ok()
        .filter(|n| *n >= 0 && *n <= 366 * 86_400)
        .ok_or_else(|| {
            format!("`{text}` is not a span: a number and `s`, `m`, `h` or `d`, as `10m`")
        })?;
    Ok(n * ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_span_is_a_number_and_its_unit() {
        assert_eq!(parse("30s"), Ok(30_000));
        assert_eq!(parse("10m"), Ok(600_000));
        assert_eq!(parse("2h"), Ok(7_200_000));
        assert_eq!(parse("1d"), Ok(86_400_000));
        assert_eq!(parse(" 0m "), Ok(0));
        for bad in ["10", "m", "-1m", "1.5h", "10 minutes", "", "1w"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }
}
