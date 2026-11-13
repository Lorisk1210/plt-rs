# The type stub of matplotlib.backends.backend_nbagg: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from base64 import b64encode
import io
import json
import pathlib
import uuid
from ipykernel.comm import Comm
from IPython.display import display, Javascript, HTML
from matplotlib import is_interactive
from matplotlib._pylab_helpers import Gcf
from matplotlib.backend_bases import _Backend, CloseEvent, NavigationToolbar2
from .backend_webagg_core import FigureCanvasWebAggCore, FigureManagerWebAgg, NavigationToolbar2WebAgg
from .backend_webagg_core import TimerTornado, TimerAsyncio

def connection_info():
    ...
_FONT_AWESOME_CLASSES = {'home': 'fa fa-home', 'back': 'fa fa-arrow-left', 'forward': 'fa fa-arrow-right', 'zoom_to_rect': 'fa fa-square-o', 'move': 'fa fa-arrows', 'download': 'fa fa-floppy-o', None: None}

class NavigationIPy(NavigationToolbar2WebAgg):
    toolitems = [(text, tooltip_text, _FONT_AWESOME_CLASSES[image_file], name_of_method) for text, tooltip_text, image_file, name_of_method in NavigationToolbar2.toolitems + (('Download', 'Download plot', 'download', 'download'),) if image_file in _FONT_AWESOME_CLASSES]

class FigureManagerNbAgg(FigureManagerWebAgg):
    _toolbar2_class = ToolbarCls = NavigationIPy

    def __init__(self, canvas, num):
        self._shown = False

    @classmethod
    def create_with_canvas(cls, canvas_class, figure, num):
        ...

    def display_js(self):
        ...

    def show(self):
        self._shown = True

    def reshow(self):
        self._shown = False

    @property
    def connected(self):
        ...

    @classmethod
    def get_javascript(cls, stream=None):
        ...

    def _create_comm(self):
        ...

    def destroy(self):
        ...

    def clearup_closed(self):
        self.web_sockets = {socket for socket in self.web_sockets if socket.is_open()}

    def remove_comm(self, comm_id):
        self.web_sockets = {socket for socket in self.web_sockets if socket.comm.comm_id != comm_id}

class FigureCanvasNbAgg(FigureCanvasWebAggCore):
    manager_class = FigureManagerNbAgg

class CommSocket:

    def __init__(self, manager):
        self.supports_binary = None
        self.manager = manager
        self.uuid = str(uuid.uuid4())
        self._ext_close = False
        self.comm = Comm('matplotlib', data={'id': self.uuid})

    def is_open(self):
        ...

    def on_close(self):
        ...

    def send_json(self, content):
        ...

    def send_binary(self, blob):
        ...

    def on_message(self, message):
        self.supports_binary = message['value']

@_Backend.export
class _BackendNbAgg(_Backend):
    FigureCanvas = FigureCanvasNbAgg
    FigureManager = FigureManagerNbAgg
__all__ = ['b64encode', 'io', 'json', 'pathlib', 'uuid', 'Comm', 'display', 'Javascript', 'HTML', 'is_interactive', 'Gcf', 'CloseEvent', 'NavigationToolbar2', 'FigureCanvasWebAggCore', 'FigureManagerWebAgg', 'NavigationToolbar2WebAgg', 'TimerTornado', 'TimerAsyncio', 'connection_info', 'NavigationIPy', 'FigureManagerNbAgg', 'FigureCanvasNbAgg', 'CommSocket']
