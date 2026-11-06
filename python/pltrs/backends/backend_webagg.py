"""
Displays Agg images in the browser, with interactivity (matplotlib's
``backend_webagg``).

matplotlib's module runs Tornado; pltrs runs pltrs (Rust)'s web server
instead (``_native.web``, a ``WebRelay``), so it needs no Tornado. The
server serves matplotlib's pages, client script (`.FigureManagerWebAgg.
get_javascript`), style sheets, toolbar images and downloads (saved by
the figure's `~.Figure.savefig`), and hands over what the pages'
WebSockets send: `WebAggApplication` gives each WebSocket a
`WebAggApplication.WebSocket`, which matplotlib's figure managers
(`.backend_webagg_core`) talk to as they talk to Tornado's. The server
runs in an asyncio event loop (`WebAggApplication.start`), as Tornado
does, which also runs the canvases' `.TimerAsyncio` timers. Its pages
and files are served under the URL prefix `WebAggApplication.initialize`
is given, as Tornado's handlers are.

From matplotlib 3.11.2, under the Matplotlib License, for what is kept
from it (the backend's classes, the WebSocket's protocol, the start and
stop of the server, the inline display).
"""

# The WebAgg backend is divided into two modules:
#
# - `backend_webagg_core.py` contains code necessary to embed a WebAgg
#   plot inside of a web application, and communicate in an abstract
#   way over a web socket.
#
# - `backend_webagg.py` contains a concrete implementation of a basic
#   application, implemented with pltrs (Rust)'s web server.

import asyncio
from base64 import b64encode
from contextlib import contextmanager
from io import BytesIO
import json
import logging
from pathlib import Path
import signal
import sys
from urllib.parse import urlparse

from .. import colors, rcParams
from .._native import web as _web
from .._pylab_helpers import Gcf
from ..backend_bases import _Backend
from . import backend_webagg_core as core
from .backend_webagg_core import (  # noqa: F401 # pylint: disable=W0611
    TimerAsyncio, TimerTornado)

_log = logging.getLogger(__name__)

#: How often the server hands over what the clients sent, in seconds.
_POLL_INTERVAL = 0.01


class FigureManagerWebAgg(core.FigureManagerWebAgg):
    _toolbar2_class = core.NavigationToolbar2WebAgg

    @classmethod
    def pyplot_show(cls, *, block=None):
        WebAggApplication.initialize()

        url = "http://{address}:{port}{prefix}".format(
            address=WebAggApplication.address,
            port=WebAggApplication.port,
            prefix=WebAggApplication.url_prefix)

        if rcParams['webagg.open_in_browser']:
            import webbrowser
            if not webbrowser.open(url):
                print(f"To view figure, visit {url}")
        else:
            print(f"To view figure, visit {url}")

        WebAggApplication.start()


class FigureCanvasWebAgg(core.FigureCanvasWebAggCore):
    manager_class = FigureManagerWebAgg


def _is_dark(figure):
    """Whether *figure* is drawn on a dark background (the modern chrome
    shows its page in its dark theme)."""
    red, green, blue = colors.to_rgb(figure.get_facecolor())
    return 0.2126 * red + 0.7152 * green + 0.0722 * blue < 0.5


