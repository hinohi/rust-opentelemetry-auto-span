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

#[get("/hello/{name}")]
#[auto_span]
async fn greet(
    name: web::Path<String>,
    pool: web::Data<sqlx::MySqlPool>,
) -> actix_web::Result<String> {
    let _r: Vec<i32> = sqlx::query_scalar("SELECT id")
        .fetch_all(pool.as_ref())
        .await
        .map_err(SqlxError)?;
    Ok(format!("Hello {}!", name.into_inner()))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let _tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(TRACE_NAME)
        .install_batch(TokioCurrentThread)
        .expect("pipeline install error");

    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(10)
        .connect("")
        .await
        .expect("failed to connect db");

    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(RequestTracing::new())
            .service(greet)
    });
    server.bind(("0.0.0.0", 3000))?.run().await?;
    global::shutdown_tracer_provider();
    Ok(())
}
