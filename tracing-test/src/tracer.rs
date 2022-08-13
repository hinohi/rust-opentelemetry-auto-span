use std::{
    borrow::Cow,
    sync::{Arc, Mutex, MutexGuard},
};

use opentelemetry::{
    trace::{SpanBuilder, SpanContext, TraceContextExt, Tracer},
    Context,
};

use crate::{provider::TestTracerProviderInner, span::TestSpan};

#[derive(Debug, Clone)]
pub struct TestTracer {
    pub provider: Arc<Mutex<TestTracerProviderInner>>,
}

impl TestTracer {
    pub fn new(provider: Arc<Mutex<TestTracerProviderInner>>) -> TestTracer {
        TestTracer { provider }
    }

    pub fn provider(&mut self) -> MutexGuard<TestTracerProviderInner> {
        self.provider.lock().unwrap()
    }
}

impl Tracer for TestTracer {
    type Span = TestSpan;

    fn start_with_context<T>(&self, name: T, parent_cx: &Context) -> Self::Span
    where
        T: Into<Cow<'static, str>>,
    {
        self.build_with_context(SpanBuilder::from_name(name), parent_cx)
    }

    fn span_builder<T>(&self, name: T) -> SpanBuilder
    where
        T: Into<Cow<'static, str>>,
    {
        SpanBuilder::from_name(name)
    }

    fn build_with_context(&self, builder: SpanBuilder, parent_cx: &Context) -> Self::Span {
        let mut provider = self.provider.lock().unwrap();

        let parent_span = if parent_cx.has_active_span() {
            Some(parent_cx.span())
        } else {
            None
        };
        let trace_id = if let Some(sc) = parent_span.as_ref().map(|parent| parent.span_context()) {
            sc.trace_id()
        } else {
            builder.trace_id.unwrap_or_else(|| provider.new_trace_id())
        };
        let span_id = provider.new_span_id();

        let span_context = SpanContext::new(
            trace_id,
            span_id,
            Default::default(),
            false,
            Default::default(),
        );
        TestSpan::new(builder.name, span_context, self.clone())
    }
}
