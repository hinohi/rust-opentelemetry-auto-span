use std::path::Path;

use proc_macro2::Span;
use regex::Regex;

pub struct LineAccess {
    lines: Vec<Line>,
    span_pat: Regex,
}

#[derive(Debug, Eq, PartialEq)]
struct Line {
    start_bytes: usize,
    end_bytes: usize,
    line_number: usize,
    data: String,
}

impl LineAccess {
    pub fn new<P: AsRef<Path>>(path: P) -> LineAccess {
        let buf = std::fs::read(path).unwrap_or(Vec::new());
        Self::from_bytes(&buf)
    }

    pub fn from_bytes(buf: &[u8]) -> LineAccess {
        let mut lines = Vec::<Line>::new();
        for (i, &b) in buf.iter().enumerate() {
            if b == b'\n' {
                let start_bytes = lines
                    .last()
                    .and_then(|l| Some(l.end_bytes + 1))
                    .unwrap_or(0);
                let data = buf[start_bytes..i]
                    .iter()
                    .copied()
                    .skip_while(|&b| b.is_ascii_whitespace())
                    .collect::<Vec<_>>();
                let line = Line {
                    start_bytes,
                    end_bytes: i,
                    line_number: lines
                        .last()
                        .and_then(|l| Some(l.line_number + 1))
                        .unwrap_or(1),
                    data: String::from_utf8_lossy(&data).to_string(),
                };
                lines.push(line);
            }
        }
        LineAccess {
            lines,
            // example: #39 bytes(890..902)
            span_pat: Regex::new(r#"^#\d+ bytes\((\d+)\.\.\d+\)"#).unwrap(),
        }
    }

    pub fn span(&self, span: Span) -> Option<(usize, String)> {
        if self.lines.is_empty() {
            return None;
        }
        let span_debug = format!("{:?}", span);
        let captures = self.span_pat.captures(&span_debug)?;
        let start_bytes: usize = captures.get(1)?.as_str().parse().ok()?;
        self.find_line(start_bytes)
            .and_then(|line| Some((line.line_number, line.data.clone())))
    }

    fn find_line(&self, bytes: usize) -> Option<&Line> {
        let line = self.lines.binary_search_by(|line| {
            if bytes < line.start_bytes {
                std::cmp::Ordering::Greater
            } else if line.end_bytes < bytes {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        });
        match line {
            Ok(i) => self.lines.get(i),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line() {
        let code = r#"a
bcd {
  123;
  5678;
}
"#;
        let line1 = Line {
            start_bytes: 0,
            end_bytes: 1,
            line_number: 1,
            data: String::from("a"),
        };
        let line2 = Line {
            start_bytes: 2,
            end_bytes: 7,
            line_number: 2,
            data: String::from("bcd {"),
        };
        let line3 = Line {
            start_bytes: 8,
            end_bytes: 14,
            line_number: 3,
            data: String::from("123;"),
        };
        let la = LineAccess::from_bytes(code.as_bytes());
        println!("{:?}", la.lines);
        assert_eq!(la.find_line(0), Some(&line1));
        assert_eq!(la.find_line(3), Some(&line2));
        assert_eq!(la.find_line(11), Some(&line3));
        assert_eq!(la.find_line(100), None);
    }
}
