/*
(C) 2015,2016 Paul Nathan

writes stdin to postgres on the monitoring.log table

This is how you compile this file, more or less.

#!/bin/bash
g++ -I/usr/include/postgresql -L/usr/lib/postgresql/9.4/lib -lpq <this-file>.cpp -o log2pg -Wall -pedantic


If you use rsyslog, add this to your conf, assuming that

# this will tell rsyslog to tee out to this.
*.*   ^/usr/local/bin/remark-log-to-pg

*/
#include <stdio.h>
#include <stdlib.h>
#include <string>
#include <string.h>
#include <unistd.h>

#include <libpq-fe.h>
#include "pmetrics-common.h"
using namespace std;

/*
  PGHOST behaves the same as the host connection parameter.

  PGHOSTADDR behaves the same as the hostaddr connection
  parameter. This can be set instead of or in addition to PGHOST to
  avoid DNS lookup overhead.

  PGPORT behaves the same as the port connection parameter.

  PGUSER behaves the same as the user connection parameter.

  PGDATABASE is the database to connect to.

  PGPASSWORD behaves the same as the password connection
  parameter. Use of this environment variable is not recommended for
  security reasons, as some operating systems allow non-root users to
  see process environment variables via ps; instead consider using the
  ~/.pgpass file (see Section 31.15).

*/


int main() {
   string data = vacuum_from_stdin();

   // we pull in the connection settings from environment variables.
   PGconn* conn = connect_or_die("");

   string query = "insert into monitoring.log (insertion_time, logtext) values (now(), $1)";
   const char* temp = data.c_str();
   const char* params[]={temp};

   PGresult* res = PQexecParams(conn,
                                query.c_str(),
                                1,
                                NULL,
                                params,
                                0,
                                0,
                                0);

   check_insert_or_die(conn, res);

   PQclear(res);
   PQfinish(conn);

   return 0;
}
