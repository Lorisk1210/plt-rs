# The order artists are drawn in: a renderer wrapper records draw calls by gid, for zorder ties, set_zorder and axisbelow.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backends.backend_agg import RendererAgg

fig, ax = plt.subplots(figsize=(3, 3))
ax.plot([0, 1], [0, 1], gid="line-a")
ax.bar([0.5], [1], gid="bar", width=0.3)
ax.text(0.5, 0.5, "t", gid="text")
ax.plot([0, 1], [1, 0], gid="line-b", zorder=1)
ax.scatter([0.2], [0.2], gid="scatter", zorder=2.5)
ax.grid(True)
ax.set_axisbelow(True)
order = []
original = {name: getattr(RendererAgg, name) for name in ("draw_path", "draw_text", "draw_path_collection", "draw_markers")}


def record(name):
    def wrapper(self, gc, *args, **kw):
        gid = gc.get_gid()
        if gid and (not order or order[-1] != gid):
            order.append(gid)
        return original[name](self, gc, *args, **kw)
    return wrapper


for name in original:
    setattr(RendererAgg, name, record(name))
try:
    fig.canvas.draw()
    print(order)
    order.clear()
    ax.lines[0].set_zorder(10)
    ax.texts[0].set_zorder(0.5)
    fig.canvas.draw()
    print(order)
    order.clear()
    ax.set_axisbelow("line")
    fig.canvas.draw()
    print(order, [a.get_zorder() for a in ax.get_children()[:6]])
finally:
    for name, fn in original.items():
        setattr(RendererAgg, name, fn)
fig.savefig("zorder.png")
print(sorted(set(a.get_zorder() for a in ax.get_children())))
