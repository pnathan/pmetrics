#include "pmetrics-common.h"
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <string.h>

void exit_nicely(PGconn *conn) {
   PQfinish(conn);
   exit(1);
}

PGconn* connect_or_die(std::string conninfo) {
   PGconn *conn = PQconnectdb(conninfo.c_str());

   if (PQstatus(conn) != CONNECTION_OK) {
      fprintf(stderr, "Connection to database failed: %s", PQerrorMessage(conn));
      exit_nicely(conn);
   }

   return conn;
}

void check_insert_or_die(PGconn* conn, PGresult* res) {
   if (PQresultStatus(res) != PGRES_COMMAND_OK) {
      fprintf(stderr, "insert failed: %s", PQerrorMessage(conn));
      PQclear(res);
      exit_nicely(conn);
   }
}


std::string vacuum_from_stdin() {
   std::string data;
   ssize_t n = 0;
   const int buff_size = 256;
   char buff[buff_size];
   do
   {
      /* read(fd, buf, count) attempts to read up to count bytes from
       * file descriptor fd into the buffer starting at buf.  read
       * returns number of bytes read or -1 on error. */
      n = read(STDIN_FILENO, buff, buff_size);
      if (n == -1) {
         fprintf(stderr, "Unable to read from stdin, stopping the read and quitting");
         exit(2);
      }
      // only grab the memory vacuumed in.
      data.append(buff, n);
      // zero the buffer.
      memset(buff, 0, buff_size);
   }
   while (n > 0);
   return data;
}
