# Artist.properties() dict, Artist.update(dict) and set_* with aliases (lw, c, ls, mec).
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
line, = ax.plot([1, 2, 3], [2, 1, 3])
line.update({"color": "purple", "linewidth": 2.5})
line.set(lw=3, ls="--", marker="o", mec="k", c="orange")
props = line.properties()
print(sorted(k for k in props if k in ("color", "linewidth", "linestyle", "marker", "markeredgecolor")))
print(props["color"], props["linewidth"], props["linestyle"], props["markeredgecolor"])
fig.savefig("updated.png")
fig.savefig("properties_get_set_dict_small.png", dpi=40)
print(fig.get_size_inches().tolist(), len(fig.axes))
