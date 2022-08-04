# rust-opentelemetry-auto-span

![](./image.png)

## Convert Example

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
   use opentelemetry::trace::{Span, TraceContextExt, Tracer};
   let __tracer = opentelemetry::global::tracer(&*TRACE_NAME);
   let __ctx = opentelemetry::Context::current_with_span(__tracer.start("fn:greet"));
   let __guard = __ctx.clone().attach();
   let __span = __ctx.span();
   let r: Vec<i32> = {
      let __ctx = opentelemetry::Context::current_with_span(__tracer.start("db"));
      let __guard = __ctx.clone().attach();
      let __span = __ctx.span();
      {
         __span.set_attribute(opentelemetry::KeyValue::new("sql", "SELECT id"));
         sqlx::query_scalar("SELECT id")
      }.fetch_all(pool.as_ref()).await
   }.map_err(SqlxError)?;
   Ok(format!("Hello {:?}!", r))
}
```

## Option

usage:

```rust
#[auto_span(debug)]
fn my_func() {}
```

| name          | action                                                               |
|:--------------|:---------------------------------------------------------------------|
| name/name_def | Tracer name token. `name` must be str, `name_def` parse as rust expr |
| debug         | Dump the migrated code to ./target/auto_span or /tmp/auto_span       |
| no_func_span  | Not generate function level span split                               |
| all_await     | Generate span for all `await`                                        |
