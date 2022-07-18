use quote::quote;
use syn::{parse_macro_input, visit_mut::VisitMut, Expr, ItemFn};

#[proc_macro_attribute]
pub fn auto_span(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);
    Visitor.visit_item_fn_mut(&mut input);
    insert_tracer(&mut input);
    let token = quote! {#input};
    println!("{}", token);
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
            #(#stmts )*
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

struct Visitor;

impl VisitMut for Visitor {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match i {
            Expr::Call(_) | Expr::MethodCall(_) | Expr::Try(_) => {
                let sql = find_sqlx_query(i).expect("Unsupported SQL style");
                if let Some(sql) = sql {
                    let t = quote! {
                        {
                            let mut __span = __tracer.start(concat!("db:", line!()));
                            __span.set_attribute(opentelemetry::KeyValue::new("sql", #sql));
                            #i
                        }
                    };
                    *i = syn::parse2(t).unwrap();
                }
            }
            _ => syn::visit_mut::visit_expr_mut(self, i),
        };
    }
}

fn find_sqlx_query(i: &Expr) -> Result<Option<Expr>, ()> {
    match i {
        Expr::Array(_) => Ok(None),
        Expr::Assign(_) => Ok(None),
        Expr::AssignOp(_) => Ok(None),
        Expr::Async(_) => Ok(None),
        Expr::Await(expr) => find_sqlx_query(&expr.base),
        Expr::Binary(_) => Ok(None),
        Expr::Block(_) => Ok(None),
        Expr::Box(_) => Ok(None),
        Expr::Break(_) => Ok(None),
        Expr::Call(expr) => {
            if !is_sqlx_query(&expr.func) {
                return Ok(None);
            }
            if expr.args.is_empty() {
                return Err(());
            }
            if let Some(a) = expr.args.first() {
                match a {
                    Expr::Lit(expr) => Ok(Some(Expr::Lit(expr.clone()))),
                    _ => Err(()),
                }
            } else {
                Err(())
            }
        }
        Expr::Cast(_) => Ok(None),
        Expr::Closure(_) => Ok(None),
        Expr::Continue(_) => Ok(None),
        Expr::Field(_) => Ok(None),
        Expr::ForLoop(_) => Ok(None),
        Expr::Group(_) => Ok(None),
        Expr::If(_) => Ok(None),
        Expr::Index(_) => Ok(None),
        Expr::Let(_) => Ok(None),
        Expr::Lit(_) => Ok(None),
        Expr::Loop(_) => Ok(None),
        Expr::Macro(_) => Ok(None),
        Expr::Match(_) => Ok(None),
        Expr::MethodCall(expr) => find_sqlx_query(&expr.receiver),
        Expr::Paren(_) => Ok(None),
        Expr::Path(_) => Ok(None),
        Expr::Range(_) => Ok(None),
        Expr::Reference(_) => Ok(None),
        Expr::Repeat(_) => Ok(None),
        Expr::Return(_) => Ok(None),
        Expr::Struct(_) => Ok(None),
        Expr::Try(expr) => find_sqlx_query(&expr.expr),
        Expr::TryBlock(_) => Ok(None),
        Expr::Tuple(_) => Ok(None),
        Expr::Type(_) => Ok(None),
        Expr::Unary(_) => Ok(None),
        Expr::Unsafe(_) => Ok(None),
        Expr::Verbatim(_) => Ok(None),
        Expr::While(_) => Ok(None),
        Expr::Yield(_) => Ok(None),
        _ => Ok(None),
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
