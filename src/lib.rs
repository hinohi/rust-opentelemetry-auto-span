mod utils;

use std::marker::PhantomData;

use quote::quote;
use syn::{
    parse_macro_input, visit::Visit, visit_mut::VisitMut, AttributeArgs, Expr, ExprAwait, ExprCall,
    ItemFn, Meta,
};

use crate::utils::{path_match, path_starts_with};

#[proc_macro_attribute]
pub fn auto_span(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let opt = {
        let attrs = parse_macro_input!(attr as AttributeArgs);
        let mut visitor = AttrVisitor::new();
        for attr in attrs.iter() {
            visitor.visit_nested_meta(attr);
        }
        visitor.opt
    };

    let mut input = parse_macro_input!(item as ItemFn);
    AwaitVisitor::new(opt.all_await).visit_item_fn_mut(&mut input);
    insert_tracer(&mut input, opt.func_span);
    let token = quote! {#input};

    if opt.debug {
        let mut target = std::path::PathBuf::from(
            std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "/tmp".to_owned()),
        );
        target.push("auto-span");
        std::fs::create_dir_all(&target).unwrap();
        target.push(format!("{}.rs", input.sig.ident));
        std::fs::write(&target, format!("{}", token)).unwrap();
    }

    token.into()
}

fn insert_tracer(i: &mut ItemFn, with_span: bool) {
    let func_span_name = format!("fn:{}", i.sig.ident);
    let stmts = &i.block.stmts;
    let body: Expr = syn::parse2(if with_span {
        quote! {
            {
                #[allow(unused_imports)]
                use opentelemetry::trace::{Tracer, Span};
                let __tracer = opentelemetry::global::tracer(TRACE_NAME);
                let __span = __tracer.start(#func_span_name);
                #(#stmts)*
            }
        }
    } else {
        quote! {
            {
                #[allow(unused_imports)]
                use opentelemetry::trace::{Tracer, Span};
                let __tracer = opentelemetry::global::tracer(TRACE_NAME);
                #(#stmts)*
            }
        }
    })
    .unwrap();
    match body {
        Expr::Block(block) => {
            i.block.stmts = block.block.stmts;
        }
        _ => unreachable!(),
    }
}

struct AwaitVisitor {
    all_await: bool,
}

struct SqlxVisitor {
    mutate: bool,
}

struct ReqwestVisitor {
    mutate: bool,
}

impl AwaitVisitor {
    fn new(all_await: bool) -> AwaitVisitor {
        AwaitVisitor { all_await }
    }

    fn handle_sqlx(&self, expr_await: &mut ExprAwait) -> bool {
        let mut visitor = SqlxVisitor::new();
        visitor.visit_expr_await_mut(expr_await);
        visitor.mutate
    }

    fn handle_reqwest(&self, expr_await: &mut ExprAwait) -> bool {
        let mut visitor = ReqwestVisitor::new();
        visitor.visit_expr_await_mut(expr_await);
        visitor.mutate
    }
}

impl VisitMut for AwaitVisitor {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match i {
            Expr::Await(expr) => {
                if self.handle_sqlx(expr) {
                    let t = quote! {
                        {
                            let mut __span = __tracer.start(concat!("db:", line!()));
                            #expr
                        }
                    };
                    *i = syn::parse2(t).unwrap();
                } else if self.handle_reqwest(expr) {
                    let t = quote! {
                        {
                            let mut __span = __tracer.start(concat!("http:", line!()));
                            #expr
                        }
                    };
                    *i = syn::parse2(t).unwrap();
                } else {
                    syn::visit_mut::visit_expr_await_mut(self, expr);
                    if self.all_await {
                        let t = quote! {
                            {
                                let mut __span = __tracer.start(concat!("await:", line!()));
                                #expr
                            }
                        };
                        *i = syn::parse2(t).unwrap();
                    }
                }
            }
            _ => syn::visit_mut::visit_expr_mut(self, i),
        };
    }
}

impl SqlxVisitor {
    fn new() -> SqlxVisitor {
        SqlxVisitor { mutate: false }
    }

    fn try_sqlx(&self, call: &ExprCall) -> Option<Expr> {
        if !is_sqlx_query(&call.func) {
            return None;
        }
        if let Some(a) = call.args.first() {
            match a {
                Expr::Lit(_) | Expr::Path(_) => Some(a.clone()),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl VisitMut for SqlxVisitor {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        let sql = match i {
            Expr::Call(expr) => self.try_sqlx(expr),
            Expr::Await(_) => return,
            _ => None,
        };
        if let Some(sql) = sql {
            let t = quote! {
                {
                    __span.set_attribute(opentelemetry::KeyValue::new("sql", #sql));
                    #i
                }
            };
            *i = syn::parse2(t).unwrap();
            self.mutate = true;
        } else {
            syn::visit_mut::visit_expr_mut(self, i);
        }
    }
}

fn is_sqlx_query(func: &Expr) -> bool {
    let query_functions = vec![
        "query",
        "query_as",
        "query_as_with",
        "query_scalar",
        "query_scalar_with",
        "query_with",
    ];
    match func {
        Expr::Path(path) => path_match(&path.path, vec![vec!["sqlx"], query_functions]),
        _ => false,
    }
}

impl ReqwestVisitor {
    fn new() -> ReqwestVisitor {
        ReqwestVisitor { mutate: false }
    }
}

impl VisitMut for ReqwestVisitor {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        let b = match i {
            Expr::Call(expr) => is_reqwest(&expr.func),
            Expr::Await(_) => return,
            _ => false,
        };
        if b {
            self.mutate = true;
        } else {
            syn::visit_mut::visit_expr_mut(self, i);
        }
    }
}

fn is_reqwest(func: &Expr) -> bool {
    match func {
        Expr::Path(path) => path_starts_with(&path.path, vec!["reqwest", "*"]),
        _ => false,
    }
}

struct AttrVisitor<'ast> {
    opt: Opt,
    _phantom: PhantomData<&'ast ()>,
}

struct Opt {
    func_span: bool,
    all_await: bool,
    debug: bool,
}

impl<'ast> AttrVisitor<'ast> {
    fn new() -> AttrVisitor<'ast> {
        AttrVisitor {
            opt: Opt {
                func_span: true,
                all_await: true,
                debug: false,
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
                } else if path_match(path, "no_all_await") {
                    self.opt.all_await = false;
                } else {
                    panic!("Unexpected option: {:?}", path);
                }
            }
            Meta::NameValue(_) => (),
            Meta::List(meta_list) => self.visit_meta_list(meta_list),
        }
    }
}
