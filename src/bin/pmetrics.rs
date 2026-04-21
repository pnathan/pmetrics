/**
pmetrics entry point
 **/
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use clap::{Parser, Subcommand};
use metrics_exporter_prometheus::PrometheusHandle;
use rand::distributions::{Alphanumeric, DistString};
use std::fs::File;
use std::io;
use std::io::Read;
use std::time::Instant;
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
struct ApiPermissions(Vec<String>);

#[derive(Clone)]
struct AppState {
    pool: deadpool_postgres::Pool,
    prometheus: PrometheusHandle,
}

// API key management types
#[derive(Debug, Deserialize)]
struct CreateKeyRequest {
    label: String,
    permissions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ApiKeyRecord {
    id: i32,
    label: String,
    permissions: Vec<String>,
    created_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
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
    Extension(ApiPermissions(perms)): Extension<ApiPermissions>,
    Json(body): Json<MeasureIngest>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    if !perms.iter().any(|p| p == "tenant_write") {
        return Err((StatusCode::FORBIDDEN, "missing tenant_write permission"));
    }
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
    Extension(ApiPermissions(perms)): Extension<ApiPermissions>,
) -> Result<Json<Vec<Measure>>, (StatusCode, &'static str)> {
    if !perms.iter().any(|p| p == "tenant_read") {
        return Err((StatusCode::FORBIDDEN, "missing tenant_read permission"));
    }
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
    Extension(ApiPermissions(perms)): Extension<ApiPermissions>,
    Json(body): Json<EventIngest>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    if !perms.iter().any(|p| p == "tenant_write") {
        return Err((StatusCode::FORBIDDEN, "missing tenant_write permission"));
    }
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
    Extension(ApiPermissions(perms)): Extension<ApiPermissions>,
) -> Result<Json<Vec<Event>>, (StatusCode, &'static str)> {
    if !perms.iter().any(|p| p == "tenant_read") {
        return Err((StatusCode::FORBIDDEN, "missing tenant_read permission"));
    }
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

async fn prometheus_metrics(State(state): State<AppState>) -> String {
    state.prometheus.render()
}

async fn metrics_middleware(req: axum::extract::Request, next: Next) -> Response {
    let path = req
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|mp| mp.as_str().to_owned())
        .unwrap_or_else(|| req.uri().path().to_owned());
    if path == "/metrics" {
        return next.run(req).await;
    }
    let method = req.method().to_string();
    let start = Instant::now();
    let resp = next.run(req).await;
    let status = resp.status().as_u16().to_string();
    let elapsed = start.elapsed().as_secs_f64();
    metrics::counter!(
        "pmetrics_requests_total",
        "method" => method.clone(),
        "path" => path.clone(),
        "status" => status
    )
    .increment(1);
    metrics::histogram!(
        "pmetrics_request_duration_seconds",
        "method" => method,
        "path" => path
    )
    .record(elapsed);
    resp
}

//////////////////////////////
// Key management handlers

async fn list_keys(
    State(state): State<AppState>,
    Extension(TenantId(tid)): Extension<TenantId>,
    Extension(ApiPermissions(perms)): Extension<ApiPermissions>,
) -> Result<Json<Vec<ApiKeyRecord>>, (StatusCode, &'static str)> {
    if !perms.iter().any(|p| p == "make_api_key") {
        return Err((StatusCode::FORBIDDEN, "missing make_api_key permission"));
    }
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "pool"))?;
    client
        .query(
            "SELECT id, label, permissions, created_at, revoked_at \
             FROM monitoring.api_key WHERE tenant_id = $1 ORDER BY created_at DESC",
            &[&tid],
        )
        .await
        .map_err(|e| {
            tracing::error!(?e, "db list keys");
            (StatusCode::INTERNAL_SERVER_ERROR, "server error")
        })
        .map(|rows| {
            Json(
                rows.iter()
                    .map(|row| ApiKeyRecord {
                        id: row.get(0),
                        label: row.get(1),
                        permissions: row.get(2),
                        created_at: row.get(3),
                        revoked_at: row.get(4),
                    })
                    .collect(),
            )
        })
}

#[derive(Serialize)]
struct CreatedKey {
    key: String,
}

const VALID_PERMISSIONS: &[&str] = &[
    "tenant_read",
    "tenant_write",
    "make_api_key",
    "disable_api_key",
];

