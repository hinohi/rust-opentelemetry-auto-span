use std::sync::{Arc, Mutex};

use opentelemetry::{
    trace::{SpanContext, SpanId, TraceId, TracerProvider},
    InstrumentationLibrary,
};

use crate::{span::TestSpanData, tracer::TestTracer};

#[derive(Debug)]
pub struct TestTracerProvider {
    pub inner: Arc<Mutex<TestTracerProviderInner>>,
}

#[derive(Debug)]
pub struct TestTracerProviderInner {
    pub id: u64,
    pub spans: Vec<(SpanContext, TestSpanData)>,
}

impl TestTracerProvider {
    pub fn new(inner: Arc<Mutex<TestTracerProviderInner>>) -> TestTracerProvider {
        TestTracerProvider { inner }
    }
}

impl TestTracerProviderInner {
    pub fn new() -> TestTracerProviderInner {
        TestTracerProviderInner {
            id: 1,
            spans: Vec::new(),
        }
    }

    pub fn new_span_id(&mut self) -> SpanId {
        let id = SpanId::from_bytes(self.id.to_ne_bytes());
        self.id += 1;
        id
    }

    pub fn new_trace_id(&mut self) -> TraceId {
        let id = TraceId::from_bytes((self.id as u128).to_ne_bytes());
        self.id += 1;
        id
    }
}

impl TracerProvider for TestTracerProvider {
    type Tracer = TestTracer;

    fn library_tracer(&self, _library: Arc<InstrumentationLibrary>) -> Self::Tracer {
        TestTracer::new(self.inner.clone())
    }
}
