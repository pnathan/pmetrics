extern crate postgres;
use std::env;
use postgres::{Client, NoTls};
use std::collections::HashMap;

use crate::audit;

trait PgConn {
    fn from_env() -> Self where Self: Sized;
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
        let envmap: HashMap<String, String> =  env::vars().into_iter().collect();


        let pguser = match envmap.get("PGUSER") {
            Some(s) => s,
            None => ""
        };
        let pgpass = match envmap.get("PGPASSWORD") {
            Some(s) =>  s,
            None => ""
        };

        let pgdb = match envmap.get("PGDATABASE") {
            Some(s) => s,
            None => "postgres"
        };

        let pgsocket = match envmap.get("INSTANCE_UNIX_SOCKET") {
            Some(s) => s,
            None => {  panic!("env var disappeared mid-process") }
        };

        PostgresSocketClientArgs {
            user: pguser.to_string(),
            password: pgpass.to_string(),
            socket: pgsocket.to_string(),
            db: pgdb.to_string()
        }
    }

    fn connection_string(&self) -> String {
        format!("user={} password={} database={} host={}", self.user, self.password, self.db, self.socket)
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
        let envmap: HashMap<String, String> =  env::vars().into_iter().collect();


        let pguser = match envmap.get("PGUSER") {
            Some(s) => s,
            None => ""
        };
        let pgpass = match envmap.get("PGPASSWORD") {
            Some(s) =>  s,
            None => ""
        };


        let pgdb = match envmap.get("PGDATABASE") {
            Some(s) => s,
            None => "postgres"
        };

        let pgport = match envmap.get("PGPORT") {
            Some(s) => s.parse::<u16>().unwrap(),
            None => 5432
        };
        let pghost = match envmap.get("PGHOST") {
            Some(s) => s,
            None => "localhost"
        };



        PostgresClientArgs {
            user: pguser.to_string(),
            password: pgpass.to_string(),
            host: pghost.to_string(),
            port: pgport,
            db: pgdb.to_string()
        }
    }

    fn connection_string(&self) -> String {
        format!("postgres://{}:{}@{}:{}/{}", self.user, self.password, self.host, self.port, self.db)
    }
}

// hacky facade for different env var combos.
fn get_connection_string() -> String {
    let envmap: HashMap<String, String> =  env::vars().into_iter().collect();
    if let Some(_) = envmap.get("INSTANCE_UNIX_SOCKET")  {
        PostgresSocketClientArgs::from_env().connection_string()

    } else {
        PostgresClientArgs::from_env().connection_string()
    }
}

pub fn connect_to_db(auditor: &audit::Audit) -> postgres::Client {
    let cs = get_connection_string();
    let conn = Client::connect(cs.as_str(), NoTls).unwrap();
    auditor.tell(&audit::Concern::Info(audit::Event::new("started", "pg conn")));

    conn
}