async fn create_key(
    State(state): State<AppState>,
    Extension(TenantId(tid)): Extension<TenantId>,
    Extension(ApiPermissions(perms)): Extension<ApiPermissions>,
    Json(body): Json<CreateKeyRequest>,
) -> Result<Json<CreatedKey>, (StatusCode, &'static str)> {
    if !perms.iter().any(|p| p == "make_api_key") {
        return Err((StatusCode::FORBIDDEN, "missing make_api_key permission"));
    }
    for p in &body.permissions {
        if !VALID_PERMISSIONS.contains(&p.as_str()) {
            return Err((StatusCode::UNPROCESSABLE_ENTITY, "unknown permission value"));
        }
    }
    let new_key = format!(
        "a-{}",
        Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
    );
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "pool"))?;
    client
        .execute(
            "INSERT INTO monitoring.api_key (tenant_id, key, label, permissions) \
             VALUES ($1, $2, $3, $4)",
            &[&tid, &new_key, &body.label, &body.permissions],
        )
        .await
        .map_err(|e| {
            tracing::error!(?e, "db create key");
            (StatusCode::BAD_GATEWAY, "server error")
        })?;
    Ok(Json(CreatedKey { key: new_key }))
}

async fn revoke_key(
    State(state): State<AppState>,
    Extension(TenantId(tid)): Extension<TenantId>,
    Extension(ApiPermissions(perms)): Extension<ApiPermissions>,
    Path(key_id): Path<i32>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    if !perms.iter().any(|p| p == "disable_api_key") {
        return Err((StatusCode::FORBIDDEN, "missing disable_api_key permission"));
    }
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "pool"))?;
    let n = client
        .execute(
            "UPDATE monitoring.api_key SET revoked_at = now() \
             WHERE id = $1 AND tenant_id = $2 AND revoked_at IS NULL",
            &[&key_id, &tid],
        )
        .await
        .map_err(|e| {
            tracing::error!(?e, "db revoke key");
            (StatusCode::BAD_GATEWAY, "server error")
        })?;
    if n == 0 {
        Err((StatusCode::NOT_FOUND, "key not found"))
    } else {
        Ok(StatusCode::OK)
    }
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
            "SELECT tenant_id, permissions FROM monitoring.api_key \
             WHERE key = $1 AND revoked_at IS NULL",
            &[&key],
        )
        .await
        .map_err(|e| {
            tracing::error!(?e, "db auth lookup");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::FORBIDDEN)?;

    let tid: i32 = row.get(0);
    let perms: Vec<String> = row.get(1);
    req.extensions_mut().insert(TenantId(tid));
    req.extensions_mut().insert(ApiPermissions(perms));
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
        .route("/keys", get(list_keys).post(create_key))
        .route("/keys/:id", delete(revoke_key))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            check_api_keys,
        ));

    Router::new()
        .route("/", get(root))
        .route("/healthz", get(healthz))
        .route("/metrics", get(prometheus_metrics))
        .nest("/api/v1", api)
        .merge(SwaggerUi::new("/api-docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(middleware::from_fn(metrics_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn launch_server(server_options: &ServerOptions) {
    tracing::info!("server initializing");

    let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
    let prometheus = recorder.handle();
    if let Err(e) = metrics::set_global_recorder(recorder) {
        tracing::warn!("metrics recorder already set: {e}");
    }

    let state = AppState {
        pool: db::build_pool(),
        prometheus,
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

    let row = client
        .query_opt(
            "SELECT tenant_id, permissions FROM monitoring.api_key \
             WHERE key = $1 AND revoked_at IS NULL",
            &[&apikey],
        )
        .await
        .expect("api key lookup")
        .unwrap_or_else(|| {
            tracing::info!("api key failure {}", &apikey);
            panic!("api key didn't work");
        });
    let tid: i32 = row.get(0);
    let perms: Vec<String> = row.get(1);
    if !perms.iter().any(|p| p == "tenant_write") {
        tracing::error!("api key {} lacks tenant_write permission", &apikey);
        panic!("api key lacks tenant_write permission");
    }

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

    fn test_state() -> AppState {
        let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
        let prometheus = recorder.handle();
        AppState { pool: test_pool(), prometheus }
    }

    async fn seed_tenant(client: &deadpool_postgres::Client) -> (i32, String) {
        let key = format!("test-{}", uuid::Uuid::new_v4());
        client
            .execute(
                "INSERT INTO monitoring.tenant (tenantname) VALUES ('integration-test')",
                &[],
            )
            .await
            .expect("seed tenant");
        let row = client
            .query_one(
                "SELECT uid FROM monitoring.tenant WHERE tenantname = 'integration-test' ORDER BY uid DESC LIMIT 1",
                &[],
            )
            .await
            .expect("get seeded tenant");
        let tid: i32 = row.get(0);
        client
            .execute(
                "INSERT INTO monitoring.api_key (tenant_id, key, label, permissions) \
                 VALUES ($1, $2, 'test-key', '{tenant_read,tenant_write,make_api_key,disable_api_key}')",
                &[&tid, &key],
            )
            .await
            .expect("seed api key");
        (tid, key)
    }

    async fn cleanup_tenant(client: &deadpool_postgres::Client, tid: i32) {
        client
            .execute("DELETE FROM monitoring.api_key WHERE tenant_id = $1", &[&tid])
            .await
            .ok();
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
        let state = test_state();
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
        let state = test_state();
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
        let state = test_state();
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

        let state = test_state();
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

        let state = test_state();
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
