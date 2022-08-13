mod attr_options;
mod dig;
mod handle_reqwest;
mod handle_sqlx;
mod line;
mod utils;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, visit::Visit, visit_mut::VisitMut, AttributeArgs, Expr,
    ExprAwait, ExprClosure, ExprTry, ItemFn, Signature,
};

use crate::{attr_options::AttrVisitor, dig::find_source_path, line::LineAccess};

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
    let line_access = find_source_path(dir, &input).map(LineAccess::new);
    let mut visitor = AutoSpanVisitor::new(line_access, opt.all_await);
    visitor.visit_item_fn_mut(&mut input);

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
    let mut tokens = quote! {
        #[allow(unused_imports)]
        use opentelemetry::trace::{Tracer, Span, TraceContextExt};
        let __tracer = opentelemetry::global::tracer(#tracer_expr);
    };
    if with_span {
        tokens.extend(quote! {
            let __ctx = opentelemetry::Context::current_with_span(__tracer.start(#func_span_name));
            let __guard = __ctx.clone().attach();
            let __span = __ctx.span();
            #(#stmts)*
        });
    }
    let body: Expr = syn::parse2(quote! {{#tokens}}).unwrap();
    match body {
        Expr::Block(block) => {
            i.block.stmts = block.block.stmts;
        }
        _ => unreachable!(),
    }
}

struct AutoSpanVisitor {
    line_access: Option<LineAccess>,
    context: Vec<ReturnTypeContext>,
    all_await: bool,
}

#[derive(Copy, Clone)]
enum ReturnTypeContext {
    Unknown,
    Result,
    Option,
}

impl AutoSpanVisitor {
    fn new(line_access: Option<LineAccess>, all_await: bool) -> AutoSpanVisitor {
        AutoSpanVisitor {
            line_access,
            context: Vec::new(),
            all_await,
        }
    }

    fn push_fn_context(&mut self, sig: &Signature) {
        let rt = match &sig.output {
            syn::ReturnType::Default => ReturnTypeContext::Unknown,
            syn::ReturnType::Type(_, ty) => match ty.as_ref() {
                syn::Type::Path(path) => {
                    let name = path.path.segments.last().unwrap().ident.to_string();
                    if name.contains("Result") {
                        ReturnTypeContext::Result
                    } else if name.contains("Option") {
                        ReturnTypeContext::Option
                    } else {
                        ReturnTypeContext::Unknown
                    }
                }
                _ => ReturnTypeContext::Unknown,
            },
        };
        self.context.push(rt);
    }

    pub fn push_closure_context(&mut self) {
        self.context.push(ReturnTypeContext::Unknown);
    }

    pub fn pop_context(&mut self) {
        self.context.pop();
    }

    pub fn current_context(&self) -> ReturnTypeContext {
        *self.context.last().unwrap()
    }

    fn handle_sqlx(&self, expr_await: &mut ExprAwait) -> bool {
        let mut visitor = handle_sqlx::SqlxVisitor::new();
        visitor.visit_expr_await_mut(expr_await);
        visitor.is_mutate()
    }

    fn handle_reqwest(&self, expr_await: &mut ExprAwait) -> bool {
        let mut visitor = handle_reqwest::ReqwestVisitor::new();
        visitor.visit_expr_await_mut(expr_await);
        visitor.is_mutate()
    }

    fn get_line_info(&self, span: Span) -> Option<(i64, String)> {
        self.line_access.as_ref().and_then(|la| la.span(span))
    }
}

impl VisitMut for AutoSpanVisitor {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        self.push_fn_context(&i.sig);
        if self.context.len() == 1 {
            // skip inner function, because `span` is not shared
            syn::visit_mut::visit_item_fn_mut(self, i);
        }
        self.pop_context();
    }

    fn visit_expr_mut(&mut self, i: &mut Expr) {
        let span = i.span();

        let add_line_info = |tokens: &mut TokenStream, line| {
            if let Some((line, code)) = line {
                tokens.extend(quote! {
                    __span.set_attribute(opentelemetry::KeyValue::new("aut_span.line", #line));
                    __span.set_attribute(opentelemetry::KeyValue::new("aut_span.code", #code));
                });
            }
        };
        let new_span = |name, line, expr| {
            let mut tokens = quote! {
                let __ctx = opentelemetry::Context::current_with_span(__tracer.start(#name));
                let __guard = __ctx.clone().attach();
                let __span = __ctx.span();
            };
            add_line_info(&mut tokens, line);
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
                    *i = new_span("db", self.get_line_info(span), expr);
                } else if self.handle_reqwest(expr) {
                    *i = new_span("http", self.get_line_info(span), expr);
                } else {
                    syn::visit_mut::visit_expr_await_mut(self, expr);
                    if self.all_await {
                        *i = new_span("await", self.get_line_info(span), expr);
                    }
                }
            }
            _ => syn::visit_mut::visit_expr_mut(self, i),
        };
    }

    fn visit_expr_try_mut(&mut self, i: &mut ExprTry) {
        syn::visit_mut::visit_expr_try_mut(self, i);

        let span = i.span();
        match self.current_context() {
            ReturnTypeContext::Result => {
                let inner = i.expr.as_ref();
                let err = if let Some((line, code)) = self.get_line_info(span) {
                    quote! {format!("line {}, {}\n{}", #line, #code, e)}
                } else {
                    quote! {format!("{}", e)}
                };
                let tokens = quote! {
                    __span.set_status(::opentelemetry::trace::StatusCode::Error, #err);
                };
                i.expr = Box::new(
                    syn::parse2(quote! {
                        #inner.map_err(|e| { #tokens e })
                    })
                    .unwrap(),
                );
            }
            _ => (),
        }
    }

    fn visit_expr_closure_mut(&mut self, i: &mut ExprClosure) {
        self.push_closure_context();
        syn::visit_mut::visit_expr_closure_mut(self, i);
        self.pop_context();
    }
}
