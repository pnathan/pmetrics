/**
pmetrics entry point
 **/
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io;
use std::io::Read;
use tower_http::trace::TraceLayer;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

use chrono::prelude::{DateTime, Utc};

use pmetrics::db;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
struct MeasureIngest {
    name: String,
    measurement: f64,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct IntrusiveMeasure {
    insertion_time: DateTime<Utc>,
    name: String,
    measurement: f64,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct Measure {
    d: MeasureIngest,
    insertion_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
struct EventIngest {
    name: String,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct IntrusiveEvent {
    insertion_time: DateTime<Utc>,
    name: String,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct Event {
    d: EventIngest,
    insertion_time: DateTime<Utc>,
}

#[derive(Clone, Copy)]
struct TenantId(i32);

#[derive(Clone)]
struct AppState {
    pool: deadpool_postgres::Pool,
}

// TODO: Api should have an accept: application/json request type, along with appropriate error codes.
// Said error codes will be:
//
// 400 YOU SEND ME A BAD THING, CANT UNDERSTAND IT. MAYBE BETTER JSON NEXT TIME?
// 422 GOOD REQUEST SYNTAX, BUT DIDN'T MAKE SENSE. WATFACE
// 500 BARF
// 502 COULDN'T TALK TO DATABASE.

// Technicall 201 should be the typical response for a POST here.

// In semantic, the GETs for the API are essentially tailing the stream.

// TODO: add middleware specifying the specifics of the ip connection

// TODO: Write a Search api.

async fn writemeasure(
    client: &deadpool_postgres::Client,
    tid: i32,
    l: &MeasureIngest,
) -> Result<u64, tokio_postgres::Error> {
    client
        .execute(
            "INSERT INTO monitoring.measure (name, tenant_id, measurement, dict) \
             VALUES ($1, $2, $3, $4)",
            &[&l.name, &tid, &l.measurement, &l.dict],
        )
        .await
}

#[utoipa::path(
    post,
    path = "/api/v1/measure",
    request_body = MeasureIngest,
    responses(
        (status = 200, description = "Measurement recorded"),
        (status = 403, description = "Forbidden"),
        (status = 502, description = "Database error"),
    ),
    security(("ApiKeyAuth" = []))
)]
async fn post_measure(
    State(state): State<AppState>,
    Extension(TenantId(tid)): Extension<TenantId>,
    Json(body): Json<MeasureIngest>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "pool"))?;
    writemeasure(&client, tid, &body)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|e| {
            tracing::error!(?e, "db insert measure");
            (StatusCode::BAD_GATEWAY, "server error")
        })
}

#[utoipa::path(
    get,
    path = "/api/v1/measure",
    responses(
        (status = 200, description = "List of recent measurements", body = Vec<Measure>),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error"),
    ),
    security(("ApiKeyAuth" = []))
)]
async fn get_measure(
    State(state): State<AppState>,
    Extension(TenantId(tid)): Extension<TenantId>,
) -> Result<Json<Vec<Measure>>, (StatusCode, &'static str)> {
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "pool"))?;
    client
        .query(
            "SELECT insertion_time, name, measurement, dict from monitoring.measure \
             where tenant_id = $1 order by insertion_time desc limit 100",
            &[&tid],
        )
        .await
        .map_err(|e| {
            tracing::error!(?e, "db query measures");
            (StatusCode::INTERNAL_SERVER_ERROR, "server error")
        })
        .map(|rows| {
            Json(
                rows.iter()
                    .map(|row| Measure {
                        insertion_time: row.get(0),
                        d: MeasureIngest {
                            name: row.get(1),
                            measurement: row.get(2),
                            dict: row.get(3),
                        },
                    })
                    .collect(),
            )
        })
}

