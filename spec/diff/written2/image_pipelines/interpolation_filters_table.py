# Every interpolation filter upsampling a checkerboard: mean/std of the rendered region, filterrad and filternorm effects.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import image as mimage

board = (np.add.outer(np.arange(6), np.arange(6)) % 2).astype(float)
names = sorted(mimage.interpolations_names)
print(names)
fig, ax = plt.subplots(figsize=(2, 2), dpi=60)
ax.set_position([0, 0, 1, 1])
ax.set_axis_off()
im = ax.imshow(board, cmap="gray", vmin=0, vmax=1)
for name in names:
    im.set_interpolation(name)
    buf = io.BytesIO()
    fig.savefig(buf, format="png")
    buf.seek(0)
    img = plt.imread(buf)[..., 0]
    print(name, round(float(img.mean()), 4), round(float(img.std()), 4))
for rad in (0.5, 2.0, 4.0):
    im.set_interpolation("sinc")
    im.set_filterrad(rad)
    buf = io.BytesIO()
    fig.savefig(buf, format="png")
    buf.seek(0)
    print("filterrad", rad, round(float(plt.imread(buf)[..., 0].std()), 4), im.get_filterrad())
im.set_filternorm(False)
buf = io.BytesIO()
fig.savefig(buf, format="png")
buf.seek(0)
print("filternorm off", round(float(plt.imread(buf)[..., 0].mean()), 4), im.get_filternorm())
try:
    im.set_filterrad(-1)
except ValueError as e:
    print("ValueError", str(e)[:50])
try:
    im.set_interpolation("cubic-spline")
except ValueError as e:
    print("ValueError", str(e)[:50])
