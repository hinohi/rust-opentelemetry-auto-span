# rust-opentelemetry-auto-span

![](./image.png)

## Usage

add dependencies

```toml
[dependencies]
opentelemetry-auto-span = "0.2"
```

annotate function

```rust
use opentelemetry_auto_span::auto_span;

#[get("/user/{id}")]
#[auto_span]
async fn get_user(
    id: web::Path<(i64,)>,
    db: web::Data<sqlx::MySqlPool>,
) -> actix_web::Result<HttpResponse, Error> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(id.into_inner().0)
        .fetch_one(&**db)
        .await?;
    Ok(HttpResponse::Ok().json(&user))
}
```

then, capture bellow information

* function span (from `get_user` start to end)
* `.await` span
* if error return (at `.await?`) and handle by `?`, logging the error

## Convert Example

### use `sqlx::query*`

```rust
#[auto_span]
async fn get_user(
    id: web::Path<(i64,)>,
    db: web::Data<sqlx::MySqlPool>,
) -> actix_web::Result<HttpResponse, Error> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(id.into_inner().0)
        .fetch_one(&**db)
        .await?;
    Ok(HttpResponse::Ok().json(&user))
}
```

↓

```rust
async fn get_user(
    id: web::Path<(i64,)>,
    db: web::Data<sqlx::MySqlPool>,
) -> actix_web::Result<HttpResponse, Error> {
    #[allow(unused_imports)]
    use opentelemetry::trace::{Span, TraceContextExt, Tracer};
    // make tracer
    // tracer name can customize like `#[auto_span(name_def="get_name()")]`
    // Default name `&*TRACE_NAME` is intended to be defined in `lazy_static!`
    let __tracer = opentelemetry::global::tracer(&*TRACE_NAME);
    // start function level span
    let __ctx = opentelemetry::Context::current_with_span(__tracer.start("fn:get_user"));
    let __guard = __ctx.clone().attach();
    let __span = __ctx.span();

    let user: User = {
        // start sqlx `.await` span
        let __ctx = opentelemetry::Context::current_with_span(__tracer.start("db"));
        let __guard = __ctx.clone().attach();
        let __span = __ctx.span();
        __span.set_attribute(opentelemetry::KeyValue::new("aut_span.line", 57i64));
        __span.set_attribute(opentelemetry::KeyValue::new(
            "aut_span.code",
            "let user: User = sqlx::query_as(\"SELECT * FROM users WHERE id = ?\")",
        ));
        {
            // capture SQL string
            __span.set_attribute(opentelemetry::KeyValue::new(
                "sql",
                "SELECT * FROM users WHERE id = ?",
            ));
            sqlx::query_as("SELECT * FROM users WHERE id = ?")
        }
            .bind(id.into_inner().0)
            .fetch_one(&**db)
            .await
    }
        // logging error
        .map_err(|e| {
            __span.set_status(
                ::opentelemetry::trace::StatusCode::Error,
                format!(
                    "line {}, {}\n{}",
                    57i64, "let user: User = sqlx::query_as(\"SELECT * FROM users WHERE id = ?\")", e
                ),
            );
            e
        })?;
    Ok(HttpResponse::Ok().json(&user))
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