async fn writeevent(
    client: &deadpool_postgres::Client,
    tid: i32,
    l: &EventIngest,
) -> Result<u64, tokio_postgres::Error> {
    client
        .execute(
            "INSERT INTO monitoring.event (name, tenant_id, dict) VALUES ($1, $2, $3)",
            &[&l.name, &tid, &l.dict],
        )
        .await
}

#[utoipa::path(
    post,
    path = "/api/v1/event",
    request_body = EventIngest,
    responses(
        (status = 200, description = "Event recorded"),
        (status = 403, description = "Forbidden"),
        (status = 502, description = "Database error"),
    ),
    security(("ApiKeyAuth" = []))
)]
async fn post_event(
    State(state): State<AppState>,
    Extension(TenantId(tid)): Extension<TenantId>,
    Json(body): Json<EventIngest>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "pool"))?;
    writeevent(&client, tid, &body)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|e| {
            tracing::error!(?e, "db insert event");
            (StatusCode::BAD_GATEWAY, "server error")
        })
}

#[utoipa::path(
    get,
    path = "/api/v1/event",
    responses(
        (status = 200, description = "List of recent events", body = Vec<Event>),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error"),
    ),
    security(("ApiKeyAuth" = []))
)]
async fn get_event(
    State(state): State<AppState>,
    Extension(TenantId(tid)): Extension<TenantId>,
) -> Result<Json<Vec<Event>>, (StatusCode, &'static str)> {
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "pool"))?;
    client
        .query(
            "SELECT insertion_time, name, dict from monitoring.event \
             where tenant_id = $1 order by insertion_time desc limit 100",
            &[&tid],
        )
        .await
        .map_err(|e| {
            tracing::error!(?e, "db query events");
            (StatusCode::INTERNAL_SERVER_ERROR, "server error")
        })
        .map(|rows| {
            Json(
                rows.iter()
                    .map(|row| Event {
                        insertion_time: row.get(0),
                        d: EventIngest {
                            name: row.get(1),
                            dict: row.get(2),
                        },
                    })
                    .collect(),
            )
        })
}

#[utoipa::path(
    get,
    path = "/",
    responses((status = 200, description = "Welcome message"))
)]
async fn root() -> &'static str {
    "welcome to pmetrics"
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses((status = 200, description = "Service is healthy"))
)]
async fn healthz() -> &'static str {
    tracing::info!("healthz");
    "ok"
}

//////////////////////////////
// API KEY / tenant middleware.

async fn check_api_keys(
    State(state): State<AppState>,
    mut req: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let key = req
        .headers()
        .get("X-PMETRICS-API-KEY")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?
        .to_string();

    let client = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt(
            "SELECT uid FROM monitoring.tenant WHERE apikey = $1",
            &[&key],
        )
        .await
        .map_err(|e| {
            tracing::error!(?e, "db auth lookup");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::FORBIDDEN)?;

    let tid: i32 = row.get(0);
    req.extensions_mut().insert(TenantId(tid));
    Ok(next.run(req).await)
}

//////////////////////////////
// OpenAPI

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "ApiKeyAuth",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-PMETRICS-API-KEY"))),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(root, healthz, post_event, get_event, post_measure, get_measure),
    components(schemas(EventIngest, Event, MeasureIngest, Measure, IntrusiveEvent, IntrusiveMeasure)),
    modifiers(&SecurityAddon),
    info(title = "pmetrics API", version = "1.0.2", description = "Metrics and event tracking")
)]
struct ApiDoc;

//////////////////////////////
// Router

