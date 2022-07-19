use rust_opentelemetry_auto_span::auto_span;

const TRACE_NAME: &str = "opts";

#[auto_span]
fn a() -> i32 {
    0b0000
}

#[auto_span(debug)]
fn b() -> i32 {
    0b0001
}

#[auto_span(no_func_span)]
fn c() -> i32 {
    0b0010
}

#[auto_span(debug, no_func_span)]
fn d() -> i32 {
    0b0011
}

#[auto_span(no_func_span, debug)]
fn e() -> i32 {
    0b0011
}

fn main() {
    let _ = a();
    let _ = b();
    let _ = c();
    let _ = d();
    let _ = e();
}
