```python
#!/usr/bin/env python3

import socket
import sys
from typing import Tuple
import requests

# Hardcoded configuration
LISTEN_HOST = "192.168.111.1"
LISTEN_PORT = 5000
TARGET_HOST = "127.0.0.1"
TARGET_PORT = 5001
RECV_BUFSIZE = 65536


def recv_until_eof(sock: socket.socket) -> bytes:
    """Receive from a socket until the peer closes its write side (EOF)."""
    chunks = []
    while True:
        try:
            data = sock.recv(RECV_BUFSIZE)
        except InterruptedError:
            continue
        if not data:
            break
        chunks.append(data)
    return b"".join(chunks)


def send_all_and_shutdown_wr(sock: socket.socket, data: bytes) -> None:
    """Send all data to sock, then half-close the write side to signal end-of-data."""
    if data:
        sock.sendall(data)
    try:
        sock.shutdown(socket.SHUT_WR)
    except OSError:
        # Already closed or platform limitation; ignore
        pass


def create_server(address: Tuple[str, int]) -> socket.socket:
    """Create a simple IPv4 TCP server socket bound to address and listening."""
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    except OSError:
        pass
    srv.bind(address)
    srv.listen(1)
    return srv


def main() -> int:
    print(f"[+] Proxy starting. Listen {LISTEN_HOST}:{LISTEN_PORT} -> Target {TARGET_HOST}:{TARGET_PORT}", flush=True)

    srv = None
    client = None
    target = None

    try:
        srv = create_server((LISTEN_HOST, LISTEN_PORT))
        client, client_addr = srv.accept()
        print(f"[+] Client connected from {client_addr}", flush=True)

        # 1) Receive everything from client until FIN/EOF
        print("[*] Receiving request from client until EOF...", flush=True)
        client_request = recv_until_eof(client)
        print(f"[+] Received {len(client_request)} bytes from client", flush=True)

        # 2) Connect to target, forward all at once, shutdown write
        print(f"[*] Connecting to target {TARGET_HOST}:{TARGET_PORT}...", flush=True)
        target = socket.create_connection((TARGET_HOST, TARGET_PORT))
        print("[*] Sending request to target and shutting down write...", flush=True)
        send_all_and_shutdown_wr(target, client_request)

        # 3) Receive response from target until it closes
        print("[*] Receiving response from target until EOF...", flush=True)
        target_response = recv_until_eof(target)
        print(f"[+] Received {len(target_response)} bytes from target", flush=True)

        # 4) Send response back to client and shutdown write
        print("[*] Sending response back to client...", flush=True)
        send_all_and_shutdown_wr(client, target_response)

        print("[+] Done. Closing sockets.", flush=True)
        return 0

    except Exception as e:
        print(f"[!] Proxy error: {e}", file=sys.stderr, flush=True)
        return 1
    finally:
        for s in (target, client, srv):
            try:
                if s:
                    s.close()
            except Exception:
                pass


if __name__ == "__main__":
    sys.exit(main())
```
