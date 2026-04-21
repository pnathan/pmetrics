use deadpool_postgres::{Config, Pool, Runtime};
use percent_encoding_rfc3986::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::collections::HashMap;
use std::env;
use tokio_postgres::NoTls;

trait PgConn {
    fn from_env() -> Self
    where
        Self: Sized;
    fn connection_string(&self) -> String;
}

struct PostgresSocketClientArgs {
    user: String,
    password: String,
    socket: String,
    db: String,
}

impl PgConn for PostgresSocketClientArgs {
    fn from_env() -> PostgresSocketClientArgs {
        let envmap: HashMap<String, String> = env::vars().collect();

        let pguser = match envmap.get("PGUSER") {
            Some(s) => s,
            None => "",
        };
        let pgpass = match envmap.get("PGPASSWORD") {
            Some(s) => s,
            None => "",
        };

        let pgdb = match envmap.get("PGDATABASE") {
            Some(s) => s,
            None => "postgres",
        };

        let pgsocket = match envmap.get("INSTANCE_UNIX_SOCKET") {
            Some(s) => s,
            None => {
                panic!("env var disappeared mid-process")
            }
        };

        PostgresSocketClientArgs {
            user: pguser.to_string(),
            password: pgpass.to_string(),
            socket: pgsocket.to_string(),
            db: pgdb.to_string(),
        }
    }

    fn connection_string(&self) -> String {
        format!(
            "user={} password={} dbname={} host={}",
            self.user, self.password, self.db, self.socket
        )
    }
}

struct PostgresClientArgs {
    user: String,
    password: String,
    host: String,
    port: u16,
    db: String,
}

impl PgConn for PostgresClientArgs {
    fn from_env() -> PostgresClientArgs {
        let envmap: HashMap<String, String> = env::vars().collect();

        let pguser = match envmap.get("PGUSER") {
            Some(s) => s,
            None => "",
        };
        let pgpass = match envmap.get("PGPASSWORD") {
            Some(s) => s,
            None => "",
        };

        let pgdb = match envmap.get("PGDATABASE") {
            Some(s) => s,
            None => "postgres",
        };

        let pgport = match envmap.get("PGPORT") {
            Some(s) => match s.parse::<u16>() {
                Ok(p) => p,
                Err(err) => {
                    tracing::info!("error=true module=db class=env-parse-int value={s} error={err:?}");
                    5432
                }
            },
            None => 5432,
        };
        let pghost = match envmap.get("PGHOST") {
            Some(s) => s,
            None => "localhost",
        };

        PostgresClientArgs {
            user: pguser.to_string(),
            password: pgpass.to_string(),
            host: pghost.to_string(),
            port: pgport,
            db: pgdb.to_string(),
        }
    }

    fn connection_string(&self) -> String {
        let pw = utf8_percent_encode(&self.password, NON_ALPHANUMERIC).to_string();
        let user = utf8_percent_encode(&self.user, NON_ALPHANUMERIC).to_string();

        format!(
            "postgres://{}:{}@{}:{}/{}",
            user, pw, self.host, self.port, self.db
        )
    }
}

fn get_connection_string() -> String {
    let envmap: HashMap<String, String> = env::vars().collect();
    if envmap.contains_key("INSTANCE_UNIX_SOCKET") {
        PostgresSocketClientArgs::from_env().connection_string()
    } else {
        PostgresClientArgs::from_env().connection_string()
    }
}

pub fn build_pool() -> Pool {
    let mut cfg = Config::new();
    cfg.url = Some(get_connection_string());
    cfg.pool = Some(deadpool_postgres::PoolConfig::new(16));
    cfg.create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("create pg pool")
}
