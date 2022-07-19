use syn::Path;

pub struct PathPat {
    segments: Vec<Vec<String>>,
}

pub(crate) fn match_path<P: Into<PathPat>>(path: &Path, pat: P) -> bool {
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
                if p.iter().all(|p| s.ident != p) {
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
