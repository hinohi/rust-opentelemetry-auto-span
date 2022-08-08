mod attr_options;
mod dig;
mod line;
mod utils;

use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, visit::Visit, visit_mut::VisitMut, AttributeArgs, Expr,
    ExprAwait, ExprCall, ItemFn,
};

use crate::{
    attr_options::AttrVisitor,
    dig::find_source_path,
    line::LineAccess,
    utils::{path_match, path_starts_with},
};

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

    let mut dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    dir.push("src");
    let line_access = if let Some(path) = find_source_path(dir, &input) {
        Some(LineAccess::new(path))
    } else {
        None
    };
    AwaitVisitor::new(line_access, opt.all_await).visit_item_fn_mut(&mut input);
    let tracer_expr = opt
        .name_def
        .unwrap_or_else(|| syn::parse2(quote!(&*TRACE_NAME)).unwrap());
    insert_tracer(&mut input, opt.func_span, tracer_expr);
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

fn insert_tracer(i: &mut ItemFn, with_span: bool, tracer_expr: Expr) {
    let func_span_name = format!("fn:{}", i.sig.ident);
    let stmts = &i.block.stmts;
    let body: Expr = syn::parse2(if with_span {
        quote! {
            {
                #[allow(unused_imports)]
                use opentelemetry::trace::{Tracer, Span, TraceContextExt};
                let __tracer = opentelemetry::global::tracer(#tracer_expr);
                let __ctx = opentelemetry::Context::current_with_span(__tracer.start(#func_span_name));
                let __guard = __ctx.clone().attach();
                let __span = __ctx.span();
                #(#stmts)*
            }
        }
    } else {
        quote! {
            {
                #[allow(unused_imports)]
                use opentelemetry::trace::{Tracer, Span, TraceContextExt};
                let __tracer = opentelemetry::global::tracer(#tracer_expr);
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
    line_access: Option<LineAccess>,
    all_await: bool,
}

struct SqlxVisitor {
    mutate: bool,
}

struct ReqwestVisitor {
    mutate: bool,
}

impl AwaitVisitor {
    fn new(line_access: Option<LineAccess>, all_await: bool) -> AwaitVisitor {
        AwaitVisitor {
            line_access,
            all_await,
        }
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

    fn get_line_info(&self, name: &str, span: Span) -> (String, Option<String>) {
        if let Some(ref line_access) = self.line_access {
            if let Some((n, line)) = line_access.span(span) {
                return (format!("{}:#L{}", name, n), Some(line));
            }
        }
        (name.to_owned(), None)
    }
}

impl VisitMut for AwaitVisitor {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        let span = i.span();

        let new_span = |name, line, expr| {
            let mut tokens = quote! {
                let __ctx = opentelemetry::Context::current_with_span(__tracer.start(#name));
                let __guard = __ctx.clone().attach();
                let __span = __ctx.span();
            };
            if let Some(line) = line {
                tokens.extend(quote! {
                    __span.set_attribute(opentelemetry::KeyValue::new("line", #line));
                });
            }
            let tokens = quote! {
                {
                    #tokens
                    #expr
                }
            };
            syn::parse2(tokens).unwrap()
        };

        match i {
            Expr::Await(expr) => {
                if self.handle_sqlx(expr) {
                    let (name, line) = self.get_line_info("db", span);
                    *i = new_span(name, line, expr);
                } else if self.handle_reqwest(expr) {
                    let (name, line) = self.get_line_info("http", span);
                    *i = new_span(name, line, expr);
                } else {
                    syn::visit_mut::visit_expr_await_mut(self, expr);
                    if self.all_await {
                        let (name, line) = self.get_line_info("await", span);
                        *i = new_span(name, line, expr);
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
