use quote::quote;
use syn::{parse_macro_input, visit_mut::VisitMut, Expr, ExprAwait, ExprCall, ItemFn};

#[proc_macro_attribute]
pub fn auto_span(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);
    AwaitVisitor::new().visit_item_fn_mut(&mut input);
    insert_tracer(&mut input);
    let token = quote! {#input};
    token.into()
}

fn insert_tracer(i: &mut ItemFn) {
    let func_span_name = format!("fn:{}", i.sig.ident);
    let stmts = &i.block.stmts;
    let body: Expr = syn::parse2(quote! {
        {
            #[allow(unused_imports)]
            use opentelemetry::trace::{Tracer, Span};
            let __tracer = opentelemetry::global::tracer(TRACE_NAME);
            let __span = __tracer.start(#func_span_name);
            #(#stmts)*
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

struct AwaitVisitor;

struct SqlxVisitor {
    mutate: bool,
}

impl AwaitVisitor {
    fn new() -> AwaitVisitor {
        AwaitVisitor
    }

    fn handle_sqlx(&self, expr_await: &mut ExprAwait) -> bool {
        let mut sqlx_visitor = SqlxVisitor::new();
        sqlx_visitor.visit_expr_await_mut(expr_await);
        sqlx_visitor.mutate
    }
}

impl VisitMut for AwaitVisitor {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match i {
            Expr::Await(expr) => {
                if self.handle_sqlx(expr) {
                    let attrs = &expr.attrs;
                    let base = &expr.base;
                    let t = quote! {
                        {
                            let mut __span = __tracer.start(concat!("db:", line!()));
                            #(#attrs)*
                            #base
                        }.await
                    };
                    *i = syn::parse2(t).unwrap();
                } else {
                    syn::visit_mut::visit_expr_await_mut(self, expr);
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
                Expr::Lit(_) => Some(a.clone()),
                Expr::Path(_) => {
                    let t = quote! {&#a};
                    Some(syn::parse2(t).unwrap())
                }
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
    let query_functions = [
        "query",
        "query_as",
        "query_as_with",
        "query_scalar",
        "query_scalar_with",
        "query_with",
    ];
    match func {
        Expr::Path(path) => {
            let mut it = path.path.segments.iter();
            if let Some(s) = it.next() {
                if s.ident == "sqlx" {
                    if let Some(s) = it.next() {
                        if query_functions.iter().any(|q| s.ident == q) {
                            return it.next().is_none();
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}
