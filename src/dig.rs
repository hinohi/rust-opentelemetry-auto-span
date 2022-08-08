use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::{Attribute, Item, ItemFn};

use crate::utils::path_match;

macro_rules! unwrap_or_return {
    ($expr:expr, $ret:expr) => {
        match $expr {
            Ok(inner) => inner,
            Err(_) => return $ret,
        }
    };
}

pub fn find_source_path<P: AsRef<Path>>(root: P, func: &ItemFn) -> Option<PathBuf> {
    let name = func.sig.ident.to_string();
    let sig = func.to_token_stream().to_string();
    walk(root.as_ref(), &mut |path| {
        is_contain_target_func(path, &name, &sig)
    })
}

fn is_contain_target_func(path: &Path, name: &str, sig: &str) -> bool {
    let content = unwrap_or_return!(std::fs::read_to_string(path), false);
    let file = unwrap_or_return!(syn::parse_file(&content), false);
    for item in file.items {
        if let Item::Fn(mut func) = item {
            if func.sig.ident != name {
                continue;
            }
            if let Some(attrs) = strip_attrs(&func.attrs) {
                func.attrs = attrs;
                if func.to_token_stream().to_string() == sig {
                    return true;
                }
            }
        }
    }
    false
}

fn strip_attrs(attrs: &[Attribute]) -> Option<Vec<Attribute>> {
    for (i, attr) in attrs.iter().enumerate() {
        if path_match(&attr.path, "auto_span") {
            return Some(attrs[i + 1..].to_vec());
        }
    }
    None
}

fn walk<F>(path: &Path, task: &mut F) -> Option<PathBuf>
where
    F: FnMut(&Path) -> bool,
{
    if path.is_dir() {
        let dir = unwrap_or_return!(path.read_dir(), None);
        for entry in dir {
            let path = unwrap_or_return!(entry, None).path();
            if let Some(path) = walk(&path, task) {
                return Some(path);
            }
        }
    } else if path.extension().and_then(|ex| ex.to_str()) == Some("rs") && task(path) {
        return Some(path.to_path_buf());
    }
    None
}
