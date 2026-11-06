"""
What the embedding programs share (``embed_qt.py``, ``embed_tk.py``,
``embed_wx.py``): the package they embed (pltrs, or real matplotlib with
``--matplotlib``, to compare), the figure and the animation, and the steps
every program takes, the toolkit's own timers spacing them.

Each program prints ``checkpoint <name>`` when its window is settled at a
step (and waits ``--hold`` seconds there, for a screenshot from outside),
then, once its window is closed, the facts it gathered as one line of
JSON: the sizes of the canvas and its widget at each step, the toolbar's
items, how often the figure was drawn and how many animation frames ran.
"""

import argparse
import importlib
import json

import numpy as np

#: The title of every program's window (screenshots find it by its title).
TITLE = "pltrs embedded"

#: The frames the blitting animation runs before the program goes on.
FRAMES = 12


def arguments():
    """The programs' command line: ``--matplotlib``, ``--hold SECONDS``."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matplotlib", action="store_true",
                        help="embed real matplotlib instead of pltrs")
    parser.add_argument("--hold", type=float, default=0.2,
                        help="seconds to stay at each checkpoint")
    return parser.parse_args()


def package_module(args, name):
    """The module *name* of the package the program embeds."""
    package = "matplotlib" if args.matplotlib else "pltrs"
    return importlib.import_module(f"{package}.{name}")


class Embedding:
    """
    The figure a program embeds and the steps it takes with it.

    *later(milliseconds, function)* is the toolkit's one-shot timer;
    *widget_size()* gives the canvas widget's size as the toolkit reports
    it, *click()* presses the program's Redraw button, *resize(width,
    height)* resizes its window and *close()* closes it.
    """

    def __init__(self, args, later):
        self._args = args
        self._later = later
        self.facts = {"steps": {}, "draws": 0, "frames": 0, "redraws": 0}
        self.figure = package_module(args, "figure").Figure(figsize=(5, 3), dpi=100)
        self._animation = package_module(args, "animation")
        self.axes = self.figure.add_subplot()
        self._x = np.linspace(0, 2 * np.pi, 200)
        (self.line,) = self.axes.plot(self._x, np.sin(self._x))
        (self.dot,) = self.axes.plot([0], [0], "o", animated=True)
        self.axes.set_title("embedded")
        self.canvas = None
        self.toolkit = {}
        self._movie = None

    def connect(self, canvas, toolbar_items, **toolkit):
        """Start counting *canvas*'s draws; *toolbar_items* lists the
        toolbar's items, *toolkit* holds ``widget_size``, ``click``,
        ``resize`` and ``close``."""
        self.canvas = canvas
        self.toolkit = toolkit
        self.facts["toolbar"] = toolbar_items
        canvas.mpl_connect("draw_event", self._drawn)

    def _drawn(self, event):
        self.facts["draws"] += 1

    def redraw(self):
        """What the Redraw button does: new data, drawn when idle."""
        self.facts["redraws"] += 1
        self.line.set_ydata(np.sin(self._x + self.facts["redraws"]))
        self.canvas.draw_idle()

    def start(self):
        """Take the steps, the first once the window has settled."""
        self._later(500, self._shown)

    def _record(self, step):
        canvas = self.canvas
        self.facts["steps"][step] = {
            "widget": list(self.toolkit["widget_size"]()),
            "canvas": list(canvas.get_width_height()),
            "physical": list(canvas.get_width_height(physical=True)),
            "device_pixel_ratio": canvas.device_pixel_ratio,
            "figure_pixels": [round(v) for v in self.figure.bbox.size],
            "dpi": self.figure.dpi,
            "draws": self.facts["draws"],
        }
        print(f"checkpoint {step}", flush=True)

    def _then(self, function):
        self._later(round(self._args.hold * 1000) + 300, function)

    def _shown(self):
        self._record("shown")
        self._then(self._click)

    def _click(self):
        self.toolkit["click"]()
        self._later(300, self._clicked)

    def _clicked(self):
        self._record("redrawn")
        self.facts["ydata_changed"] = bool(
            not np.allclose(self.line.get_ydata(), np.sin(self._x)))
        self._then(self._animate)

    def _animate(self):
        self._movie = self._animation.FuncAnimation(
            self.figure, self._frame, frames=FRAMES, interval=30, blit=True,
            repeat=False, cache_frame_data=False)
        self._later(FRAMES * 30 + 1500, self._animated)

    def _frame(self, i):
        self.facts["frames"] += 1
        angle = 2 * np.pi * i / FRAMES
        self.dot.set_data([angle], [np.sin(angle)])
        return (self.dot,)

    def _animated(self):
        self._record("animated")
        self._then(self._resize)

    def _resize(self):
        self.toolkit["resize"](720, 540)
        self._later(800, self._resized)

    def _resized(self):
        self._record("resized")
        self._then(self.toolkit["close"])

    def report(self):
        """Print the facts, once the window is closed."""
        print(json.dumps(self.facts, sort_keys=True), flush=True)
