#!/usr/bin/env python3
"""Simple HTTP server with GET /hello and GET /health endpoints."""

import json
from http.server import HTTPServer, BaseHTTPRequestHandler


class RequestHandler(BaseHTTPRequestHandler):
    """Custom request handler for the HTTP server."""

    def _send_json_response(self, status_code: int, data: dict) -> None:
        """Send a JSON response with the given status code and data."""
        self.send_response(status_code)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(data).encode("utf-8"))

    def do_GET(self) -> None:
        """Handle GET requests."""
        if self.path == "/hello":
            self._send_json_response(200, {"message": "Hello, World!"})
        elif self.path == "/health":
            self._send_json_response(200, {"status": "healthy"})
        else:
            self._send_json_response(404, {"error": "Not Found"})

    def log_message(self, format: str, *args) -> None:
        """Log HTTP requests to stdout."""
        print(f"{self.address_string()} - {format % args}")


def run_server(host: str = "localhost", port: int = 8000) -> None:
    """Run the HTTP server."""
    server_address = (host, port)
    httpd = HTTPServer(server_address, RequestHandler)
    print(f"Server running on http://{host}:{port}")
    print("Endpoints:")
    print("  GET /hello  - Returns a greeting message")
    print("  GET /health - Returns health status")
    print("\nPress Ctrl+C to stop the server")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nServer stopped.")
        httpd.server_close()


if __name__ == "__main__":
    run_server()
