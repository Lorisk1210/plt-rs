# Clip many lines to a circle given in axes coordinates; the clip moves with the axes when the figure is resized.
import numpy as np
import matplotlib
matplotlib.use("agg")
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.patches import Circle

fig = Figure(figsize=(4, 4), dpi=72)
FigureCanvasAgg(fig)
ax = fig.add_subplot()
clip = Circle((0.5, 0.5), 0.35, transform=ax.transAxes, fc="none", ec="r")
ax.add_patch(clip)
lines = []
for k in range(20):
    t = np.linspace(0, 1, 50)
    ln, = ax.plot(t, (t * (k + 1) / 10) % 1, lw=1)
    ln.set_clip_path(clip)
    lines.append(ln)


def ink(fig):
    fig.canvas.draw()
    a = np.asarray(fig.canvas.buffer_rgba())[..., :3].astype(int)
    return int((a.sum(axis=2) < 600).sum())


before = ink(fig)
tp, aff = lines[0].get_clip_path().get_transformed_path_and_affine()
print(len(tp.vertices), np.round(aff.transform([[0.5, 0.5]]), 2).tolist())
fig.set_size_inches(6, 3)
after = ink(fig)
tp, aff = lines[0].get_clip_path().get_transformed_path_and_affine()
print(np.round(aff.transform([[0.5, 0.5]]), 2).tolist(), before > 0, after > 0)
lines[0].set_clip_path(clip.get_path(), clip.get_transform())
print(type(lines[0].get_clip_path()).__name__)
for ln in lines[::2]:
    ln.set_clip_on(False)
print(sum(ln.get_clip_on() for ln in lines))
fig.savefig("circle_clip.png")
