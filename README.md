# rust-opentelemetry-auto-span

![](./image.png)

## Usage

add dependencies

```toml
[dependencies]
opentelemetry-auto-span = "0.4"
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
* `.await` of `sqlx::query*` span
    * also capture SQL string
* if error return (at `.await?`) and handle by `?`, logging the error

See Examples.
