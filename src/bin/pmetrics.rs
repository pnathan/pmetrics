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
use rand::distributions::{Alphanumeric, DistString};
use std::fs::File;
use std::io;
use std::io::Read;
use tower_http::trace::TraceLayer;

use chrono::prelude::{DateTime, Utc};

use pmetrics::db;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MeasureIngest {
    name: String,
    measurement: f64,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct IntrusiveMeasure {
    insertion_time: DateTime<Utc>,
    name: String,
    measurement: f64,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct Measure {
    d: MeasureIngest,
    insertion_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct EventIngest {
    name: String,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct IntrusiveEvent {
    insertion_time: DateTime<Utc>,
    name: String,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize)]
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

async fn root() -> &'static str {
    "welcome to pmetrics"
}

async fn healthz() -> &'static str {
    tracing::info!("healthz");
    "ok"
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

async fn create_key(
    State(state): State<AppState>,
    Extension(TenantId(tid)): Extension<TenantId>,
    Extension(ApiPermissions(perms)): Extension<ApiPermissions>,
    Json(body): Json<CreateKeyRequest>,
) -> Result<Json<CreatedKey>, (StatusCode, &'static str)> {
    if !perms.iter().any(|p| p == "make_api_key") {
        return Err((StatusCode::FORBIDDEN, "missing make_api_key permission"));
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

async fn launch_server(server_options: &ServerOptions) {
    tracing::info!("server initializing");

    let state = AppState {
        pool: db::build_pool(),
    };

    let api = Router::new()
        .route("/event", post(post_event).get(get_event))
        .route("/measure", post(post_measure).get(get_measure))
        .route("/keys", get(list_keys).post(create_key))
        .route("/keys/:id", delete(revoke_key))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            check_api_keys,
        ));

    let app = Router::new()
        .route("/", get(root))
        .route("/healthz", get(healthz))
        .nest("/api/v1", api)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

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
            "SELECT tenant_id FROM monitoring.api_key WHERE key = $1 AND revoked_at IS NULL",
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
