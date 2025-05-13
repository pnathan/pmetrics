/**
pmetrics entry point
 **/
#[macro_use]
extern crate nickel;

use clap::{Parser, Subcommand};
use std::fs::File;
use std::io;
use std::io::Read;
use std::{thread, time};

use chrono::prelude::{DateTime, Utc};

use nickel::status::StatusCode;
use nickel::{/* QueryString, */ HttpRouter, MiddlewareResult, Nickel, Request, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
// this is /not obvious/.
use pmetrics::audit;
use pmetrics::db;
use serde_json::Value;
use std::mem;
use std::sync::{Arc, Mutex, Once};

#[derive(Clone)]
struct SingletonReader {
    inner: Arc<Mutex<audit::Audit>>,
}

impl SingletonReader {
    fn get(&self) -> audit::Audit {
        *self.inner.lock().unwrap()
    }
}

fn recorder() -> SingletonReader {
    // !! null pointer.
    static mut SINGLETON: *const SingletonReader = 0 as *const SingletonReader;
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            // Make it
            let singleton = SingletonReader {
                inner: Arc::new(Mutex::new(audit::Audit {
                    level: audit::ConcernLevel::Crisis,
                    t: audit::AuditTarget::Stderr(),
                })),
            };

            // Put it in the heap so it can outlive this call
            SINGLETON = mem::transmute(Box::new(singleton));
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        (*SINGLETON).clone()
    }
}

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

fn generic_post<'a, T, F>(
    req: &'a mut Request,
    insert_function: F,
) -> (nickel::status::StatusCode, String)
where
    T: DeserializeOwned,
    F: Fn(&mut postgres::Client, &T) -> Result<u64, postgres::Error>,
{
    let mut buffer = String::new();

    match req.origin.read_to_string(&mut buffer) {
        Ok(_) => {} // no-op
        Err(_) => {
            recorder().get().info(audit::eventw(&[
                "error",
                "true",
                "module",
                "web",
                "class",
                "string read",
            ]));
            return (StatusCode::BadRequest, "unable to read string".to_string());
        }
    }

    let v: Result<T, serde_json::Error> = serde_json::from_str(&buffer);
    match v {
        Ok(deserialized) => {
            let mut conn = db::connect_to_db(&recorder().get());
            match insert_function(&mut conn, &deserialized) {
                Ok(_) => (StatusCode::Ok, "ok".to_string()),
                Err(err) => {
                    recorder().get().debug(audit::eventw(&[
                        "error",
                        "true",
                        "module",
                        "web",
                        "class",
                        "db insert",
                        "details",
                        &err.to_string(),
                    ]));
                    recorder().get().info(audit::eventw(&[
                        "error",
                        "true",
                        "module",
                        "web",
                        "class",
                        "db insert",
                    ]));
                    (StatusCode::BadGateway, "server error".to_string())
                }
            }
        }

        Err(_) => {
            recorder().get().info(audit::eventw(&[
                "error",
                "true",
                "module",
                "web",
                "class",
                "deserialize/parse",
            ]));
            (StatusCode::BadRequest, "bad parse and cast".to_string())
        }
    }
}

fn writemeasure(
    conn: &mut postgres::Client,
    tid: i32,
    l: &MeasureIngest,
) -> Result<u64, postgres::Error> {
    conn.execute("INSERT INTO monitoring.measure (name, tenant_id, measurement, dict) VALUES ($1, $2, $3, $4)",
                 &[&l.name, &tid, &l.measurement, &l.dict])
}

fn postmeasure(req: &mut Request) -> (nickel::status::StatusCode, String) {
    match get_tid(req) {
        Some(tid) => {
            let f = |conn: &mut postgres::Client,
                     l: &MeasureIngest|
             -> Result<u64, postgres::Error> { writemeasure(conn, tid, l) };

            generic_post(req, f)
        }
        None => {
            recorder().get().info(audit::eventw(&[
                "error",
                "true",
                "module",
                "web",
                "what",
                "failed to get the x tenant id from the middleware",
            ]));
            (StatusCode::BadRequest, "\"key failure\"".to_string())
        }
    }
}

