use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::{Attribute, File, Item, ItemFn, Meta};

use crate::utils::path_match;

pub fn find_source_path<P: AsRef<Path>>(root: P, func: &ItemFn) -> Option<PathBuf> {
    let name = func.sig.ident.to_string();
    let sig = func.to_token_stream().to_string();
    walk(root.as_ref(), &mut |file| {
        is_contain_target_func(file, &name, &sig)
    })
}

fn is_contain_target_func(file: File, name: &str, sig: &str) -> bool {
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
        let is_auto_span = match &attr.meta {
            Meta::Path(path) => path_match(path, "auto_span"),
            Meta::List(lis) => path_match(&lis.path, "auto_span"),
            Meta::NameValue(_) => false,
        };
        if is_auto_span {
            return Some(attrs[i + 1..].to_vec());
        }
    }
    None
}

fn walk<F>(path: &Path, task: &mut F) -> Option<PathBuf>
where
    F: FnMut(File) -> bool,
{
    if path.is_dir() {
        let dir = path.read_dir().ok()?;
        for entry in dir {
            let path = entry.ok()?.path();
            if let Some(path) = walk(&path, task) {
                return Some(path);
            }
        }
    } else if path.extension().and_then(|ex| ex.to_str()) == Some("rs") {
        let content = std::fs::read_to_string(path).ok()?;
        let file = syn::parse_file(&content).ok()?;
        if task(file) {
            return Some(path.to_path_buf());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_contain_target_func() {
        let target_func = r#"#[get("/")] pub fn a() -> &'static str { "hello" }"#;
        let func_item = syn::parse_str::<ItemFn>(target_func).unwrap();
        let sig = func_item.to_token_stream().to_string();

        let target_file = r#"
use actix_web::get;

fn b() -> i32 { 1 }

/// do a
/// this is comment
#[auto_span]
#[get("/")]
pub fn a() -> &'static str {
    "hello"
}
"#;
        assert!(is_contain_target_func(
            syn::parse_file(target_file).unwrap(),
            &func_item.sig.ident.to_string(),
            &sig
        ))
    }

    #[test]
    fn strip_attrs_no_option() {
        let target_func = r#"
/// document
#[auto_span]
pub fn a() -> &'static str {
    "hello"
}"#;
        let func_item = syn::parse_str::<ItemFn>(target_func).unwrap();
        assert!(strip_attrs(&func_item.attrs).unwrap().is_empty());
    }

    #[test]
    fn strip_attrs_with_option() {
        let target_func = r#"#[auto_span(debug)] pub fn a() -> &'static str { "hello" }"#;
        let func_item = syn::parse_str::<ItemFn>(target_func).unwrap();
        assert!(strip_attrs(&func_item.attrs).unwrap().is_empty());
    }
}
