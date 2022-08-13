

use syn::{
    visit_mut::VisitMut, Expr,
};

use crate::{
    utils::path_starts_with,
};

pub struct ReqwestVisitor {
    mutate: bool,
}

impl ReqwestVisitor {
    pub fn new() -> ReqwestVisitor {
        ReqwestVisitor { mutate: false }
    }

    pub fn is_mutate(&self) -> bool {
        self.mutate
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