fn build_app(state: AppState) -> Router {
    let api = Router::new()
        .route("/event", post(post_event).get(get_event))
        .route("/measure", post(post_measure).get(get_measure))
        .route("/ingest", post(post_ingest))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            check_api_keys,
        ));

    Router::new()
        .route("/", get(root))
        .route("/healthz", get(healthz))
        .nest("/api/v1", api)
        .merge(SwaggerUi::new("/api-docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn launch_server(server_options: &ServerOptions) {
    tracing::info!("server initializing");

    let state = AppState {
        pool: db::build_pool(),
    };

    let app = build_app(state);

    let addr: std::net::SocketAddr = ([0, 0, 0, 0], server_options.port).into();
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    tracing::info!(%addr, "listening");
    axum::serve(listener, app).await.expect("serve");
}

async fn launch_query(qo: &QueryOptions) {
    let pool = db::build_pool();
    let client = pool.get().await.expect("pool get for query");
    println!("{qo:?}");
    let last: i64 = qo.last.into();
    let printable = match qo.metric_type {
        MetricTypeOption::E => {
            let query = "SELECT insertion_time, name, dict from monitoring.event \
                         order by insertion_time desc limit $1";
            match client.query(query, &[&last]).await {
                Ok(rows) => {
                    let vec: Vec<IntrusiveEvent> = rows
                        .iter()
                        .map(|row| IntrusiveEvent {
                            insertion_time: row.get(0),
                            name: row.get(1),
                            dict: row.get(2),
                        })
                        .collect();
                    match serde_json::to_string_pretty(&vec) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("error=true module=web error={e:?} class=json-render");
                            return;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("error=true module=db error={e:?} query={query}");
                    return;
                }
            }
        }
        MetricTypeOption::M => {
            let query = "SELECT insertion_time, name, measurement, dict \
                         from monitoring.measure order by insertion_time desc limit $1";
            match client.query(query, &[&last]).await {
                Ok(rows) => {
                    let vec: Vec<IntrusiveMeasure> = rows
                        .iter()
                        .map(|row| IntrusiveMeasure {
                            insertion_time: row.get(0),
                            name: row.get(1),
                            measurement: row.get(2),
                            dict: row.get(3),
                        })
                        .collect();
                    match serde_json::to_string_pretty(&vec) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("error=true module=web error={e:?} class=json-render");
                            return;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("error=true module=db error={e:?} query={query}");
                    return;
                }
            }
        }
    };

    println!("{printable}");
}

#[derive(Debug, Serialize, Deserialize)]
enum PipeReader {
    M(MeasureIngest),
    E(EventIngest),
}

async fn post_ingest(
    State(state): State<AppState>,
    Extension(TenantId(tid)): Extension<TenantId>,
    Json(items): Json<Vec<PipeReader>>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    if items.is_empty() {
        return Ok(StatusCode::OK);
    }
    let mut client = state
        .pool
        .get()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "pool"))?;
    let txn = client.transaction().await.map_err(|e| {
        tracing::error!(?e, "db begin transaction");
        (StatusCode::BAD_GATEWAY, "server error")
    })?;
    for (idx, item) in items.iter().enumerate() {
        match item {
            PipeReader::M(m) => txn
                .execute(
                    "INSERT INTO monitoring.measure (name, tenant_id, measurement, dict) \
                     VALUES ($1, $2, $3, $4)",
                    &[&m.name, &tid, &m.measurement, &m.dict],
                )
                .await
                .map_err(|e| {
                    tracing::error!(?e, idx, "bulk insert measure");
                    (StatusCode::BAD_GATEWAY, "db error")
                })?,
            PipeReader::E(ev) => txn
                .execute(
                    "INSERT INTO monitoring.event (name, tenant_id, dict) VALUES ($1, $2, $3)",
                    &[&ev.name, &tid, &ev.dict],
                )
                .await
                .map_err(|e| {
                    tracing::error!(?e, idx, "bulk insert event");
                    (StatusCode::BAD_GATEWAY, "db error")
                })?,
        };
    }
    txn.commit().await.map_err(|e| {
        tracing::error!(?e, "db commit transaction");
        (StatusCode::BAD_GATEWAY, "server error")
    })?;
    Ok(StatusCode::OK)
}

