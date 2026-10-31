"""Local web viewer for LooNN games.

Usage: py -3.12 viewer/server.py   then open http://localhost:8765

Runs target/release/replay.exe to play a new game (build it with
`cargo build --release --bin replay`), and saves every game in viewer/games/
so it can be reopened instantly.
"""
from datetime import datetime
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse
import json
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
REPLAY = ROOT / "target" / "release" / "replay.exe"
GAMES = HERE / "games"
PORT = 8765
SEAT_KINDS = {"loonn", "randy"}
# Bird name -> power text, from `replay --powers`, so every game (including ones
# saved before power text existed) shows the current descriptions
POWERS = {}


def load_powers():
    global POWERS
    try:
        out = subprocess.run([str(REPLAY), "--powers"], cwd=ROOT, capture_output=True, timeout=120)
        POWERS = json.loads(out.stdout) if out.returncode == 0 else {}
    except (OSError, ValueError, subprocess.TimeoutExpired):
        POWERS = {}


def with_powers(game):
    for name, bird in game.get("birds", {}).items():
        if name in POWERS:
            bird["power_text"] = POWERS[name]
    return game


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(HERE), **kwargs)

    def log_message(self, fmt, *args):
        if "/api/" in self.path:
            super().log_message(fmt, *args)

    def send_json(self, status, body):
        data = body if isinstance(body, bytes) else json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        url = urlparse(self.path)
        if url.path == "/api/new":
            return self.new_game(parse_qs(url.query))
        if url.path == "/api/games":
            return self.list_games()
        if url.path == "/api/game":
            return self.load_game(parse_qs(url.query))
        return super().do_GET()

    def new_game(self, query):
        seats = (query.get("seats", ["loonn,loonn,loonn,loonn"])[0]).split(",")
        if len(seats) != 4 or any(s not in SEAT_KINDS for s in seats):
            return self.send_json(400, {"error": "seats must be 4 of: loonn, randy"})
        if not REPLAY.exists():
            return self.send_json(500, {"error": f"{REPLAY} not found; run cargo build --release --bin replay"})
        result = subprocess.run([str(REPLAY), *seats], cwd=ROOT, capture_output=True, timeout=900)
        if result.returncode != 0:
            tail = result.stderr.decode(errors="replace")[-2000:]
            return self.send_json(500, {"error": "replay failed", "detail": tail})
        GAMES.mkdir(exist_ok=True)
        name = datetime.now().strftime("%Y-%m-%d_%H-%M-%S") + ".json"
        (GAMES / name).write_bytes(result.stdout)
        game = with_powers(json.loads(result.stdout))
        game["file"] = name
        return self.send_json(200, game)

    def list_games(self):
        GAMES.mkdir(exist_ok=True)
        games = []
        for path in sorted(GAMES.glob("*.json"), reverse=True)[:50]:
            try:
                game = json.loads(path.read_text())
                final = game["steps"][-1]["players"]
                games.append({
                    "file": path.name,
                    "players": game["players"],
                    "scores": [p["score"] for p in final],
                })
            except (ValueError, KeyError, IndexError):
                continue
        return self.send_json(200, games)

    def load_game(self, query):
        name = Path(query.get("file", [""])[0]).name
        path = GAMES / name
        if not name.endswith(".json") or not path.exists():
            return self.send_json(404, {"error": "no such game"})
        game = with_powers(json.loads(path.read_text()))
        game["file"] = name
        return self.send_json(200, game)


if __name__ == "__main__":
    load_powers()
    print(f"LooNN game viewer at http://localhost:{PORT}  (Ctrl+C to stop)")
    ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
