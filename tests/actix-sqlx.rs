use actix_web::{get, web, HttpResponse};
use actix_web_opentelemetry::RequestTracing;
use opentelemetry::{
    global, runtime::TokioCurrentThread, sdk::propagation::TraceContextPropagator,
};
use rust_opentelemetry_auto_span::auto_span;

const TRACE_NAME: &str = "a";

#[derive(Debug)]
struct SqlxError(sqlx::Error);

impl std::fmt::Display for SqlxError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl actix_web::ResponseError for SqlxError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::InternalServerError().body("")
    }
}

async fn fetch_one_scalar<'q, 'c, O>(
    query: sqlx::query::QueryScalar<'q, sqlx::Sqlite, O, sqlx::sqlite::SqliteArguments<'q>>,
    tx: &mut sqlx::Transaction<'c, sqlx::Sqlite>,
) -> sqlx::Result<O>
where
    O: 'q + Send + Unpin,
    (O,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
{
    let mut r: Vec<O> = query.fetch_all(tx).await?;
    Ok(r.pop().unwrap())
}

#[get("/nest")]
#[auto_span(debug)]
async fn nest(pool: web::Data<sqlx::SqlitePool>) -> actix_web::Result<String> {
    let mut tx = pool.begin().await.map_err(SqlxError)?;
    let r: i32 = fetch_one_scalar(sqlx::query_scalar("SELECT b"), &mut tx)
        .await
        .map_err(SqlxError)?;
    Ok(format!("r={}", r))
}

#[get("/test-if")]
#[auto_span(debug)]
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

#[get("/hello")]
#[auto_span(debug)]
async fn greet(pool: web::Data<sqlx::SqlitePool>) -> actix_web::Result<String> {
    let r: Vec<i32> = sqlx::query_scalar("SELECT id")
        .fetch_all(pool.as_ref())
        .await
        .map_err(SqlxError)?;
    Ok(format!("Hello {:?}!", r))
}

#[get("/req")]
#[auto_span(debug)]
async fn use_reqwest() -> actix_web::Result<String> {
    let target_url = "http://localhost";
    let r = reqwest::Client::new().get(target_url).send().await.unwrap();
    Ok(format!("Reqwest {:?}", r))
}

async fn f() -> i32 {
    42
}

#[get("/req")]
#[auto_span(debug)]
async fn anonymous_await() -> String {
    let r = f().await;
    format!("Reqwest {:?}", r)
}

#[get("/req")]
#[auto_span(debug, all_await)]
async fn anonymous_await2() -> String {
    let r = f().await;
    format!("Reqwest {:?}", r)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let _tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(TRACE_NAME)
        .install_batch(TokioCurrentThread);
    // do not connect for test

    let pool = sqlx::sqlite::SqlitePool::connect(":memory:").await.unwrap();

    let _server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(RequestTracing::new())
            .service(greet)
            .service(test_if)
            .service(nest)
    });
    global::shutdown_tracer_provider();
    Ok(())
}
