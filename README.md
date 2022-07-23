# rust-opentelemetry-auto-span

## What is this?

This library is intended for easy insertion of the opentelemetry's spans
into rust code containing `await`.
Simply annotate the function you wish to measure with `#[auto_span]`,
and the span generation code will be inserted into the function itself,
as well as the await for `sqlx` and `reqwest`,
which are used inside the function,
to make it possible to measure the function with the opentelemetry.

## Usage

1. Add dependency (not published for creates.io yet)

   ```toml
   opentelemetry = { version = "0.17", default-features = false, features = ["trace", "rt-tokio-current-thread"] }
   opentelemetry-jaeger = { version = "0.16", features = ["rt-tokio-current-thread"] }
   actix-web-opentelemetry = { git = "https://github.com/OutThereLabs/actix-web-opentelemetry" }
   ```
2. Define `const TRACE_NAME: &str = "・・・`
3. Initialize opentelemetry-jaeger

   ```rust
   use opentelemetry::{
       global, runtime::TokioCurrentThread, sdk::propagation::TraceContextPropagator,
   };

   global::set_text_map_propagator(TraceContextPropagator::new());
   let _tracer = opentelemetry_jaeger::new_pipeline()
       .with_service_name(TRACE_NAME)
       .with_agent_endpoint("192.168.0.12:6831")
       .with_auto_split_batch(true)
       .install_batch(TokioCurrentThread)
       .expect("pipeline install error");
   ```
4. Add annotation

   ```rust
   use rust_opentelemetry_auto_span::auto_span;

   #[auto_span]
   async fn my_func() {}
   ```

## Convert Example

### Just function

```rust
#[auto_span]
fn b() -> i32 {
    0b0001
}
```

↓

```rust
fn b() -> i32 {
    #[allow(unused_imports)]
    use opentelemetry::trace::{Span, Tracer};
    let __tracer = opentelemetry::global::tracer(TRACE_NAME);
    let __span = __tracer.start("fn:b");
    0b0001
}
```

### use `sqlx::query*`

```rust
#[auto_span(debug)]
async fn greet(pool: web::Data<sqlx::SqlitePool>) -> actix_web::Result<String> {
    let r: Vec<i32> = sqlx::query_scalar("SELECT id")
        .fetch_all(pool.as_ref())
        .await
        .map_err(SqlxError)?;
    Ok(format!("Hello {:?}!", r))
}
```

↓

```rust
async fn greet(pool: web::Data<sqlx::SqlitePool>) -> actix_web::Result<String> {
    #[allow(unused_imports)]
    use opentelemetry::trace::{Span, Tracer};
    let __tracer = opentelemetry::global::tracer(TRACE_NAME);
    let __span = __tracer.start("fn:greet");
    let r: Vec<i32> = {
        let mut __span = __tracer.start(concat!("db:", line!()));
        {
            __span.set_attribute(opentelemetry::KeyValue::new("sql", "SELECT id"));
            sqlx::query_scalar("SELECT id")
        }
        .fetch_all(pool.as_ref())
        .await
    }
    .map_err(SqlxError)?;
    Ok(format!("Hello {:?}!", r))
}
```

### use `sqlx::query*` many times until await

```rust
#[auto_span]
async fn test_if(pool: web::Data<sqlx::SqlitePool>) -> actix_web::Result<String> {
    let r: Vec<i32> = if true {
        sqlx::query_scalar("SELECT b").fetch_all(pool.as_ref())
    } else {
        sqlx::query_scalar("SELECT a").fetch_all(pool.as_ref())
    }
    .await
    .map_err(SqlxError)?;
    Ok(format!("r={:?}", r))
}
```

↓

```rust
async fn test_if(pool: web::Data<sqlx::SqlitePool>) -> actix_web::Result<String> {
    #[allow(unused_imports)]
    use opentelemetry::trace::{Span, Tracer};
    let __tracer = opentelemetry::global::tracer(TRACE_NAME);
    let __span = __tracer.start("fn:test_if");
    let r: Vec<i32> = {
        let mut __span = __tracer.start(concat!("db:", line!()));
        if true {
            {
                __span.set_attribute(opentelemetry::KeyValue::new("sql", "SELECT b"));
                sqlx::query_scalar("SELECT b")
            }
            .fetch_all(pool.as_ref())
        } else {
            {
                __span.set_attribute(opentelemetry::KeyValue::new("sql", "SELECT a"));
                sqlx::query_scalar("SELECT a")
            }
            .fetch_all(pool.as_ref())
        }
        .await
    }
    .map_err(SqlxError)?;
    Ok(format!("r={:?}", r))
}
```

## Option

usage:

```rust
#[auto_span(debug)]
fn my_func() {}
```

| name         | action                                                         |
|:-------------|:---------------------------------------------------------------|
| debug        | Dump the migrated code to ./target/auto_span or /tmp/auto_span |
| no_func_span | Not generate function level span split                         |
| no_all_await | Generate span for `await` of `sqlx` and `reqwest` only         |
