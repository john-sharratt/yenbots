//! CSV as a spreadsheet reads and writes it (RFC 4180): a cell holding a
//! comma, a quote or a line break is quoted, a quote inside doubled; lines
//! end `\r\n`. A file written starts with the byte-order mark, so a
//! spreadsheet reads its accents as UTF-8; one read may start with it.

/// The byte-order mark a written file starts with.
pub const BOM: char = '\u{feff}';

/// Append `cells` to `out` as one line.
pub fn write_row(out: &mut String, cells: &[String]) {
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        if cell.contains([',', '"', '\r', '\n']) {
            out.push('"');
            out.push_str(&cell.replace('"', "\"\""));
            out.push('"');
        } else {
            out.push_str(cell);
        }
    }
    out.push_str("\r\n");
}

/// The rows of `text`; why not, when a quoted cell is never closed. A blank
/// line is no row.
pub fn parse(text: &str) -> Result<Vec<Vec<String>>, String> {
    let text = text.strip_prefix(BOM).unwrap_or(text);
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    let (mut line, mut opened) = (1, 1);
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    cell.push('"');
                }
                '"' => quoted = false,
                '\n' => {
                    line += 1;
                    cell.push(c);
                }
                _ => cell.push(c),
            }
            continue;
        }
        match c {
            '"' if cell.is_empty() => {
                quoted = true;
                opened = line;
            }
            ',' => row.push(std::mem::take(&mut cell)),
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' => {
                line += 1;
                row.push(std::mem::take(&mut cell));
                if !(row.len() == 1 && row[0].is_empty()) {
                    rows.push(std::mem::take(&mut row));
                }
                row.clear();
            }
            _ => cell.push(c),
        }
    }
    if quoted {
        return Err(format!(
            "a quoted cell opened on line {opened} is never closed"
        ));
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// **A row is written with only the cells that need it quoted**, as
    /// these bytes.
    #[test]
    fn rows_are_written_as_a_spreadsheet_reads_them() {
        let mut out = String::new();
        write_row(&mut out, &cells(&["title", "notes"]));
        write_row(&mut out, &cells(&["Phở bò", "say \"hi\", then\nstir"]));
        assert_eq!(
            out.as_bytes(),
            "title,notes\r\nPhở bò,\"say \"\"hi\"\", then\nstir\"\r\n".as_bytes()
        );
    }

    /// **What is written is read back the same**, a mark, blank lines and a
    /// last line without its ending aside.
    #[test]
    fn rows_are_read_back() {
        let mut out = String::from(BOM);
        write_row(&mut out, &cells(&["a", "b,c"]));
        write_row(&mut out, &cells(&["", "x\"y\nz"]));
        assert_eq!(
            parse(&out).unwrap(),
            [cells(&["a", "b,c"]), cells(&["", "x\"y\nz"])]
        );
        assert_eq!(
            parse("a,b\n\nc,d").unwrap(),
            [cells(&["a", "b"]), cells(&["c", "d"])]
        );
        assert_eq!(
            parse("a,\"b\nc").unwrap_err(),
            "a quoted cell opened on line 1 is never closed"
        );
    }
}
