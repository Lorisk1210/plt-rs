# Replace clip paths repeatedly: patch, then (Path, Transform), then a Rectangle, then None; check stale flags and the clip box.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle
from matplotlib.path import Path
from matplotlib.transforms import IdentityTransform

fig, ax = plt.subplots(figsize=(3, 3))
ln, = ax.plot([0, 1], [0, 1], lw=5)
fig.canvas.draw()
print(ln.stale, ln.get_clip_box() is ax.bbox, ln.get_clip_on())
log = []
ln.add_callback(lambda a: log.append("changed"))
ln.set_clip_path(Circle((0.5, 0.5), 0.3, transform=ax.transAxes))
print(ln.stale, type(ln.get_clip_path()).__name__, len(log))
fig.canvas.draw()
ln.set_clip_path(Path.unit_rectangle(), IdentityTransform())
print(ln.stale, type(ln.get_clip_path()).__name__, len(log))
ln.set_clip_path(Rectangle((0.1, 0.1), 0.5, 0.5, transform=ax.transAxes))
print(type(ln.get_clip_path()).__name__, ln.get_clip_box())
ln.set_clip_path(Rectangle((0, 0), 1, 1, transform=ax.transAxes))
print(type(ln.get_clip_path()).__name__, ln.get_clip_box() is not None)
ln.set_clip_path(None)
print(ln.get_clip_path(), ln.get_clip_box() is ax.bbox, len(log))
try:
    ln.set_clip_path(Path.unit_rectangle())
except TypeError as e:
    print("TypeError:", e)
try:
    ln.set_clip_path("circle")
except TypeError as e:
    print("TypeError:", e)
fig.savefig("reclip.png")