async fn launch_writer(filename: String, apikey: String) {
    let pool = db::build_pool();
    let client = pool.get().await.expect("pool get for writer");

    let mut file: Box<dyn Read> = match filename.as_str() {
        "-" => Box::new(io::stdin()),
        _ => {
            tracing::info!("opening {}", &filename);
            match File::open(filename) {
                Ok(f) => Box::new(f),
                Err(e) => {
                    tracing::error!("error=true module=fs error={e:?} class=file-open");
                    panic!("could not open file");
                }
            }
        }
    };

    let tid: i32 = client
        .query_opt(
            "SELECT uid FROM monitoring.tenant WHERE apikey = $1",
            &[&apikey],
        )
        .await
        .expect("api key lookup")
        .map(|row| row.get(0))
        .unwrap_or_else(|| {
            tracing::info!("api key failure {}", &apikey);
            panic!("api key didn't work");
        });

    loop {
        let mut buffer = String::new();

        // this frankly should be epoll based for a named pipe, but
        // let's let it live for now. If this is a _useful_ system, we
        // can do more with it.
        let result = file.read_to_string(&mut buffer);
        match result {
            Ok(bytecount) => {
                if bytecount > 0 {
                    tracing::info!("status=rx");
                    let v: Result<Vec<PipeReader>, serde_json::Error> =
                        serde_json::from_str(&buffer);

                    match v {
                        Ok(dataz) => {
                            for row in &dataz {
                                match row {
                                    PipeReader::M(measure) => {
                                        if let Err(e) = writemeasure(&client, tid, measure).await {
                                            tracing::error!("error=true module=db error={e:?} class=measure-write");
                                        }
                                    }
                                    PipeReader::E(event) => {
                                        if let Err(e) = writeevent(&client, tid, event).await {
                                            tracing::error!("error=true module=db error={e:?} class=event-write");
                                        }
                                    }
                                }
                                tracing::info!("status=written");
                            }
                        }
                        Err(e) => {
                            tracing::error!("err={e:?}");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::info!("err={e:?}");
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

#[derive(Debug)]
enum ServerType {
    Http,
}

#[derive(Debug)]
struct CliOptions {
    filename: String,
    apikey: String,
}

#[derive(Debug)]
struct ServerOptions {
    port: u16,
}

#[derive(Debug)]
enum MetricTypeOption {
    M,
    E,
}

#[derive(Debug)]
struct QueryOptions {
    metric_type: MetricTypeOption,
    last: u16,
}

#[derive(Debug)]
enum Command {
    PipeReader(CliOptions),
    Server(ServerOptions, ServerType),
    Querier(QueryOptions),
}

#[derive(Debug, Subcommand)]
enum PmetricsMode {
    Pipe {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        api_key: String,
    },
    Server {
        #[arg(short, long)]
        server_type: String,
        #[arg(short, long)]
        port: u16,
    },
    Querier {
        metric_type: String,
        last: u16,
    },
}

#[derive(Debug, Parser)]
#[command(name = "pmetrics")]
#[command(version = "1.0.1")]
#[command(about = "an observability system", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long, default_value = "1", help = "verbosity level")]
    v: u8,
    #[command(subcommand)]
    cmd: PmetricsMode,
}

fn clapparser() -> (Command, u8) {
    let cli = Cli::parse();

    let cmd = match cli.cmd {
        PmetricsMode::Server { server_type, port } => {
            let st = match server_type.as_str() {
                "http" => ServerType::Http,
                _ => panic!("Unable to start server, crashing. specify type http"),
            };

            let so = ServerOptions { port };

            Command::Server(so, st)
        }
        PmetricsMode::Pipe { file, api_key } => Command::PipeReader(CliOptions {
            filename: file,
            apikey: api_key,
        }),
        PmetricsMode::Querier {
            metric_type, last, ..
        } => {
            let mt = match metric_type.as_str() {
                "m" => MetricTypeOption::M,
                "e" => MetricTypeOption::E,
                _ => panic!("not a valid metric type - try m or e"),
            };

            let qo = QueryOptions {
                metric_type: mt,
                last,
            };
            Command::Querier(qo)
        }
    };
    (cmd, cli.v)
}

#[tokio::main]
async fn main() {
    let (cmd, verbosity) = clapparser();

    let level = match verbosity {
        0 => "error",
        1 => "info",
        _ => "debug",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level)),
        )
        .init();

    match cmd {
        Command::Server(server_options, _st) => launch_server(&server_options).await,
        Command::PipeReader(clioptions) => {
            launch_writer(clioptions.filename, clioptions.apikey).await
        }
        Command::Querier(qo) => launch_query(&qo).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_pool() -> deadpool_postgres::Pool {
        db::build_pool()
    }

    async fn seed_tenant(client: &deadpool_postgres::Client) -> (i32, String) {
        let key = format!("test-{}", uuid::Uuid::new_v4());
        client
            .execute(
                "INSERT INTO monitoring.tenant (tenantname, apikey) VALUES ('integration-test', $1)",
                &[&key],
            )
            .await
            .expect("seed tenant");
        let row = client
            .query_one(
                "SELECT uid FROM monitoring.tenant WHERE apikey = $1",
                &[&key],
            )
            .await
            .expect("get seeded tenant");
        (row.get(0), key)
    }

    async fn cleanup_tenant(client: &deadpool_postgres::Client, tid: i32) {
        client
            .execute(
                "DELETE FROM monitoring.measure WHERE tenant_id = $1",
                &[&tid],
            )
            .await
            .ok();
        client
            .execute("DELETE FROM monitoring.event WHERE tenant_id = $1", &[&tid])
            .await
            .ok();
        client
            .execute("DELETE FROM monitoring.tenant WHERE uid = $1", &[&tid])
            .await
            .ok();
    }

    #[tokio::test]
    async fn healthz_returns_ok() {
        let state = AppState { pool: test_pool() };
        let app = build_app(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn post_event_without_key_returns_403() {
        let state = AppState { pool: test_pool() };
        let app = build_app(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/event")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"test","dict":{}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn invalid_api_key_returns_403() {
        let state = AppState { pool: test_pool() };
        let app = build_app(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/event")
                    .header("X-PMETRICS-API-KEY", "not-a-real-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn post_and_get_event_roundtrip() {
        let pool = test_pool();
        let client = pool.get().await.expect("pool");
        let (tid, key) = seed_tenant(&client).await;

        let state = AppState { pool: test_pool() };
        let app = build_app(state);

        let post_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/event")
                    .header("content-type", "application/json")
                    .header("X-PMETRICS-API-KEY", &key)
                    .body(Body::from(r#"{"name":"roundtrip-event","dict":{"k":"v"}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(post_resp.status(), StatusCode::OK);

        let get_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/event")
                    .header("X-PMETRICS-API-KEY", &key)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);
        let body = get_resp.into_body().collect().await.unwrap().to_bytes();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(text.contains("roundtrip-event"));

        cleanup_tenant(&client, tid).await;
    }

    #[tokio::test]
    async fn post_and_get_measure_roundtrip() {
        let pool = test_pool();
        let client = pool.get().await.expect("pool");
        let (tid, key) = seed_tenant(&client).await;

        let state = AppState { pool: test_pool() };
        let app = build_app(state);

        let post_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/measure")
                    .header("content-type", "application/json")
                    .header("X-PMETRICS-API-KEY", &key)
                    .body(Body::from(
                        r#"{"name":"roundtrip-measure","measurement":42.5,"dict":{}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(post_resp.status(), StatusCode::OK);

        let get_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/measure")
                    .header("X-PMETRICS-API-KEY", &key)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);
        let body = get_resp.into_body().collect().await.unwrap().to_bytes();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(text.contains("roundtrip-measure"));

        cleanup_tenant(&client, tid).await;
    }
}
