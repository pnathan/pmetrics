extern crate postgres;
use std::env;
use postgres::{Connection, TlsMode};
use std::collections::HashMap;

use audit;

struct PostgresClientArgs {
    user: String,
    password: String,
    host: String,
    port: u16,
    db: String,
}

impl PostgresClientArgs {
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

        let pgport = match envmap.get("PGPORT") {
            Some(s) => s.parse::<u16>().unwrap(),
            None => 5432
        };
        let pghost = match envmap.get("PGHOST") {
            Some(s) => s,
            None => "localhost"
        };
        let pgdb = match envmap.get("PGDATABASE") {
            Some(s) => s,
            None => "postgres"
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


pub fn connect_to_db(auditor: &audit::Audit) -> postgres::Connection {
    let cs = PostgresClientArgs::from_env().connection_string();
    let conn = Connection::connect(cs, TlsMode::None).unwrap();
    auditor.tell(&audit::Concern::Info(audit::Event::new("started", "pg conn")));

    conn
}
