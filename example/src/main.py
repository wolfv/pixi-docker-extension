#!/usr/bin/env python3
"""
Example Python application for pixi-docker demo
"""

import http.server
import socketserver
import os
from pathlib import Path

def serve_static():
    """Serve static files from the static directory"""
    PORT = int(os.environ.get('PORT', 8000))
    
    # Change to static directory if it exists, otherwise serve from current directory
    static_dir = Path('static')
    if static_dir.exists():
        os.chdir(static_dir)
    
    Handler = http.server.SimpleHTTPRequestHandler
    
    with socketserver.TCPServer(("", PORT), Handler) as httpd:
        print(f"Server running at http://localhost:{PORT}")
        httpd.serve_forever()

if __name__ == "__main__":
    serve_static()