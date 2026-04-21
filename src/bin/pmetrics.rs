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
use metrics_exporter_prometheus::PrometheusHandle;
use std::fs::File;
use std::io;
use std::io::Read;
use std::time::Instant;
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
struct AppState {
    pool: deadpool_postgres::Pool,
    prometheus: PrometheusHandle,
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

async fn root() -> &'static str {
    "welcome to pmetrics"
}

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

    let api = Router::new()
        .route("/event", post(post_event).get(get_event))
        .route("/measure", post(post_measure).get(get_measure))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            check_api_keys,
        ));

    let app = Router::new()
        .route("/", get(root))
        .route("/healthz", get(healthz))
        .route("/metrics", get(prometheus_metrics))
        .nest("/api/v1", api)
        .layer(middleware::from_fn(metrics_middleware))
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
