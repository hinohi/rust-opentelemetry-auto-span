use std::marker::PhantomData;

use quote::quote;
use syn::{visit::Visit, Expr, Lit, Meta};

use crate::utils::path_match;

pub struct AttrVisitor<'ast> {
    pub opt: Opt,
    _phantom: PhantomData<&'ast ()>,
}

pub struct Opt {
    pub func_span: bool,
    pub all_await: bool,
    pub debug: bool,
    pub name_def: Option<Expr>,
}

impl<'ast> AttrVisitor<'ast> {
    pub fn new() -> AttrVisitor<'ast> {
        AttrVisitor {
            opt: Opt {
                func_span: true,
                all_await: false,
                debug: false,
                name_def: None,
            },
            _phantom: PhantomData,
        }
    }
}

impl<'ast> Visit<'ast> for AttrVisitor<'ast> {
    fn visit_meta(&mut self, i: &'ast Meta) {
        match i {
            Meta::Path(path) => {
                if path_match(path, "debug") {
                    self.opt.debug = true;
                } else if path_match(path, "no_func_span") {
                    self.opt.func_span = false;
                } else if path_match(path, "all_await") {
                    self.opt.all_await = true;
                } else {
                    panic!("Unexpected option: {:?}", path);
                }
            }
            Meta::NameValue(kv) => {
                if path_match(&kv.path, "name") {
                    assert!(self.opt.name_def.is_none());
                    let s = match &kv.lit {
                        Lit::Str(s) => s.value(),
                        _ => panic!("Unexpected token literal: {:?}", kv.lit),
                    };
                    self.opt.name_def = Some(syn::parse2(quote!(#s)).unwrap());
                } else if path_match(&kv.path, "name_def") {
                    assert!(self.opt.name_def.is_none());
                    let s = match &kv.lit {
                        Lit::Str(s) => s.value(),
                        _ => panic!("Unexpected token literal: {:?}", kv.lit),
                    };
                    let expr = syn::parse_str(&s)
                        .unwrap_or_else(|e| panic!("Syntax error: {} by {}", e, s));
                    self.opt.name_def = Some(expr);
                } else {
                    panic!("Unexpected option: {:?}", kv.path);
                }
            }
            Meta::List(meta_list) => self.visit_meta_list(meta_list),
        }
    }
}
