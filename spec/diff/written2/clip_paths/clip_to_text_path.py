# A gradient image seen only through the outline of a word: TextPath placed with a transform as the clip path, and a PathPatch outline.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import PathPatch
from matplotlib.textpath import TextPath
from matplotlib.transforms import Affine2D
from matplotlib.font_manager import FontProperties
plt.rcParams["svg.hashsalt"] = "written2"

fig, ax = plt.subplots(figsize=(6, 2.5))
ax.set_xlim(0, 10)
ax.set_ylim(0, 4)
word = TextPath((0, 0), "CLIP", size=1, prop=FontProperties(family="DejaVu Sans", weight="bold"))
ext = word.get_extents()
scale = 8 / ext.width
placed = Affine2D().translate(-ext.x0, -ext.y0).scale(scale).translate(1, 0.5)
patch = PathPatch(word, transform=placed + ax.transData, fc="none", ec="k", lw=1)
ax.add_patch(patch)
grad = np.outer(np.linspace(0, 1, 50), np.linspace(1, 0, 200))
im = ax.imshow(grad, extent=(0, 10, 0, 4), cmap="plasma", aspect="auto")
im.set_clip_path(patch)
fig.canvas.draw()
print(round(scale, 4), [round(v, 3) for v in ext.bounds])
print([round(v, 2) for v in patch.get_window_extent().bounds])
print([round(v, 2) for v in patch.get_extents().bounds])
buf = np.asarray(fig.canvas.buffer_rgba())
colored = (buf[..., 0] != buf[..., 1]) | (buf[..., 1] != buf[..., 2])
print(int(colored.sum()) > 1000, buf.shape)
fig.savefig("text_clip.png")
fig.savefig("text_clip.svg", metadata={"Date": None})
print(len(word.vertices), word.codes is not None)
