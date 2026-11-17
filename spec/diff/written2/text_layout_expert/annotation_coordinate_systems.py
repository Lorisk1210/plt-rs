# annotate with every coordinate system for xy and xytext (data, axes fraction, figure points, offset points, polar, callables, artists).
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.transforms import Bbox

fig, ax = plt.subplots(figsize=(5, 4), dpi=80)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
box = ax.text(2, 8, "anchor", bbox=dict(fc="w"))
specs = [
    dict(xy=(5, 5), xycoords="data", xytext=(20, 20), textcoords="offset points"),
    dict(xy=(0.5, 0.1), xycoords="axes fraction", xytext=(0.8, 0.3), textcoords="figure fraction"),
    dict(xy=(100, 100), xycoords="figure pixels", xytext=(-30, 10), textcoords="offset pixels"),
    dict(xy=(1, 0.5), xycoords=box, xytext=(10, 0), textcoords="offset fontsize"),
    dict(xy=(0.5, 1), xycoords=ax.title, xytext=(0, 5), textcoords="offset points"),
    dict(xy=(7, 2), xycoords=("data", "axes fraction"), xytext=(0.5, 0.5), textcoords="axes fraction"),
    dict(xy=(0, 0), xycoords=lambda r: Bbox.from_bounds(50, 50, 10, 10), xytext=(5, 5), textcoords="offset points"),
    dict(xy=(1, 1), xycoords=ax.transData, xytext=(2, 2), textcoords=ax.transData),
    dict(xy=(20, 20), xycoords="figure points", xytext=(30, 30), textcoords="axes points"),
]
anns = [ax.annotate(f"a{k}", arrowprops=dict(arrowstyle="->"), **s) for k, s in enumerate(specs)]
ax.set_title("title")
fig.canvas.draw()
r = fig.canvas.get_renderer()
for k, a in enumerate(anns):
    print(k, [round(v, 1) for v in a.get_window_extent(r).bounds], np.round(a._get_xy_display(), 1).tolist())
ax.set_xlim(0, 20)
fig.canvas.draw()
print([round(v, 1) for v in anns[0].get_window_extent(r).bounds], [round(v, 1) for v in anns[5].get_window_extent(r).bounds])
print(anns[0].anncoords, anns[0].xycoords, anns[1].get_anncoords())
anns[0].set_anncoords("axes fraction")
anns[0].xyann = (0.9, 0.9)
fig.canvas.draw()
print([round(v, 1) for v in anns[0].get_window_extent(r).bounds])
fig.savefig("coords.png")
