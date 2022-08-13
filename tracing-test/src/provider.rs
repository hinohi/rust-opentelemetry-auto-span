use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};

use opentelemetry::{
    trace::{
        SpanContext, SpanId, TraceId, TraceResult, TracerProvider,
    },
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
    pub fn new() -> TestTracerProvider {
        TestTracerProvider {
            inner: Arc::new(Mutex::new(TestTracerProviderInner {
                id: 1,
                spans: Vec::new(),
            })),
        }
    }
}

impl TestTracerProviderInner {
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

    fn versioned_tracer(
        &self,
        _name: impl Into<Cow<'static, str>>,
        _version: Option<&'static str>,
        _schema_url: Option<&'static str>,
    ) -> Self::Tracer {
        TestTracer::new(self.inner.clone())
    }

    fn force_flush(&self) -> Vec<TraceResult<()>> {
        Vec::new()
    }
}
