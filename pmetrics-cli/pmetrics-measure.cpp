#include <libpq-fe.h>
#include <stdio.h>
#include <stdlib.h>
#include <string>
#include <string.h>
#include <unistd.h>
#include <vector>

#include "pmetrics-common.h"

using namespace std;

// yoinked from
// http://stackoverflow.com/questions/13172158/c-split-string-by-line
vector<string> split_string(const string& str, const string& delimiter) {
    vector<string> strings;

    string::size_type pos = 0;
    string::size_type prev = 0;
    while ((pos = str.find(delimiter, prev)) != string::npos) {
        strings.push_back(str.substr(prev, pos - prev));
        prev = pos + 1;
    }

    // To get the last substring (or only, if delimiter is not found)
    strings.push_back(str.substr(prev));

    return strings;
}

// stdlib algorithms and template code seem to be super awkward
string join_string(vector<string> vec, int begin, int end_size, string delimiter) {
   string result;
   for(int i = begin; i < end_size - 1; i++) {
      result += vec[i] + delimiter;
   }

   result += vec[end_size - 1];
   return result;
}

int main() {

   string stdin_data = vacuum_from_stdin();

   // this is not very efficient. Better to read 1 line, read the next
   // line, then slurrrrp away. tomorrow we dine in speed.
   vector<string> inputs = split_string(stdin_data, "\n");
   string name = inputs[0];
   string measurement = inputs[1];
   string json = join_string(inputs, 2, inputs.size(), "\n");

   PGconn* conn = connect_or_die("");

   string query = "insert into monitoring.measure (insertion_time, name, measurement, dict) values (now(), $1, $2, $3)";
   const char* name_char = name.c_str();
   const char* measurement_char = measurement.c_str();
   const char* dict = json.c_str();
   const char* params[] = {name_char, measurement_char, dict};

   PGresult* res = PQexecParams(conn,
                                query.c_str(),
                                3,
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
