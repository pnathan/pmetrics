#include <libpq-fe.h>
#include <string>

void exit_nicely(PGconn* conn);
PGconn* connect_or_die(std::string conninfo);
void check_insert_or_die(PGconn* conn, PGresult* res);
std::string vacuum_from_stdin();
