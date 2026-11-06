//! Reading source the way the rules need it, without a full parser: comments
//! blanked so a rule is not misled by one, the bodies of tagged templates
//! (`html\`…\``, `css\`…\``) with their `${…}` parts found, CSS split into its
//! declarations, and a byte offset turned into the line a refusal names.
//! Every function keeps byte offsets: what is blanked becomes spaces, so a
//! line counted in the result is the line in the file.

/// The line, from 1, of byte `offset` in `text`.
pub fn line_of(text: &str, offset: usize) -> usize {
    1 + text.as_bytes()[..offset.min(text.len())]
        .iter()
        .filter(|b| **b == b'\n')
        .count()
}

/// Where the reader is in JavaScript.
#[derive(Clone, Copy)]
enum Mode {
    Code,
    /// Inside `${…}` of a template, with this many braces open.
    Expr(u32),
    Template,
    Quoted(u8),
}

/// Spaces for every byte of `bytes[from..to]` but its newlines.
fn blank(out: &mut [u8], from: usize, to: usize) {
    for b in &mut out[from..to] {
        if *b != b'\n' {
            *b = b' ';
        }
    }
}

/// `js` with its comments blanked. Strings and templates are read past, so
/// a `//` inside one is not a comment.
pub fn without_comments(js: &str) -> String {
    let src = js.as_bytes();
    let mut out = src.to_vec();
    let mut stack = vec![Mode::Code];
    let mut i = 0;
    while i < src.len() {
        let b = src[i];
        let next = src.get(i + 1).copied();
        match *stack.last().unwrap_or(&Mode::Code) {
            Mode::Code | Mode::Expr(_) => {
                if b == b'/' && next == Some(b'/') {
                    let end = src[i..]
                        .iter()
                        .position(|c| *c == b'\n')
                        .map_or(src.len(), |n| i + n);
                    blank(&mut out, i, end);
                    i = end;
                    continue;
                }
                if b == b'/' && next == Some(b'*') {
                    let end = src[i + 2..]
                        .windows(2)
                        .position(|w| w == b"*/")
                        .map_or(src.len(), |n| i + 2 + n + 2);
                    blank(&mut out, i, end);
                    i = end;
                    continue;
                }
                match (b, stack.last_mut()) {
                    (b'"' | b'\'', _) => stack.push(Mode::Quoted(b)),
                    (b'`', _) => stack.push(Mode::Template),
                    (b'{', Some(Mode::Expr(depth))) => *depth += 1,
                    (b'}', Some(Mode::Expr(0))) => {
                        stack.pop();
                    }
                    (b'}', Some(Mode::Expr(depth))) => *depth -= 1,
                    _ => {}
                }
            }
            Mode::Quoted(q) => {
                if b == b'\\' {
                    i += 2;
                    continue;
                }
                if b == q || b == b'\n' {
                    stack.pop();
                }
            }
            Mode::Template => {
                if b == b'\\' {
                    i += 2;
                    continue;
                }
                if b == b'`' {
                    stack.pop();
                } else if b == b'$' && next == Some(b'{') {
                    stack.push(Mode::Expr(0));
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| js.to_string())
}

/// A tagged template's body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// The byte offset of the body's first byte in the file.
    pub start: usize,
    /// The body as written, `${…}` included.
    pub raw: String,
    /// The body with every `${…}` blanked: only the template's own text.
    pub text: String,
}

/// Every template tagged `tag` in `js` (comments already blanked):
/// `html\`…\`` for `"html"`. A template inside another's `${…}` is found as
/// well, and blanked from the outer one's text.
pub fn tagged(js: &str, tag: &str) -> Vec<Template> {
    let src = js.as_bytes();
    let marker = format!("{tag}`");
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = js[from..].find(&marker).map(|n| from + n) {
        from = at + marker.len();
        let before = at.checked_sub(1).map(|p| src[p]);
        if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c == b'.')
        {
            continue;
        }
        let start = at + marker.len();
        let (end, text) = body(src, start);
        found.push(Template {
            start,
            raw: js[start..end].to_string(),
            text,
        });
    }
    found
}

/// The end of the template body starting at `start`, and its text with each
/// `${…}` blanked.
fn body(src: &[u8], start: usize) -> (usize, String) {
    let mut text = Vec::new();
    let mut i = start;
    // Inside `${…}`: what is open there — quotes, nested templates, braces.
    let mut expr: Option<Vec<u8>> = None;
    while i < src.len() {
        let b = src[i];
        let next = src.get(i + 1).copied();
        let Some(open) = expr.as_mut() else {
            if b == b'\\' {
                text.extend_from_slice(&src[i..(i + 2).min(src.len())]);
                i += 2;
                continue;
            }
            if b == b'`' {
                break;
            }
            if b == b'$' && next == Some(b'{') {
                expr = Some(Vec::new());
                text.extend_from_slice(b"  ");
                i += 2;
                continue;
            }
            text.push(b);
            i += 1;
            continue;
        };
        let mut step = 1;
        let mut closed = false;
        match open.last().copied() {
            Some(q @ (b'"' | b'\'')) => {
                if b == b'\\' {
                    step = 2;
                } else if b == q {
                    open.pop();
                }
            }
            Some(b'`') => {
                if b == b'\\' {
                    step = 2;
                } else if b == b'`' {
                    open.pop();
                } else if b == b'$' && next == Some(b'{') {
                    open.push(b'{');
                    step = 2;
                }
            }
            top => match b {
                b'"' | b'\'' | b'`' | b'{' => open.push(b),
                b'}' if top.is_some() => {
                    open.pop();
                }
                b'}' => closed = true,
                _ => {}
            },
        }
        for &c in &src[i..(i + step).min(src.len())] {
            text.push(if c == b'\n' { b'\n' } else { b' ' });
        }
        if closed {
            expr = None;
        }
        i += step;
    }
    (i, String::from_utf8_lossy(&text).to_string())
}

/// `css` with its `/* … */` comments blanked.
pub fn css_without_comments(css: &str) -> String {
    let src = css.as_bytes();
    let mut out = src.to_vec();
    let mut i = 0;
    while let Some(n) = css[i..].find("/*") {
        let at = i + n;
        let end = css[at + 2..]
            .find("*/")
            .map_or(css.len(), |e| at + 2 + e + 2);
        blank(&mut out, at, end);
        i = end;
    }
    String::from_utf8(out).unwrap_or_else(|_| css.to_string())
}

/// A CSS declaration: its property, lowercased, and its value with the
/// value's byte offset in the text it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub property: String,
    pub value: String,
    pub at: usize,
}

