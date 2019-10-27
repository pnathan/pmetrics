import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pmetrics

HOST_NAME = 'localhost'
PORT_NUMBER = 9001

class HttpHandler(BaseHTTPRequestHandler):
    # shared
    lut = {}

    def do_HEAD(self):
        self.send_response(200)
        self.send_header('Content-type', 'text/html')
        self.end_headers()

    def register_url(self, url, handler):
        self.lut[url] = handler

    def do_GET(self):
        if self.path in self.lut:
            content, code = self.lut[self.path](self)
            self.send_header('Content-type', 'text/html')
            self.end_headers()
        else:
            content, code = ("not found", 404)

        self.send_response(code)

        self.wfile.write(bytes(content, 'UTF-8'))

def and_pmetrix(server, code):
    block = {
        'client_host': server.client_address[0],
        'client_port': server.client_address[1],
        'method': server.command,
        'path' : server.path,
        #'headers' :
        'server_version' : server.server_version,
        'sys_version' : server.sys_version,
        'protocol_version' : server.protocol_version,
        'time': time.asctime(), # should be ISO8601
        'status_code': code
    }

    pmetrics.PMETRICS.add_event('demo_http_server_write', block)

def pmetrix(server):
    return pmetrics.PMETRICS.to_json(), 200


def indexer(server):
    content = "indexing..."
    code = 200
    and_pmetrix(server, code)
    return content, code

if __name__ == '__main__':
    hh = HttpHandler.__new__(HttpHandler)
    hh.register_url("/", indexer)
    hh.register_url("/pmetrix", pmetrix)
    httpd = ThreadingHTTPServer((HOST_NAME, PORT_NUMBER), HttpHandler)
    print(time.asctime(), 'Server Starts - %s:%s' % (HOST_NAME, PORT_NUMBER))
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    httpd.server_close()
    print(time.asctime(), 'Server Stops - %s:%s' % (HOST_NAME, PORT_NUMBER))
