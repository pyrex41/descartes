#!/usr/bin/env python3
"""
Simple HTTP server for the Descartes blog.

Usage:
    python serve.py          # Serve on port 8000
    python serve.py 3000     # Serve on port 3000

Then open http://localhost:8000 in your browser.
"""
import http.server
import socketserver
import os
import sys

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8000

# Change to the blog directory
os.chdir(os.path.dirname(os.path.abspath(__file__)))

Handler = http.server.SimpleHTTPRequestHandler

with socketserver.TCPServer(("", PORT), Handler) as httpd:
    print(f"\n  Descartes Blog Server")
    print(f"  ────────────────────────")
    print(f"  Serving at http://localhost:{PORT}")
    print(f"  Press Ctrl+C to stop\n")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n  Server stopped.")
