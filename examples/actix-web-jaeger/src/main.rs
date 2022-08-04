use actix_web::{
    get, http::StatusCode, web, App, HttpResponse, HttpServer, Responder, ResponseError,
};
use actix_web_opentelemetry::RequestTracing;
use opentelemetry::{
    global, runtime::TokioCurrentThread, sdk::propagation::TraceContextPropagator,
};
use opentelemetry_auto_span::auto_span;
use serde::Serialize;

const TRACE_NAME: &str = "auto-span-sample";

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        #[derive(Debug, Serialize)]
        struct FailureResult {
            message: String,
        }
        HttpResponse::build(self.status_code()).json(FailureResult {
            message: format!("{}", self),
        })
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[auto_span]
#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[derive(sqlx::FromRow, Serialize)]
struct User {
    id: i64,
    name: String,
    language: Option<String>,
}

#[get("/user/{id}")]
#[auto_span(debug)]
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let _tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(TRACE_NAME)
        .with_agent_endpoint("127.0.0.1:6831")
        .install_batch(TokioCurrentThread)
        .expect("pipeline install error");

    let mysql_config = sqlx::mysql::MySqlConnectOptions::new()
        .host("127.0.0.1")
        .username("root")
        .password("actix-otel-auto-span")
        .database("sample")
        .port(3306);
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(10)
        .connect_with(mysql_config)
        .await
        .expect("failed to connect mysql db");

    HttpServer::new(move || {
        App::new()
            .wrap(RequestTracing::new())
            .app_data(web::Data::new(pool.clone()))
            .service(hello)
            .service(get_user)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
