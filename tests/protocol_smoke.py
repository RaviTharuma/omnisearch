"""Executable MCP stdio smoke, exclusively against a localhost mock gateway."""
import json
import os
import select
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

calls = []
class Gateway(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass
    def do_POST(self):
        assert self.path == '/v1/search'
        assert self.headers['Authorization'] == 'Bearer mock-only-key'
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        assert body['query'] == 'protocol smoke'
        calls.append(body)
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps({'results': [{'title': 'Mock result', 'url': 'https://example.org/smoke', 'snippet': 'Local protocol fixture'}]}).encode())

server = HTTPServer(('127.0.0.1', 0), Gateway)
threading.Thread(target=server.serve_forever, daemon=True).start()
env = {'PATH': os.environ['PATH'], 'HOME': os.environ['HOME'], 'RUST_LOG': 'off',
       'OMNISEARCH_ACCOUNTS': '[]',
       'OMNISEARCH_OMNIROUTE_GATEWAYS': json.dumps([{'name': 'mock', 'base_url': f'http://127.0.0.1:{server.server_port}', 'api_key': 'mock-only-key'}])}
exe = Path(os.environ.get('OMNISEARCH_BIN', str(Path(__file__).resolve().parents[1] / 'target/debug/omnisearch')))
p = subprocess.Popen([str(exe)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env, cwd='/tmp')
assert p.stdin is not None and p.stdout is not None
def send(obj):
    assert p.stdin is not None
    p.stdin.write(json.dumps(obj)+'\n'); p.stdin.flush()
def rpc(i, method, params=None):
    assert p.stdout is not None
    send({'jsonrpc':'2.0','id':i,'method':method, **({'params':params} if params is not None else {})})
    assert select.select([p.stdout], [], [], 10)[0], 'protocol timed out'
    result = json.loads(p.stdout.readline())
    assert result.get('id') == i and 'error' not in result, result
    return result['result']
try:
    init = rpc(1, 'initialize', {'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'smoke','version':'1'}})
    assert init['serverInfo']['version'] == '0.2.0', init
    send({'jsonrpc':'2.0','method':'notifications/initialized'})
    tools = rpc(2, 'tools/list')
    assert 'account_health' in [t['name'] for t in tools['tools']]
    result = rpc(3, 'tools/call', {'name':'search','arguments':{'query':'protocol smoke','providers':['omniroute'],'limit':1}})
    assert not result.get('isError'), result
    assert 'Mock result' in json.dumps(result), result
    health = rpc(4, 'tools/call', {'name':'account_health','arguments':{}})
    assert 'mock-only-key' not in json.dumps(health)
    assert len(calls) == 1, calls
    print('PASS: initialize 0.2.0, tools/list, search gateway auth/result, account_health redaction; one local HTTP call')
finally:
    p.terminate(); p.wait(timeout=5); server.shutdown()
