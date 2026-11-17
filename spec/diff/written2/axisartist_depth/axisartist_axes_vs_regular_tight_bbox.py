# The same plot on a regular Axes and on axisartist Axes and SubplotZero: tight bboxes, tight_layout and savefig bbox_inches.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits import axisartist
from PIL import Image


def build(axes_class):
    fig = plt.figure(figsize=(4, 3), dpi=80)
    ax = fig.add_subplot(axes_class=axes_class) if axes_class else fig.add_subplot()
    ax.plot(np.linspace(0, 1, 10), np.linspace(0, 1, 10) ** 2)
    ax.set_xlabel("x label")
    ax.set_ylabel("y label")
    ax.set_title("title")
    return fig, ax


for name, cls in [("regular", None), ("axisartist", axisartist.Axes), ("zero", axisartist.axislines.AxesZero)]:
    fig, ax = build(cls)
    fig.canvas.draw()
    r = fig.canvas.get_renderer()
    print(name, [round(v, 1) for v in ax.get_tightbbox(r).bounds])
    fig.tight_layout()
    print(name, [round(v, 4) for v in ax.get_position().bounds])
    buf = io.BytesIO()
    fig.savefig(buf, format="png", bbox_inches="tight")
    buf.seek(0)
    with Image.open(buf) as im:
        print(name, im.size)
    plt.close(fig)
fig, ax = build(axisartist.Axes)
ax.axis["bottom"].label.set_text("axisartist label")
ax.axis["top"].set_visible(False)
fig.canvas.draw()
print(ax.get_xlabel(), ax.axis["bottom"].label.get_text())
print([round(v, 1) for v in ax.get_tightbbox(fig.canvas.get_renderer()).bounds])
