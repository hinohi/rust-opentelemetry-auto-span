use rust_opentelemetry_auto_span::auto_span;

const TRACE_NAME: &str = "a";

#[auto_span(debug)]
fn test_func(a: i32) -> i32 {
    let b = a.pow(2);
    b
}

fn main() {
    let _ = test_func(21);
}