class WebAggApplication:
    """
    The WebAgg server: pltrs (Rust)'s web server, serving the figures of
    `.Gcf` (matplotlib's Tornado application, without Tornado).
    """
    initialized = False
    started = False
    url_prefix = ''
    address = None
    port = None

    #: The server, once initialized.
    _relay = None
    #: The open WebSockets, by connection.
    _sockets = {}

    class WebSocket:
        """
        A page's WebSocket, as matplotlib's figure managers use it: the
        messages the page sends arrive at `on_message`, and `send_json` and
        `send_binary` send it events and frames.
        """
        supports_binary = True

        def __init__(self, connection, request=None):
            self.connection = connection
            #: The request that opened the WebSocket, when known (its
            #: ``headers`` hold the ``Host`` `check_origin` compares with).
            self.request = request

        def check_origin(self, origin):
            """
            Whether a page from *origin* may open the WebSocket: only a page
            from the same host and port as the request's ``Host`` (Tornado's
            default, which pltrs' server applies to every upgrade).
            """
            return (urlparse(origin).netloc.lower()
                    == self.request.headers.get("Host"))

        def open(self, fignum):
            self.fignum = int(fignum)
            self.manager = Gcf.get_fig_manager(self.fignum)
            self.manager.add_web_socket(self)

        def on_close(self):
            self.manager.remove_web_socket(self)

        def on_message(self, message):
            message = json.loads(message)
            # The 'supports_binary' message is on a client-by-client
            # basis.  The others affect the (shared) canvas as a
            # whole.
            if message['type'] == 'supports_binary':
                self.supports_binary = message['value']
            else:
                manager = Gcf.get_fig_manager(self.fignum)
                # It is possible for a figure to be closed,
                # but a stale figure UI is still sending messages
                # from the browser.
                if manager is not None:
                    manager.handle_json(message)

        def send_json(self, content):
            self.write_message(json.dumps(content))

        def send_binary(self, blob):
            if self.supports_binary:
                self.write_message(blob, binary=True)
            else:
                data_uri = "data:image/png;base64,{}".format(
                    b64encode(blob).decode("ascii"))
                self.write_message(data_uri)

        def write_message(self, message, binary=False):
            """Send *message* (bytes when *binary*, else text) to the page."""
            relay = WebAggApplication._relay
            if binary:
                _web.send_binary(relay, self.connection, bytes(message))
            else:
                _web.send_text(relay, self.connection, message)

        def close(self):
            """Close the WebSocket."""
            _web.close(WebAggApplication._relay, self.connection)

    @classmethod
    def initialize(cls, url_prefix='', port=None, address=None):
        if cls.initialized:
            return
        if url_prefix:
            assert url_prefix[0] == '/' and url_prefix[-1] != '/', \
                'url_prefix must start with a "/" and not end with one.'

        cls.url_prefix = url_prefix
        if address is None:
            cls.address = rcParams['webagg.address']
        else:
            cls.address = address
        relay = _web.bind(cls.address, rcParams['webagg.port'],
                          rcParams['webagg.port_retries'],
                          core.FigureManagerWebAgg.get_javascript(), url_prefix)
        if relay is None:
            raise SystemExit(
                "The webagg server could not be started because an available "
                "port could not be found")
        cls._relay = relay
        cls.port = _web.port(relay)
        cls.initialized = True

    @classmethod
    def start(cls):
        cls._publish()
        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            pass
        else:
            # Serve from the loop running (a notebook's kernel), as Tornado
            # serves from it.
            if not cls.started:
                cls.started = True
                loop.create_task(cls._serve())
            return

        if cls.started:
            return

        try:
            loop = asyncio.get_event_loop()
        except RuntimeError:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)

        def shutdown():
            cls.started = False
            print("Server is stopped")
            sys.stdout.flush()

        @contextmanager
        def catch_sigint():
            old_handler = signal.signal(
                signal.SIGINT,
                lambda sig, frame: loop.call_soon_threadsafe(shutdown))
            try:
                yield
            finally:
                signal.signal(signal.SIGINT, old_handler)

        # Set the flag to True *before* blocking on the loop.
        cls.started = True

        print("Press Ctrl+C to stop WebAgg server")
        sys.stdout.flush()
        with catch_sigint():
            loop.run_until_complete(cls._serve())

    @classmethod
    async def _serve(cls):
        """Hand what the clients send to the figures while the server runs."""
        while cls.started:
            cls._publish()
            for event in _web.poll(cls._relay, 0):
                cls._handle(event)
            await asyncio.sleep(_POLL_INTERVAL)

    @classmethod
    def _publish(cls):
        """Serve pages for the figures of `.Gcf`."""
        _web.set_figures(cls._relay, [
            (num, _is_dark(manager.canvas.figure))
            for num, manager in Gcf.figs.items() if isinstance(num, int)])

    @classmethod
    def _handle(cls, event):
        """Hand one event of the server to the WebSocket or figure it is
        for; a failing one is logged, as Tornado logs a handler's."""
        kind = event[0]
        try:
            if kind == "open":
                _, connection, fignum = event
                socket = cls._sockets[connection] = cls.WebSocket(connection)
                if Gcf.get_fig_manager(fignum) is None:
                    socket.close()
                else:
                    socket.open(fignum)
            elif kind == "message":
                _, connection, text = event
                socket = cls._sockets.get(connection)
                if socket is not None:
                    socket.on_message(text)
            elif kind == "close":
                socket = cls._sockets.pop(event[1], None)
                if socket is not None and hasattr(socket, "manager"):
                    socket.on_close()
            elif kind == "download":
                _, request, fignum, fmt = event
                _web.answer_download(cls._relay, request,
                                     cls._download(fignum, fmt))
        except Exception:
            _log.exception("Uncaught exception handling %s", kind)

    @staticmethod
    def _download(fignum, fmt):
        """Figure *fignum* saved in the format *fmt*, None when it is gone."""
        manager = Gcf.get_fig_manager(fignum)
        if manager is None:
            return None
        buff = BytesIO()
        manager.canvas.figure.savefig(buff, format=fmt)
        return buff.getvalue()


def ipython_inline_display(figure):
    """The HTML showing *figure* inline in a notebook (`.Figure.
    _repr_html_`), its WebSocket served by `WebAggApplication` from the
    kernel's event loop, which runs while the notebook waits for cells
    (pltrs (Rust) values live on the thread that made them, so there is no
    server thread)."""
    WebAggApplication.initialize()
    try:
        asyncio.get_running_loop()
    except RuntimeError:
        pass
    else:
        WebAggApplication.start()
    fignum = figure.number
    tpl = Path(core.FigureManagerWebAgg.get_static_file_path(),
               "ipython_inline_figure.html").read_text()
    for placeholder, value in [
            ("{{ port }}", str(WebAggApplication.port)),
            ("{{prefix}}", WebAggApplication.url_prefix),
            ("{{ prefix}}", WebAggApplication.url_prefix),
            ("{{ fig_id }}", str(fignum)),
            ("{{ repr(str(fig_id)) }}", repr(str(fignum))),
            ("{{repr(str(fig_id))}}", repr(str(fignum)))]:
        tpl = tpl.replace(placeholder, value)
    return tpl


@_Backend.export
class _BackendWebAgg(_Backend):
    FigureCanvas = FigureCanvasWebAgg
    FigureManager = FigureManagerWebAgg


# What matplotlib's module also has from its imports, which programs
# import from it.
import errno  # noqa: E402, F401
import mimetypes  # noqa: E402, F401
import random  # noqa: E402, F401
import threading  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
