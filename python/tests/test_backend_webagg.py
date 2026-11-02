"""
Smoke tests of the WebAgg and nbAgg backends (`pltrs.backends.
backend_webagg`, `backend_nbagg`) over pltrs (Rust)'s web server: a figure
served by ``plt.show()`` in a fresh interpreter, driven by a minimal
browser (HTTP, and a WebSocket speaking matplotlib's protocol as its
``mpl.js`` does), and stopped by Ctrl+C, as matplotlib's ``test_webagg``
stops it. The expectations are matplotlib's: its pages, protocol and
messages (``backend_webagg_core``), its WebSocket's origin check (its
``test_websocket_rejects_cross_origin``) and its server's output.
"""

import base64
import inspect
import json
import os
import re
import signal
import socket
import struct
import subprocess
import sys
import textwrap
import urllib.request
from unittest.mock import MagicMock

import pytest

#: How long a step may take, in seconds.
TIMEOUT = 30


def _serve():
    """A figure served by WebAgg, printing what reaches its callbacks."""
    import pltrs as mpl
    import pltrs.pyplot as plt

    mpl.rcParams["webagg.open_in_browser"] = False
    mpl.rcParams["webagg.port"] = 0
    fig, ax = plt.subplots()
    ax.plot([0, 1], [2, 3])
    fig.canvas.mpl_connect(
        "button_press_event",
        lambda event: print("PRESS", event.x, event.y, event.button,
                            flush=True))
    fig.canvas.mpl_connect(
        "key_press_event", lambda event: print("KEY", event.key, flush=True))
    plt.show()
    print("RETURNED", flush=True)


class _Browser:
    """A page's WebSocket, as matplotlib's ``mpl.js`` opens it."""

    def __init__(self, host, port, path):
        self.sock = socket.create_connection((host, port), timeout=TIMEOUT)
        key = base64.b64encode(os.urandom(16)).decode()
        self.sock.sendall(
            f"GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\n"
            f"Upgrade: websocket\r\nConnection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n"
            f"Origin: http://{host}:{port}\r\n\r\n".encode())
        head = b""
        while b"\r\n\r\n" not in head:
            head += self.sock.recv(1)
        assert head.startswith(b"HTTP/1.1 101"), head

    def _read(self, size):
        data = b""
        while len(data) < size:
            chunk = self.sock.recv(size - len(data))
            assert chunk, "the server closed the connection"
            data += chunk
        return data

    def send(self, message):
        """Send *message* as JSON in a masked text frame."""
        payload = json.dumps(message).encode()
        mask = os.urandom(4)
        assert len(payload) < 126
        frame = bytes([0x81, 0x80 | len(payload)]) + mask + bytes(
            b ^ mask[i % 4] for i, b in enumerate(payload))
        self.sock.sendall(frame)

    def receive(self):
        """The next message: a dict for JSON text, bytes for a frame."""
        first, second = self._read(2)
        length = second & 0x7F
        if length == 126:
            length, = struct.unpack(">H", self._read(2))
        elif length == 127:
            length, = struct.unpack(">Q", self._read(8))
        payload = self._read(length)
        if first & 0x0F == 0x1:
            return json.loads(payload)
        return payload

    def receive_until(self, wanted):
        """Messages until one *wanted* accepts, which is returned."""
        while True:
            message = self.receive()
            if wanted(message):
                return message


def _get(url):
    with urllib.request.urlopen(url, timeout=TIMEOUT) as response:
        return response.status, response.read()


