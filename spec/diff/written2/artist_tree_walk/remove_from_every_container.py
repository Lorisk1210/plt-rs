# remove() artists of every kind (lines, patches, collections, images, texts, tables, legends, colorbars, containers) and check lists.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
ln, = ax.plot([0, 1])
patch = ax.add_patch(plt.Rectangle((0, 0), 0.2, 0.2))
sc = ax.scatter([0.5], [0.5])
im = ax.imshow(np.eye(2), extent=(0.6, 0.8, 0.6, 0.8))
txt = ax.text(0.1, 0.9, "t")
tab = ax.table(cellText=[["a", "b"]], loc="bottom")
bars = ax.bar([0.3, 0.4], [0.1, 0.2], width=0.05)
eb = ax.errorbar([0.7], [0.3], yerr=0.1)
leg = ax.legend(["l"])
cb = fig.colorbar(im, ax=ax)


def counts():
    return (len(ax.lines), len(ax.patches), len(ax.collections), len(ax.images), len(ax.texts), len(ax.tables),
            len(ax.containers), ax.get_legend() is not None, len(fig.axes))


print("start", counts())
for name, art in [("line", ln), ("patch", patch), ("scatter", sc), ("text", txt), ("table", tab), ("legend", leg),
                  ("bars", bars), ("errorbar", eb), ("colorbar", cb), ("image", im)]:
    art.remove()
    print(name, counts())
try:
    ln.remove()
except Exception as e:
    print("again", type(e).__name__, str(e)[:50])
print(ln.axes, ln.get_figure(root=False))
fig.canvas.draw()
fig.savefig("removed.png")
ax.remove()
print(len(fig.axes), ax in fig.axes)
