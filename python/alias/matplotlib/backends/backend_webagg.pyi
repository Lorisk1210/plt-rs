# The type stub of matplotlib.backends.backend_webagg: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from contextlib import contextmanager
import errno
from io import BytesIO
import json
import mimetypes
from pathlib import Path
import random
import sys
import signal
import threading
try:
    import tornado.web
    import tornado.ioloop
    import tornado.websocket
except ImportError as err:
    raise RuntimeError('The WebAgg backend requires Tornado.') from err
import matplotlib as mpl
from matplotlib.backend_bases import _Backend
from matplotlib._pylab_helpers import Gcf
from . import backend_webagg_core as core
from .backend_webagg_core import TimerAsyncio, TimerTornado
webagg_server_thread = threading.Thread(target=lambda: tornado.ioloop.IOLoop.instance().start())

class FigureManagerWebAgg(core.FigureManagerWebAgg):
    _toolbar2_class = core.NavigationToolbar2WebAgg

    @classmethod
    def pyplot_show(cls, *, block=None):
        ...

class FigureCanvasWebAgg(core.FigureCanvasWebAggCore):
    manager_class = FigureManagerWebAgg

class WebAggApplication(tornado.web.Application):
    initialized = False
    started = False

    class FavIcon(tornado.web.RequestHandler):

        def get(self):
            ...

    class SingleFigurePage(tornado.web.RequestHandler):

        def __init__(self, application, request, *, url_prefix='', **kwargs):
            self.url_prefix = url_prefix

        def get(self, fignum):
            ...

    class AllFiguresPage(tornado.web.RequestHandler):

        def __init__(self, application, request, *, url_prefix='', **kwargs):
            self.url_prefix = url_prefix

        def get(self):
            ...

    class MplJs(tornado.web.RequestHandler):

        def get(self):
            ...

    class Download(tornado.web.RequestHandler):

        def get(self, fignum, fmt):
            ...

    class WebSocket(tornado.websocket.WebSocketHandler):
        supports_binary = True

        def open(self, fignum):
            self.fignum = int(fignum)
            self.manager = Gcf.get_fig_manager(self.fignum)

        def on_close(self):
            ...

        def on_message(self, message):
            self.supports_binary = message['value']

        def send_json(self, content):
            ...

        def send_binary(self, blob):
            ...

    def __init__(self, url_prefix=''):
        ...

    @classmethod
    def initialize(cls, url_prefix='', port=None, address=None):
        cls.url_prefix = url_prefix
        cls.port = mpl.rcParams['webagg.port']
        cls.initialized = True
        cls.address = mpl.rcParams['webagg.address']
        cls.address = address
        cls.port = port

    @classmethod
    def start(cls):
        cls.started = True
        cls.started = True

def ipython_inline_display(figure):
    ...

@_Backend.export
class _BackendWebAgg(_Backend):
    FigureCanvas = FigureCanvasWebAgg
    FigureManager = FigureManagerWebAgg
__all__ = ['contextmanager', 'errno', 'BytesIO', 'json', 'mimetypes', 'Path', 'random', 'sys', 'signal', 'threading', 'mpl', 'Gcf', 'core', 'TimerAsyncio', 'TimerTornado', 'webagg_server_thread', 'FigureManagerWebAgg', 'FigureCanvasWebAgg', 'WebAggApplication', 'ipython_inline_display']
