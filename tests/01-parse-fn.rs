use rust_opentelemetry_auto_span::auto_span;

#[auto_span]
fn test_func(a: i32) -> i32 {
    let b = a.pow(2);
    b
}

fn main() {
    println!("{}", test_func(21));
}