// TODO: dry up.
fn getmeasure(req: &mut Request) -> (nickel::status::StatusCode, String) {
    let mut conn = db::connect_to_db(&recorder().get());
    let tid = get_tid(req).unwrap();

    let query = "SELECT insertion_time, name, measurement, dict from monitoring.measure where tenant_id = $1 order by insertion_time desc limit 100";
    match conn.query(query, &[&tid]) {
        Ok(rows) => {
            let mut vec: Vec<Measure> = Vec::new();
            for row in &rows {
                vec.push(Measure {
                    insertion_time: row.get(0),
                    d: MeasureIngest {
                        name: row.get(1),
                        measurement: row.get(2),
                        dict: row.get(3),
                    },
                });
            }
            let result = serde_json::to_string(&vec).unwrap();
            (StatusCode::Ok, result)
        }
        Err(e) => {
            recorder().get().crisis(audit::eventw(&[
                "error",
                "true",
                "module",
                "db",
                "error",
                e.to_string().as_str(),
                "query",
                &query,
            ]));
            (
                StatusCode::InternalServerError,
                "server error, can't get data".to_string(),
            )
        }
    }
}

fn writeevent(
    conn: &mut postgres::Client,
    tid: i32,
    l: &EventIngest,
) -> Result<u64, postgres::Error> {
    conn.execute(
        "INSERT INTO monitoring.event (name, tenant_id, dict) VALUES ($1, $2, $3)",
        &[&l.name, &tid, &l.dict],
    )
}

fn postevent(req: &mut Request) -> (nickel::status::StatusCode, String) {
    match get_tid(req) {
        Some(tid) => {
            let f = |conn: &mut postgres::Client,
                     l: &EventIngest|
             -> Result<u64, postgres::Error> { writeevent(conn, tid, l) };

            generic_post(req, f)
        }
        None => {
            recorder().get().info(audit::eventw(&[
                "error",
                "true",
                "module",
                "web",
                "what",
                "failed to get the x tenant id from the middleware",
            ]));
            (StatusCode::BadRequest, "\"key failure\"".to_string())
        }
    }
}

fn getevent(req: &mut Request) -> (nickel::status::StatusCode, String) {
    let mut conn = db::connect_to_db(&recorder().get());
    let mut vec: Vec<Event> = Vec::new();
    let tid = get_tid(req).unwrap();

    for row in &conn.query("SELECT insertion_time, name, dict from monitoring.event where tenant_id = $1 order by insertion_time desc limit 100",
                           &[&tid]).unwrap() {
        vec.push(Event {
            insertion_time: row.get(0),
            d: EventIngest {
                name: row.get(1),
                dict: row.get(2),
            },
        });
    }

    let result = serde_json::to_string(&vec).unwrap();
    (StatusCode::Ok, result)
}

// index /
fn handler(_req: &mut Request) -> (nickel::status::StatusCode, String) {
    (StatusCode::Ok, "welcome to nickel'd pmetrics".to_string())
}

// healthz - am I alive?
// does not check database liveness though.
fn healthz(_req: &mut Request) -> (nickel::status::StatusCode, String) {
    recorder().get().info(audit::event("healthz", "true"));
    (StatusCode::Ok, "ok".to_string())
}

//////////////////////////////
// API KEY / tenant middleware.

fn get_tid(req: &Request) -> Option<i32> {
    match req.origin.headers.get_raw("X-TENANT-ID") {
        Some(s) => {
            let thread_string = (&s[0]).to_vec();
            let tid: i32 = String::from_utf8(thread_string).unwrap().parse().unwrap();
            return Some(tid);
        }
        None => {
            recorder().get().info(audit::eventw(&[
                "error",
                "true",
                "module",
                "web",
                "what",
                "failed to get the x tenant id from the middleware",
            ]));
            None
        }
    }
}

struct ApiKeys;

impl ApiKeys {
    fn check_keys(&self, k: &str) -> Option<i32> {
        // reads monitoring.apikeys
        let mut conn = db::connect_to_db(&recorder().get());
        let mut vec: Vec<i32> = Vec::new();

        for row in &conn
            .query(
                "SELECT uid from monitoring.tenant where apikey = $1",
                &[&k.to_string()],
            )
            .unwrap()
        {
            vec.push(row.get(0));
        }

        if vec.len() > 0 {
            return Some(vec[0].clone());
        } else {
            return None;
        }
    }
}

fn check_api_keys<'mw>(_req: &mut Request, mut res: Response<'mw>) -> MiddlewareResult<'mw> {
    let path = _req.path_without_query().unwrap();
    // Cutout for non-api routes.
    if !path.contains("api") {
        return res.next_middleware();
    }

    match _req.origin.headers.get_raw("X-PMETRICS-API-KEY") {
        Some(s) => {
            let gatekeeper = ApiKeys {};
            let header = &s[0];
            let key: String = String::from_utf8(header.to_vec()).unwrap();
            let apikeys = gatekeeper.check_keys(&key);
            match apikeys {
                Some(v) => {
                    // Set it for other users lower down in the middleware stack.
                    _req.origin
                        .headers
                        .set_raw("X-TENANT-ID", vec![v.to_string().into_bytes()]);
                }
                None => {
                    res.set(StatusCode::Forbidden);

                    return res.send("\"api key failure\"");
                }
            }
        }
        None => {
            res.set(StatusCode::Forbidden);

            return res.send("\"api key failure\"");
        }
    }

    // Pass control to the next middleware
    res.next_middleware()
}