/// Every declaration in `css` (comments already blanked): the text between
/// `{`, `;` and `}` that holds `property: value`. A selector — text ending at
/// a `{` — is not one, so `a:hover` is never read as a property.
pub fn declarations(css: &str) -> Vec<Declaration> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, b) in css.bytes().enumerate() {
        if !matches!(b, b'{' | b'}' | b';') {
            continue;
        }
        let piece = &css[start..i];
        if b != b'{' {
            if let Some((prop, value)) = piece.split_once(':') {
                let property = prop.trim().to_ascii_lowercase();
                let is_name = !property.is_empty()
                    && property
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_');
                if is_name {
                    let lead = value.len() - value.trim_start().len();
                    out.push(Declaration {
                        property,
                        value: value.trim().to_string(),
                        at: start + prop.len() + 1 + lead,
                    });
                }
            }
        }
        start = i + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_count_from_one() {
        assert_eq!(
            [0, 3, 4, 9].map(|o| line_of("one\ntwo\nthree", o)),
            [1, 1, 2, 3]
        );
    }

    /// **Comments are blanked byte for byte**; a `//` in a string or a
    /// template is not one.
    #[test]
    fn comments_are_blanked_and_strings_are_not() {
        let js = "a(\"http://x\"); // c #fff\n/* b\n c */ t`//${ x /* y */ }` + 'q//'";
        assert_eq!(
            without_comments(js),
            "a(\"http://x\");          \n    \n      t`//${ x         }` + 'q//'"
        );
        assert_eq!(without_comments(js).len(), js.len());
    }

    /// **A template's body is found with its `${…}` blanked**, a nested one
    /// is found too, and a tag that is only part of a name is not one.
    #[test]
    fn tagged_templates_are_found_with_their_expressions_blanked() {
        let js = "return html`<ul>${xs.map((x) => html`<li alt=\"${x}\">${x}</li>`)}</ul>` + myhtml`no` + o.html`no`;";
        let found = tagged(js, "html");
        assert_eq!(found.len(), 2);
        assert_eq!(
            found[0].raw,
            "<ul>${xs.map((x) => html`<li alt=\"${x}\">${x}</li>`)}</ul>"
        );
        assert_eq!(
            found[0].text,
            format!("<ul>{}</ul>", " ".repeat(found[0].raw.len() - 9))
        );
        assert_eq!(found[0].start, 12);
        assert_eq!(found[1].text, "<li alt=\"    \">    </li>");
        let css = tagged("static styles = css`\n  a { color: red; }\n`;", "css");
        assert_eq!(css[0].text, "\n  a { color: red; }\n");
        assert_eq!(
            line_of(
                "static styles = css`\n  a { color: red; }\n`;",
                css[0].start + 3
            ),
            2
        );
    }

    /// **A declaration is `property: value` between braces and semicolons**;
    /// a selector with a colon is not one.
    #[test]
    fn declarations_are_read_and_selectors_are_not() {
        let css = "a:hover { color: #fff; margin:0 }\n@media (x: y) { --t: 1px }";
        let d = declarations(css);
        assert_eq!(
            d.iter()
                .map(|d| (d.property.as_str(), d.value.as_str()))
                .collect::<Vec<_>>(),
            [("color", "#fff"), ("margin", "0"), ("--t", "1px")]
        );
        assert_eq!(&css[d[0].at..d[0].at + 4], "#fff");
        assert_eq!(&css[d[2].at..d[2].at + 3], "1px");
        assert_eq!(
            css_without_comments("a { /* #fff */ color: x }"),
            "a {            color: x }"
        );
    }
}
