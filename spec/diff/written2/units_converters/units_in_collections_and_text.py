# Units through scatter, fill_between, annotate, text, hlines/vlines and Line2D added by hand with categories and dates.
import datetime
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

d0 = datetime.date(2024, 1, 1)
dates = [d0 + datetime.timedelta(days=7 * k) for k in range(5)]
cats = ["north", "south", "east", "west", "north"]
fig, ax = plt.subplots(figsize=(6, 3))
sc = ax.scatter(dates, cats, s=[20, 40, 60, 80, 100])
ax.vlines(dates[1:3], "south", "east", colors="r")
ax.hlines("west", dates[0], dates[-1], colors="g")
ann = ax.annotate("peak", (dates[3], "west"), (dates[1], "north"), arrowprops=dict(arrowstyle="->"))
txt = ax.text(dates[2], "east", "text at units")
ln = Line2D(dates[:2], ["north", "south"], color="m")
ax.add_line(ln)
fig.canvas.draw()
print(np.round(sc.get_offsets(), 3).tolist())
print([t.get_text() for t in ax.get_yticklabels()], [round(v, 2) for v in ax.get_ylim()])
print(type(ann.xy[0]).__name__, [round(v, 1) for v in ann.get_window_extent().bounds][:2])
print([round(v, 1) for v in txt.get_window_extent().bounds][:2], type(txt.get_position()[0]).__name__)
print(np.round(ln.get_xydata(), 3).tolist())
print(len(ax.collections), [type(c).__name__ for c in ax.collections])
fig.savefig("units_collections.png")
sc.set_offsets(np.c_[[19750, 19760], [0, 1]])
fig.canvas.draw()
print(np.round(sc.get_offsets(), 2).tolist())