fn info(pairs: &[&str]) -> () {
    recorder().get().info(audit::eventw(pairs));
}

fn log_request<'mw>(_req: &mut Request, res: Response<'mw>) -> MiddlewareResult<'mw> {
    match _req.origin.headers.get_raw("X-PMETRICS-API-KEY") {
        Some(key) => {
            let header = &key[0];
            let key: String = String::from_utf8(header.to_vec()).unwrap();
            info(&[
                "module",
                "web",
                "method",
                &_req.origin.method.to_string(),
                "url",
                _req.path_without_query().unwrap(),
                //                   "code", &res.status().to_string(),
                "apikey",
                &key,
            ]);
        }
        None => {
            info(&[
                "module",
                "web",
                "method",
                &_req.origin.method.to_string(),
                "url",
                _req.path_without_query().unwrap(),
                //                   "code", &res.status().to_string()
            ])
        }
    }
    res.next_middleware()
}

fn launch_server(cl: &audit::ConcernLevel, server_options: &ServerOptions) {
    //    *Arc::get_mut(&mut recorder).unwrap() = &audit::Audit::new(*cl);
    *recorder().inner.lock().unwrap() = audit::Audit {
        level: *cl,
        t: audit::AuditTarget::Stderr(),
    };

    recorder()
        .get()
        .info(audit::event("message", "server initializing"));

    let mut server = Nickel::new();

    server.get(
        "/healthz",
        middleware! { |req|
                                              healthz(req)
        },
    );

    server.utilize(check_api_keys);

    server.get(
        "/",
        middleware! { |req|
                                      handler(req)
        },
    );

    server.utilize(log_request);

    server.post(
        "/api/v1/event",
        middleware! { |req|
            postevent(req)
        },
    );
    server.get(
        "/api/v1/event",
        middleware! { |req|
            getevent(req)
        },
    );

    server.post(
        "/api/v1/measure",
        middleware! { |req|
            postmeasure(req)
        },
    );

    server.get(
        "/api/v1/measure",
        middleware! { |req|
            getmeasure(req)
        },
    );

    recorder().get().info(audit::event(
        "server starting",
        &format!("{}", server_options.port),
    ));

    server
        .listen(format!("0.0.0.0:{}", server_options.port))
        .unwrap();
}

fn launch_query(cl: &audit::ConcernLevel, qo: &QueryOptions) {
    let auditor = audit::Audit::new(*cl);
    let mut conn = db::connect_to_db(&auditor);
    println!("{:?}", qo);
    // pg / rust-postgres demand i64 as the type to be passed in.
    let last: i64 = qo.last.into();
    let printable = match qo.metric_type {
        MetricTypeOption::E => {
            let mut vec: Vec<IntrusiveEvent> = Vec::new();
            let query = "SELECT insertion_time, name, dict from monitoring.event
order by insertion_time desc
limit $1";
            for row in &conn.query(query, &[&last]).unwrap() {
                vec.push(IntrusiveEvent {
                    insertion_time: row.get(0),
                    name: row.get(1),
                    dict: row.get(2),
                });
            }
            serde_json::to_string_pretty(&vec).unwrap()
        }
        MetricTypeOption::M => {
            let mut vec: Vec<IntrusiveMeasure> = Vec::new();
            let query = "SELECT insertion_time, name, measurement, dict
from monitoring.measure
order by insertion_time desc
limit $1";
            for row in &conn.query(query, &[&last]).unwrap() {
                vec.push(IntrusiveMeasure {
                    insertion_time: row.get(0),
                    name: row.get(1),
                    measurement: row.get(2),
                    dict: row.get(3),
                });
            }

            serde_json::to_string_pretty(&vec).unwrap()
        }
    };

    println!("{}", printable);
}

#[derive(Debug, Serialize, Deserialize)]
enum PipeReader {
    M(MeasureIngest),
    E(EventIngest),
}

