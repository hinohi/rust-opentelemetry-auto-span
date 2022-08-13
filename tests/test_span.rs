use std::sync::{Arc, Mutex};

use opentelemetry::global;
use opentelemetry_auto_span::auto_span;
use tracing_test::{TestTracerProvider, TestTracerProviderInner};

const TRACE_NAME: &str = "test_test";

#[auto_span]
fn f(x: i32) -> i32 {
    x * x
}

#[auto_span]
fn g(x: i32) -> i32 {
    f(x) * f(x)
}

#[test]
fn main() {
    let inner = Arc::new(Mutex::new(TestTracerProviderInner::new()));
    let provider = TestTracerProvider::new(inner.clone());
    let _ = global::set_tracer_provider(provider);

    let _ = g(12);

    let spans = &inner.lock().unwrap().spans;
    let mut span_iter = spans.iter();
    {
        let data = &span_iter.next().unwrap().1;
        assert_eq!(data.name, "fn:f");
    }
    {
        let data = &span_iter.next().unwrap().1;
        assert_eq!(data.name, "fn:f");
    }
    {
        let data = &span_iter.next().unwrap().1;
        assert_eq!(data.name, "fn:g");
    }
    assert!(span_iter.next().is_none())
}
