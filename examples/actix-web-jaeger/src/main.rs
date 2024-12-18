use actix_web::{
    get, http::StatusCode, web, App, HttpResponse, HttpServer, Responder, ResponseError,
};
use opentelemetry_auto_span::auto_span;
use serde::Serialize;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
    use opentelemetry_semantic_conventions::resource::SERVICE_NAME;

    // init tracer: also see https://github.com/open-telemetry/opentelemetry-rust/tree/main/examples/tracing-jaeger
    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(
            sdktrace::Config::default().with_resource(Resource::new(vec![KeyValue::new(
                SERVICE_NAME,
                "auto-span-actix-web-example",
            )])),
        )
        .install_batch(runtime::Tokio)
        .expect("pipeline install error");
    opentelemetry::global::set_tracer_provider(tracer_provider);

    let mysql_config = sqlx::mysql::MySqlConnectOptions::new()
        .host("127.0.0.1")
        .username("root")
        .password("auto-span-example")
        .database("sample")
        .port(3306);
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(10)
        .connect_with(mysql_config)
        .await
        .expect("failed to connect mysql db");

    HttpServer::new(move || {
        App::new()
            .wrap(actix_web_opentelemetry::RequestTracing::new())
            .app_data(web::Data::new(pool.clone()))
            .service(hello)
            .service(get_user)
            .service(use_awc)
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("awc error: {0}")]
    AwcSendRequest(#[from] awc::error::SendRequestError),
    #[error("awc error: {0}")]
    AwcPayload(#[from] awc::error::PayloadError),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        #[derive(Debug, Serialize)]
        struct FailureResult {
            message: String,
        }
        HttpResponse::build(self.status_code()).json(FailureResult {
            message: format!("{}", self),
        })
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

#[get("/awc")]
#[auto_span]
async fn use_awc() -> actix_web::Result<HttpResponse, Error> {
    let client = awc::Client::default();
    let req = client.get("http://localhost:8081");
    let mut res = req.send().await?;
    Ok(HttpResponse::Ok().body(res.body().await?))
}
