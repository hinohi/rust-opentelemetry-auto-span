use syn::Path;

pub(crate) struct PathPat {
    segments: Vec<Vec<String>>,
}

pub(crate) fn path_match<P: Into<PathPat>>(path: &Path, pat: P) -> bool {
    let pat = pat.into();
    let mut seg = path.segments.iter();
    let mut pat = pat.segments.iter();
    loop {
        let s = seg.next();
        let p = pat.next();
        match (s, p) {
            (None, None) => break true,
            (Some(_), None) | (None, Some(_)) => break false,
            (Some(s), Some(p)) => {
                if !p.iter().any(|p| p == "*" || s.ident == p) {
                    break false;
                }
            }
        }
    }
}

pub(crate) fn path_starts_with<P: Into<PathPat>>(path: &Path, pat: P) -> bool {
    let pat = pat.into();
    let mut seg = path.segments.iter();
    let mut pat = pat.segments.iter();
    loop {
        let s = seg.next();
        let p = pat.next();
        match (s, p) {
            (None, None) | (Some(_), None) => break true,
            (None, Some(_)) => break false,
            (Some(s), Some(p)) => {
                if !p.iter().any(|p| p == "*" || s.ident == p) {
                    break false;
                }
            }
        }
    }
}

impl From<&str> for PathPat {
    fn from(s: &str) -> Self {
        PathPat {
            segments: vec![vec![s.to_owned()]],
        }
    }
}

impl From<Vec<&str>> for PathPat {
    fn from(v: Vec<&str>) -> Self {
        PathPat {
            segments: v.into_iter().map(|s| vec![s.to_owned()]).collect(),
        }
    }
}

impl From<Vec<Vec<&str>>> for PathPat {
    fn from(vv: Vec<Vec<&str>>) -> Self {
        PathPat {
            segments: vv
                .into_iter()
                .map(|v| v.into_iter().map(|s| s.to_owned()).collect())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Path {
        let expr = syn::parse_str(s).unwrap();
        match expr {
            syn::Expr::Path(path) => path.path,
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_path_match() {
        assert!(path_match(&p("a"), "a"));
        assert!(!path_match(&p("a"), "b"));
        assert!(path_match(&p("a"), "*"));
        assert!(!path_match(&p("a::b"), "a"));
        assert!(path_match(&p("a::b"), vec!["a", "b"]));
        assert!(!path_match(&p("a::b"), vec!["a", "b", "c"]));
        assert!(path_match(&p("a::b::c"), vec!["a", "b", "*"]));
        assert!(path_match(&p("a::b::c"), vec!["a", "*", "c"]));
        assert!(path_match(&p("a::b::c"), vec!["*", "b", "c"]));
        assert!(!path_match(&p("a::b::c"), vec!["*", "*", "d"]));
        assert!(path_match(&p("a"), vec![vec!["a"]]));
        assert!(path_match(&p("a"), vec![vec!["a", "b"]]));
        assert!(!path_match(&p("a"), vec![vec!["b", "c"]]));
    }

    #[test]
    fn test_path_starts_with() {
        assert!(path_starts_with(&p("a"), "a"));
        assert!(!path_starts_with(&p("a"), "b"));
        assert!(path_starts_with(&p("a"), "*"));
        assert!(path_starts_with(&p("a::b"), "a"));
        assert!(!path_starts_with(&p("a::b"), "b"));
        assert!(path_starts_with(&p("a::b"), vec!["a", "b"]));
        assert!(path_starts_with(&p("a::b"), vec!["a", "*"]));
        assert!(!path_starts_with(&p("a"), vec!["a", "*"]));
        assert!(path_starts_with(
            &p("a::b::c"),
            vec![vec!["a"], vec!["b", "bb"]]
        ));
        assert!(!path_starts_with(&p("a::b::c"), vec![vec!["a"], vec!["c"]]));
    }
}
