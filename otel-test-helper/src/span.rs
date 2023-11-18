use std::{borrow::Cow, collections::HashMap, time::SystemTime};

use opentelemetry::{
    trace::{Event, Span, SpanContext, Status},
    KeyValue,
};

use crate::tracer::TestTracer;

#[derive(Debug)]
pub struct TestSpan {
    pub span_context: SpanContext,
    pub tracer: TestTracer,
    pub data: Option<TestSpanData>,
}

#[derive(Debug)]
pub struct TestSpanData {
    pub name: Cow<'static, str>,
    pub events: Vec<Event>,
    pub attributes: HashMap<opentelemetry::Key, opentelemetry::Value>,
    pub status: Status,
}

impl TestSpan {
    pub fn new<T>(name: T, span_context: SpanContext, tracer: TestTracer) -> TestSpan
    where
        T: Into<Cow<'static, str>>,
    {
        let data = TestSpanData {
            name: name.into(),
            events: Vec::new(),
            attributes: HashMap::new(),
            status: Status::Unset,
        };
        TestSpan {
            span_context,
            tracer,
            data: Some(data),
        }
    }

    pub fn with_data<T, F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(&mut TestSpanData) -> T,
    {
        self.data.as_mut().map(f)
    }

    pub fn end(&mut self) {
        let data = match self.data.take() {
            Some(data) => data,
            None => return,
        };
        self.tracer
            .provider()
            .spans
            .push((self.span_context.clone(), data));
    }
}

impl Span for TestSpan {
    fn add_event_with_timestamp<T>(
        &mut self,
        name: T,
        timestamp: SystemTime,
        attributes: Vec<KeyValue>,
    ) where
        T: Into<Cow<'static, str>>,
    {
        self.with_data(|data| data.events.push(Event::new(name, timestamp, attributes, 0)));
    }

    fn span_context(&self) -> &SpanContext {
        &self.span_context
    }

    fn is_recording(&self) -> bool {
        true
    }

    fn set_attribute(&mut self, attribute: KeyValue) {
        self.with_data(|data| data.attributes.insert(attribute.key, attribute.value));
    }

    fn set_status(&mut self, status: Status) {
        self.with_data(|data| {
            data.status = status;
        });
    }

    fn update_name<T>(&mut self, new_name: T)
    where
        T: Into<Cow<'static, str>>,
    {
        self.with_data(|data| {
            data.name = new_name.into();
        });
    }

    fn end_with_timestamp(&mut self, _timestamp: SystemTime) {
        self.end();
    }
}

impl Drop for TestSpan {
    fn drop(&mut self) {
        self.end();
    }
}
