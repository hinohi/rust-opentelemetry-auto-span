use quote::ToTokens;
use std::{collections::HashMap, path::Path};

use syn::{File, Item, ItemFn};

use crate::utils::path_match;

macro_rules! unwrap_or_return {
    ($expr:expr) => {
        match $expr {
            Ok(inner) => inner,
            Err(_) => return,
        }
    };
}

pub struct LineAccess {
    functions: HashMap<String, Vec<ItemFn>>,
}

impl LineAccess {
    pub fn new<P: AsRef<Path>>(root: P) -> LineAccess {
        let mut la = LineAccess {
            functions: HashMap::new(),
        };
        walk(root.as_ref(), &mut |path| la.parse_rs(path));
        la
    }

    fn parse_rs(&mut self, path: &Path) {
        let s = unwrap_or_return!(std::fs::read_to_string(path));
        let file: File = unwrap_or_return!(syn::parse_str(&s));
        for i in file.items {
            let func = match i {
                Item::Fn(f) => f,
                _ => continue,
            };
            println!("{:?}", func);
            if func.attrs.iter().any(|a| path_match(&a.path, "auto_span")) {
                self.functions
                    .entry(func.sig.ident.to_string())
                    .or_default()
                    .push(func);
            }
        }
    }

    pub fn get(&self, f: &ItemFn) -> Option<ItemFn> {
        let name = f.sig.ident.to_string();
        let body = f
            .block
            .stmts
            .iter()
            .map(|s| s.to_token_stream().to_string())
            .collect::<Vec<_>>();
        if let Some(functions) = self.functions.get(&name) {
            for i in functions {
                if body.len() == i.block.stmts.len() {
                    let b = i
                        .block
                        .stmts
                        .iter()
                        .map(|s| s.to_token_stream().to_string())
                        .collect::<Vec<_>>();
                    println!("{:?}", body);
                    println!("{:?}", b);
                    if body == b {
                        return Some(i.clone());
                    }
                }
            }
        }
        None
    }
}

fn walk<F>(path: &Path, task: &mut F)
where
    F: FnMut(&Path),
{
    println!("{:?}", path);
    if path.is_dir() {
        let dir = unwrap_or_return!(path.read_dir());
        for entry in dir {
            let path = unwrap_or_return!(entry).path();
            walk(&path, task);
        }
    } else if path.extension().and_then(|ex| ex.to_str()) == Some(".rs") {
        task(path);
    }
}
