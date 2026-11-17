# Clip an image and a contour set to a hand-made polygon patch, then a TransformedPath with an Affine2D, and read back the clip state.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Polygon
from matplotlib.transforms import Affine2D, TransformedPath
from matplotlib.path import Path

x = np.linspace(-3, 3, 120)
X, Y = np.meshgrid(x, x)
Z = np.sin(X) * np.cos(Y)
fig, (a1, a2) = plt.subplots(1, 2, figsize=(7, 3.5))
im = a1.imshow(Z, extent=(-3, 3, -3, 3), origin="lower", cmap="viridis")
hexagon = Polygon([(2.5 * np.cos(t), 2.5 * np.sin(t)) for t in np.linspace(0, 2 * np.pi, 7)[:-1]],
                  closed=True, transform=a1.transData, fc="none", ec="k")
a1.add_patch(hexagon)
im.set_clip_path(hexagon)
print(type(im.get_clip_path()).__name__, im.get_clip_on(), im.get_clip_box() is a1.bbox)
cs = a2.contourf(X, Y, Z, levels=8, cmap="coolwarm")
tri = Path([(-3, -3), (3, -3), (0, 3), (-3, -3)], closed=True)
tp = TransformedPath(tri, Affine2D().scale(0.8) + a2.transData)
cs.set_clip_path(tp)
print(type(cs.get_clip_path()).__name__)
path, affine = cs.get_clip_path().get_transformed_path_and_affine()
print(len(path.vertices), np.round(affine.get_matrix(), 3).tolist())
fig.canvas.draw()
rgba = np.asarray(fig.canvas.buffer_rgba())
print(rgba.shape, int((rgba[..., :3].sum(axis=2) == 765).sum()))
im.set_clip_path(None)
print(im.get_clip_path(), im.get_clip_box() is a1.bbox)
fig.savefig("clipped.png")
im.set_clip_on(False)
fig.canvas.draw()
print(int((np.asarray(fig.canvas.buffer_rgba())[..., :3].sum(axis=2) == 765).sum()))
