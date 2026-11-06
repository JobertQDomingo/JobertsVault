//! CSV for Excel (DATA_MODEL §8): UTF-8 with a byte order mark, commas between fields, and a
//! CRLF after every line (RFC 4180).

use std::borrow::Cow;

/// Makes Excel read the file as UTF-8, so `₱` and accents show correctly.
const BOM: char = '\u{FEFF}';

/// A whole file: the byte order mark, `header`, then each row.
pub fn document(header: &[&str], rows: impl IntoIterator<Item = Vec<String>>) -> String {
    let mut out = String::from(BOM);
    push_line(&mut out, header.iter().copied());
    for row in rows {
        push_line(&mut out, row.iter().map(String::as_str));
    }
    out
}

fn push_line<'a>(out: &mut String, fields: impl Iterator<Item = &'a str>) {
    let fields: Vec<Cow<str>> = fields.map(field).collect();
    out.push_str(&fields.join(","));
    out.push_str("\r\n");
}

/// One field: in quotes, with its own quotes doubled, when it holds a comma, a quote or a line
/// break. Anything else is written as it is.
pub fn field(text: &str) -> Cow<'_, str> {
    if text.contains([',', '"', '\r', '\n']) {
        Cow::Owned(format!("\"{}\"", text.replace('"', "\"\"")))
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_written_as_it_is() {
        assert_eq!(field("Corner Cafe"), "Corner Cafe");
        assert_eq!(field(""), "");
        assert_eq!(field(" spaced "), " spaced ");
        assert_eq!(field("Caf\u{e9} \u{20B1}50"), "Caf\u{e9} \u{20B1}50");
    }

    #[test]
    fn commas_quotes_and_line_breaks_are_quoted() {
        assert_eq!(field("Smith, John"), "\"Smith, John\"");
        assert_eq!(field("12\" pizza"), "\"12\"\" pizza\"");
        assert_eq!(field("\"quoted\""), "\"\"\"quoted\"\"\"");
        assert_eq!(field("line 1\nline 2"), "\"line 1\nline 2\"");
        assert_eq!(field("line 1\r\nline 2"), "\"line 1\r\nline 2\"");
    }

    #[test]
    fn a_document_starts_with_the_bom_and_ends_every_line_with_crlf() {
        let text = document(
            &["a", "b"],
            [
                vec!["1".to_string(), "x, y".to_string()],
                vec!["2".to_string(), "two\nlines".to_string()],
            ],
        );
        assert!(text.as_bytes().starts_with(&[0xEF, 0xBB, 0xBF]));
        assert_eq!(text, "\u{FEFF}a,b\r\n1,\"x, y\"\r\n2,\"two\nlines\"\r\n");
    }

    #[test]
    fn no_rows_is_the_header_alone() {
        assert_eq!(document(&["a", "b"], []), "\u{FEFF}a,b\r\n");
    }
}
