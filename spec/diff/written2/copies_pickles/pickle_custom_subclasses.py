# Pickle figures holding user subclasses (a Line2D subclass, a custom Formatter, a custom Normalize) defined in this module.
import pickle
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.ticker import Formatter
from matplotlib.colors import Normalize


class TaggedLine(Line2D):
    def __init__(self, *args, tag="", **kw):
        super().__init__(*args, **kw)
        self.tag = tag


class PercentFormatter(Formatter):
    def __init__(self, digits):
        self.digits = digits

    def __call__(self, x, pos=None):
        return f"{100 * x:.{self.digits}f}%"


class SqrtNorm(Normalize):
    def __call__(self, value, clip=None):
        return np.sqrt(super().__call__(value, clip))


fig, ax = plt.subplots()
line = TaggedLine([0, 1], [0, 1], tag="special")
ax.add_line(line)
ax.yaxis.set_major_formatter(PercentFormatter(1))
im = ax.imshow(np.arange(9).reshape(3, 3), norm=SqrtNorm(0, 8), extent=(0, 1, 0, 1))
fig.canvas.draw()
blob = pickle.dumps(fig)
f2 = pickle.loads(blob)
a2 = f2.axes[0]
l2 = [l for l in a2.lines if isinstance(l, TaggedLine)][0]
print(type(l2).__name__, l2.tag, type(a2.yaxis.get_major_formatter()).__name__, a2.yaxis.get_major_formatter().digits)
print(type(a2.images[0].norm).__name__, round(float(a2.images[0].norm(4)), 4))
f2.canvas.draw()
print([t.get_text() for t in a2.get_yticklabels()][:3])
print(len(blob) > 1000, pickle.loads(pickle.dumps(line)).tag)