fn launch_writer(cl: &audit::ConcernLevel, filename: String, apikey: String) {
    let auditor = audit::Audit::new(*cl);
    let mut conn = db::connect_to_db(&auditor);

    let mut file: Box<dyn Read> = match filename.as_str() {
        "-" => Box::new(io::stdin()),
        // crashing here is ok if we can't open it.
        _ => {
            auditor.info(audit::event("opening", &filename));
            Box::new(File::open(filename).unwrap())
        }
    };

    let gatekeeper = ApiKeys {};
    let tid: i32 = match gatekeeper.check_keys(&apikey) {
        Some(i) => i,
        None => {
            auditor.info(audit::event("api key failure", &apikey));
            panic!("api key didn't work");
        }
    };

    loop {
        let mut buffer = String::new();

        // this frankly should be epoll based for a named pipe, but
        // let's let it live for now. If this is a _useful_ system, we
        // can do more with it.
        let result = file.read_to_string(&mut buffer);
        match result {
            Ok(bytecount) => {
                if bytecount > 0 {
                    auditor.info(audit::event("status", "rx"));
                    let v: Result<Vec<PipeReader>, serde_json::Error> =
                        serde_json::from_str(&buffer);

                    match v {
                        Ok(dataz) => {
                            for row in &dataz {
                                match row {
                                    PipeReader::M(measure) => {
                                        writemeasure(&mut conn, tid.clone(), measure).unwrap();
                                    }
                                    PipeReader::E(event) => {
                                        writeevent(&mut conn, tid.clone(), event).unwrap();
                                    }
                                }
                                auditor.info(audit::event("status", "written"));
                            }
                        }
                        Err(e) => {
                            auditor.crisis(audit::event("err", &format!("{:?}", e)));
                        }
                    }
                }
            }
            Err(e) => {
                auditor.info(audit::event("err", &format!("{:?}", e)));
            }
        }

        // One second pulled out of thin air.
        let one = time::Duration::from_secs(1);
        thread::sleep(one);
    }
}

#[derive(Copy, Clone, Debug)]
struct GlobalOptions {
    verbosity: audit::ConcernLevel,
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
enum OutputFormat {
    Json,
    Table,
}

#[derive(Debug)]
enum MetricTypeOption {
    M,
    E,
}

enum MetricType {
    M(Measure),
    E(Event),
}

#[derive(Debug)]
struct QueryOptions {
    metric_type: MetricTypeOption,
    last: u16,
    streaming: bool,
    format: OutputFormat,
}

#[derive(Debug)]
enum Command {
    PipeReader(GlobalOptions, CliOptions),
    Server(GlobalOptions, ServerOptions, ServerType),
    Querier(GlobalOptions, QueryOptions),
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
        format: String,
        stream: bool,
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

fn clapparser() -> Command {
    let cli = Cli::parse();

    let cl = match cli.v {
        0 => audit::ConcernLevel::Crisis,
        1 => audit::ConcernLevel::Info,
        2 | _ => audit::ConcernLevel::Debug,
    };

    let go = GlobalOptions { verbosity: cl };
    {
        match cli.cmd {
            PmetricsMode::Server { server_type, port } => {
                let st = match server_type.as_str() {
                    "http" => ServerType::Http,
                    _ => panic!("Unable to start server, crashing. specify type http"),
                };

                let so = ServerOptions { port: port };

                return Command::Server(go, so, st);
            }
            PmetricsMode::Pipe { file, api_key } => {
                return Command::PipeReader(
                    go,
                    CliOptions {
                        filename: file,
                        apikey: api_key,
                    },
                );
            }
            PmetricsMode::Querier {
                metric_type,
                last,
                format,
                stream,
            } => {
                let mt = match metric_type.as_str() {
                    "m" => MetricTypeOption::M,
                    "e" => MetricTypeOption::E,
                    _ => panic!("not a valid metric type - try m or e"),
                };

                let fmt = match format.as_str() {
                    "json" => OutputFormat::Json,
                    "table" => OutputFormat::Table,
                    _ => panic!("not a valid output format - try json or table"),
                };

                let qo = QueryOptions {
                    metric_type: mt,
                    last: last,
                    streaming: stream,
                    format: fmt,
                };
                return Command::Querier(go, qo);
            }
        }
    }
}

fn main() {
    let cmd = clapparser();

    let cl = match cmd {
        Command::Server(go, _, _) => go.verbosity,
        Command::PipeReader(go, _) => go.verbosity,
        Command::Querier(go, _) => go.verbosity,
    };

    match cmd {
        Command::Server(_, server_options, servertype) => match servertype {
            ServerType::Http => launch_server(&cl, &server_options),
        },
        Command::PipeReader(_, clioptions) => {
            launch_writer(&cl, clioptions.filename, clioptions.apikey)
        }
        Command::Querier(_, qo) => launch_query(&cl, &qo),
    }
}