@pytest.mark.skipif(sys.platform == "win32", reason="no SIGINT to send")
def test_webagg_serves_and_drives_a_figure():
    source = textwrap.dedent(inspect.getsource(_serve)) + "\n_serve()\n"
    env = {**os.environ, "MPLBACKEND": "webagg", "PLTRS_LOOK": "matplotlib"}
    proc = subprocess.Popen([sys.executable, "-c", source], env=env,
                            stdout=subprocess.PIPE, text=True)
    try:
        announced = proc.stdout.readline()
        url = re.search(r"visit (https?://\S+)", announced).group(1)
        assert proc.stdout.readline() == "Press Ctrl+C to stop WebAgg server\n"
        host, port = re.match(r"http://([^:/]+):(\d+)", url).groups()
        status, page = _get(url)
        assert status == 200 and b"ready(figure_ready(1));" in page
        status, page = _get(f"{url}/1")
        assert status == 200 and b"ws://" in page
        status, script = _get(f"{url}/js/mpl.js")
        assert b"mpl.toolbar_items = " in script
        assert b'mpl.default_extension = "png"' in script
        status, png = _get(f"{url}/1/download.png")
        assert png.startswith(b"\x89PNG")

        browser = _Browser(host, int(port), "/1/ws")
        browser.send({"type": "supports_binary", "value": True,
                      "figure_id": "1"})
        browser.send({"type": "send_image_mode", "figure_id": "1"})
        browser.send({"type": "refresh", "figure_id": "1"})
        browser.send({"type": "set_device_pixel_ratio",
                      "device_pixel_ratio": 1, "figure_id": "1"})
        label = browser.receive_until(
            lambda m: isinstance(m, dict) and m["type"] == "figure_label")
        assert label["label"] == "Figure 1"
        browser.send({"type": "draw", "figure_id": "1"})
        frame = browser.receive_until(lambda m: isinstance(m, bytes))
        assert frame.startswith(b"\x89PNG")
        browser.send({"type": "button_press", "x": 100, "y": 50,
                      "button": 0, "buttons": 1, "modifiers": [],
                      "figure_id": "1"})
        # mpl.js names keys "k" + the browser's name.
        browser.send({"type": "key_press", "key": "ka", "figure_id": "1"})
        assert proc.stdout.readline() == "PRESS 100 430 1\n"
        assert proc.stdout.readline() == "KEY a\n"

        proc.send_signal(signal.SIGINT)
        stdout, _ = proc.communicate(timeout=TIMEOUT)
        assert proc.returncode == 0
        assert stdout == "Server is stopped\nRETURNED\n"
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.communicate()


@pytest.mark.parametrize("host, origin, allowed", [
    ("localhost:8988", "http://localhost:8988", True),
    ("localhost:8988", "http://evil.com", False),
    ("localhost:8988", "http://127.0.0.1:8988", False),
    ("127.0.0.1:8988", "http://127.0.0.1:8988", True),
    ("[::1]:8988", "http://[::1]:8988", True),
    ("[::1]:8988", "http://[::2]:8988", False),
])
def test_websocket_rejects_cross_origin(host, origin, allowed):
    """matplotlib's own test, without Tornado."""
    from pltrs.backends.backend_webagg import WebAggApplication

    ws = WebAggApplication.WebSocket.__new__(WebAggApplication.WebSocket)
    ws.request = MagicMock()
    ws.request.headers = {"Host": host}
    assert ws.check_origin(origin) is allowed


def test_static_files_are_matplotlibs_web_backend(tmp_path, monkeypatch):
    from pltrs.backends import backend_webagg_core as core

    monkeypatch.setattr(core, "_static_file_path", None)
    monkeypatch.setattr(core, "get_cachedir", lambda: str(tmp_path))
    path = core.FigureManagerWebAgg.get_static_file_path()
    names = sorted(str(p.relative_to(path)) for p in tmp_path.rglob("*")
                   if p.is_file())
    assert names == [
        "LICENSE", "all_figures.html", "css/boilerplate.css", "css/fbm.css",
        "css/mpl.css", "css/page.css", "ipython_inline_figure.html",
        "js/mpl.js", "js/mpl_tornado.js", "js/nbagg_mpl.js",
        "single_figure.html"]


def test_webagg_without_tornado_and_nbagg_need_ipykernel():
    """WebAgg needs no Tornado here; nbAgg needs IPython's kernel, as
    matplotlib's does."""
    import pltrs.backends.backend_webagg  # noqa: F401
    import importlib.util
    if importlib.util.find_spec("ipykernel") is None:
        with pytest.raises(ImportError):
            import pltrs.backends.backend_nbagg  # noqa: F401
