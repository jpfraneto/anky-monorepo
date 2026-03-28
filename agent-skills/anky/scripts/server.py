#!/usr/bin/env python3
"""Anky writing server. Usage: python3 server.py PROMPT HTML_PATH"""
import http.server, json, sys, threading, time
from pathlib import Path

PROMPT = sys.argv[1] if len(sys.argv) > 1 else "What you keep not saying."
HTML_PATH = sys.argv[2] if len(sys.argv) > 2 else ""
HTML = ""
if HTML_PATH and Path(HTML_PATH).exists():
    HTML = Path(HTML_PATH).read_text().replace("{PROMPT}", PROMPT)
WRITINGS = Path.home() / "anky" / "writings"
sessions = {}

class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.end_headers()
        self.wfile.write(HTML.encode())
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        sessions[body["id"]] = body
        if body.get("done"):
            WRITINGS.mkdir(parents=True, exist_ok=True)
            (WRITINGS / f"{body['id']}.txt").write_text(body.get("keystrokes", ""))
            (WRITINGS / f"{body['id']}_readable.txt").write_text(body.get("text", ""))
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"ok": True}).encode())
    def log_message(self, *a): pass

port = 8877
for _ in range(5):
    try:
        srv = http.server.HTTPServer(("127.0.0.1", port), H)
        break
    except OSError:
        port += 1
else:
    print("ANKY_SERVER_FAIL", file=sys.stderr)
    sys.exit(1)

print(f"ANKY_SERVER_READY:{port}", flush=True)
sys.stdout.flush()
srv.serve_forever()
