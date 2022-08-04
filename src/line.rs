use std::path::Path;

use proc_macro2::Span;
use regex::Regex;

pub struct LineAccess {
    file: Option<Vec<u8>>,
    span_pat: Regex,
}

impl LineAccess {
    pub fn new<P: AsRef<Path>>(path: P) -> LineAccess {
        LineAccess {
            file: std::fs::read(path).ok(),
            // example: #39 bytes(890..902)
            span_pat: Regex::new(r#"^#\d+ bytes\((\d+)\.\.(\d+)\)"#).unwrap(),
        }
    }

    pub fn span(&self, span: Span) -> Option<(usize, usize)> {
        let file = self.file.as_ref()?;
        let span_debug = format!("{:?}", span);
        let captures = self.span_pat.captures(&span_debug)?;
        let start_bytes: usize = captures.get(1)?.as_str().parse().ok()?;
        let end_bytes: usize = captures.get(2)?.as_str().parse().ok()?;

        let mut line_start = 1;
        for &b in file[..start_bytes].iter() {
            if b == b'\n' {
                line_start += 1;
            }
        }
        let mut line_end = line_start;
        for &b in file[start_bytes..end_bytes].iter() {
            if b == b'\n' {
                line_end += 1;
            }
        }
        Some((line_start, line_end))
    }
}
