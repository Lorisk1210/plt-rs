# 200,000 timestamps at one-minute spacing as datetime64 plotted with the concise formatter; ticks at several zoom levels.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

n = 200_000
t = np.datetime64("2023-01-01T00:00") + np.arange(n).astype("timedelta64[m]")
v = np.cumsum(np.sin(np.arange(n) / 500.0))
fig, ax = plt.subplots(figsize=(7, 3), dpi=70)
ax.plot(t, v, lw=0.5)
loc = mdates.AutoDateLocator()
ax.xaxis.set_major_locator(loc)
ax.xaxis.set_major_formatter(mdates.ConciseDateFormatter(loc))
for start, end in [(0, n), (0, 20_000), (1000, 2000), (1000, 1060)]:
    ax.set_xlim(t[start], t[end - 1])
    fig.canvas.draw()
    print(end - start, [tl.get_text() for tl in ax.get_xticklabels()][:4], ax.xaxis.get_offset_text().get_text())
print(type(loc.get_locator(t[0].astype(object), t[-1].astype(object))).__name__)
print(round(float(v.min()), 3), round(float(v.max()), 3))
fig.savefig("long_series.png")
ax.xaxis.set_major_formatter(mdates.AutoDateFormatter(loc))
ax.set_xlim(t[0], t[-1])
fig.canvas.draw()
print([tl.get_text() for tl in ax.get_xticklabels()][:3])
ax.set_xlim(t[5000], t[5000] + np.timedelta64(90, "s"))
fig.canvas.draw()
print([tl.get_text() for tl in ax.get_xticklabels()][:3], type(ax.xaxis.get_major_locator()).__name__)
mins = ax.xaxis.get_majorticklocs()
print(len(mins) > 1, round(float(mins[1] - mins[0]) * 86400, 3))
fig.savefig("long_series_zoom.png")
