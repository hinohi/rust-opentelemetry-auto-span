use quote::quote;
use syn::{visit_mut::VisitMut, Expr, ExprCall};

use crate::utils::path_match;

pub struct SqlxVisitor {
    mutate: bool,
}

impl SqlxVisitor {
    pub fn new() -> SqlxVisitor {
        SqlxVisitor { mutate: false }
    }

    pub fn is_mutate(&self) -> bool {
        self.mutate
    }

    fn try_sqlx(&self, call: &ExprCall) -> Option<Expr> {
        if !is_sqlx_query(&call.func) {
            return None;
        }
        let a = call.args.first()?;
        match a {
            Expr::Lit(_) | Expr::Path(_) => Some(a.clone()),
            _ => None,
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
                    ::opentelemetry::trace::get_active_span(|span| {
                        span.set_attribute(::opentelemetry::KeyValue::new("db.statement", #sql));
                    });
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
        "raw_sql",
    ];
    match func {
        Expr::Path(path) => path_match(&path.path, vec![vec!["sqlx"], query_functions]),
        _ => false,
    }
}
