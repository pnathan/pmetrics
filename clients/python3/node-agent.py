#!/usr/bin/env python3
__doc__ = """

Runs a lightweight python server on 9001 which yields information
about the linux node it's running on.

Assumes psutil is installed and the pmetrics client library is
available.

"""



import datetime
import os
import platform

import threading
import time
import traceback


from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

# https://github.com/giampaolo/psutil
import psutil

import pmetrics

import uuid
agent_temp_identity = str( uuid.uuid4())

def get_sysinfo():

    # dict of nts
    def unroll_nt(dnt):
        for k in dnt:
            dnt[k] = dict(dnt[k]._asdict())
        return dnt

    one, five, fifteen = psutil.getloadavg()
    loadavg = { '1': one, '5' : five, '15': fifteen}

    memory = dict(psutil.virtual_memory()._asdict())
    swap = dict(psutil.swap_memory()._asdict())

    cpu_usage = dict(psutil.cpu_times_percent(percpu=False)._asdict())
    cpu_freq = dict(psutil.cpu_freq(percpu=False)._asdict())

    # disk data needs to be refined in presentation.
    #mounts = psutil.disk_partitions()
    #mountdata =nt_dict(mounts)

    # these aren't super reliable. Runs ok in ipython, but, not in server context.


    # dc = unroll_nt(psutil.disk_io_counters(perdisk=True))

    # netio = unroll_nt(psutil.net_io_counters(pernic=True))



    block = {
        'time': datetime.datetime.utcnow().replace(tzinfo=datetime.timezone.utc).isoformat(),
        'agent_id': agent_temp_identity,

        'machine': platform.machine(),
        'processor': platform.processor(),
        'node': platform.node(),
        'platform': platform.platform(),
        'system': platform.system(),
        'release': platform.release(),
        'version': platform.version(),
        'cpu_usage' : cpu_usage,
        'cpu_freq' : cpu_freq,
        'loadavg' : loadavg,
        'memory': memory,
        'swap': swap
        # 'disks' : diskdata,
        # 'diskio' : dc,
        # 'netio' : netio,
        # 'mounts' : mountdata
    }

    return block

def pmetrix_updater():
    while True:
        try:
            block = get_sysinfo()
            pmetrics.PMETRICS.add_event('node_data', block)
        except Exception as e:
            # swallow it all
            traceback.print_exc()
        time.sleep(5)


HOST_NAME = 'localhost'
PORT_NUMBER = 9001

class HttpHandler(BaseHTTPRequestHandler):
    # shared
    lut = {}

    def register_url(self, url, handler):
        self.lut[url] = handler

    def do_GET(self):
        if self.path in self.lut:
            content, code = self.lut[self.path](self)
        else:
            content, code = ("not found", 404)

        self.send_response(code)
        b = bytes(content, 'UTF-8')

        self.send_header('Content-Length', len(b))
        self.send_header('Content-Type', 'application/json')
        self.end_headers()

        self.wfile.write(b)

def pmetrix(server):
    j = pmetrics.PMETRICS.to_json()
    import json
    json.loads(j)
    return j, 200

def indexer(server):
    content = "node agent."
    code = 200
    return content, code

if __name__ == '__main__':
    hh = HttpHandler.__new__(HttpHandler)
    t = threading.Thread(target=pmetrix_updater, daemon=True)
    t.start()

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
