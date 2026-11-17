# quiverkey in every coordinate system and label position, with fontproperties and labelsep; extents of keys and labels.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4), dpi=60)
X, Y = np.meshgrid(np.arange(5), np.arange(4))
q = ax.quiver(X, Y, np.cos(X), np.sin(Y))
keys = []
for k, (coords, pos, xy) in enumerate([("axes", "N", (0.2, 1.05)), ("figure", "S", (0.8, 0.05)), ("data", "E", (2, 2)),
                                        ("inches", "W", (1, 1))]):
    keys.append(ax.quiverkey(q, *xy, 1 + k, f"{1 + k} u", labelpos=pos, coordinates=coords, labelsep=0.05 * (k + 1),
                             fontproperties={"size": 8 + k}, labelcolor=f"C{k}", color="k"))
fig.canvas.draw()
r = fig.canvas.get_renderer()
for kq in keys:
    print(kq.coord, kq.labelpos, [round(v, 1) for v in kq.get_window_extent(r).bounds][:2],
          [round(v, 1) for v in kq.text.get_window_extent(r).bounds], kq.text.get_color(), kq.text.get_fontsize())
print(len(ax.artists) + len(ax.texts) + len(ax.collections), keys[0].Q is q, keys[2].vector is not None)
keys[0].text.set_text("changed")
fig.canvas.draw()
print(keys[0].text.get_text(), [round(v, 1) for v in keys[0].text.get_window_extent(r).bounds][2:])
fig.savefig("quiverkeys.png")
keys[1].remove()
print(sum(isinstance(a, type(keys[0])) for a in ax.get_children()))
keys[2].set_visible(False)
fig.canvas.draw()
print(keys[2].get_visible(), keys[3].get_zorder(), q.get_zorder())
qk_ax = ax.quiverkey(q, 0.5, 0.5, 2, r"$2\,\frac{m}{s}$", coordinates="axes", angle=45)
fig.canvas.draw()
print(qk_ax.angle, qk_ax.text.get_text(), [round(v, 1) for v in qk_ax.text.get_window_extent(r).bounds][2:])
