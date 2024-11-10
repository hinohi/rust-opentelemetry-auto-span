use axum::{
    extract::{Json, Path, State},
    routing::get,
    Router,
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
                "auto-span-axum-example",
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

    let app = Router::new()
        .route("/", get(hello))
        .route("/user/:id", get(get_user))
        .with_state(pool);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Serialize)]
struct ErrorResponseBody {
    message: String,
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            Error::Sqlx(e) => match e {
                sqlx::Error::RowNotFound => axum::http::StatusCode::NOT_FOUND,
                _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            },
        };
        (
            status,
            Json(ErrorResponseBody {
                message: self.to_string(),
            }),
        )
            .into_response()
    }
}

#[auto_span]
async fn hello() -> &'static str {
    "Hello, World!"
}

#[derive(sqlx::FromRow, Serialize)]
struct User {
    id: i64,
    name: String,
    language: Option<String>,
}

#[auto_span]
async fn get_user(
    Path(id): Path<i64>,
    State(db): State<sqlx::MySqlPool>,
) -> Result<Json<User>, Error> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_one(&db)
        .await?;
    Ok(Json(user))
}
