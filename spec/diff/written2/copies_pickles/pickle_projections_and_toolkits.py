# Pickle round trips of polar, 3-D, axes_grid1 and axisartist figures; draw the copies and compare a few properties.
import io
import pickle
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import make_axes_locatable
from mpl_toolkits import axisartist


def roundtrip(fig, tag):
    try:
        f2 = pickle.loads(pickle.dumps(fig))
    except Exception as e:
        print(tag, "pickle failed:", type(e).__name__, str(e)[:60])
        return None
    buf1, buf2 = io.BytesIO(), io.BytesIO()
    fig.savefig(buf1, format="png")
    f2.savefig(buf2, format="png")
    print(tag, [type(a).__name__ for a in f2.axes], buf1.getvalue() == buf2.getvalue())
    return f2


fig = plt.figure(figsize=(3, 3), dpi=50)
pax = fig.add_subplot(projection="polar")
pax.plot(np.linspace(0, 6, 20), np.linspace(0, 1, 20))
pax.set_rlim(0, 2)
f2 = roundtrip(fig, "polar")
print(f2.axes[0].get_rmax() if f2 else None)
fig = plt.figure(figsize=(3, 3), dpi=50)
ax3 = fig.add_subplot(projection="3d")
ax3.plot([0, 1], [0, 1], [0, 1])
ax3.view_init(20, 40)
f2 = roundtrip(fig, "3d")
print((f2.axes[0].elev, f2.axes[0].azim) if f2 else None)
fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
cax = make_axes_locatable(ax).append_axes("right", "5%", pad=0.05)
fig.colorbar(ax.imshow(np.eye(3)), cax=cax)
roundtrip(fig, "grid1")
fig = plt.figure(figsize=(3, 3), dpi=50)
aa = fig.add_subplot(axes_class=axisartist.Axes)
aa.axis["top"].set_visible(False)
f2 = roundtrip(fig, "axisartist")
print(f2.axes[0].axis["top"].get_visible() if f2 else None)
