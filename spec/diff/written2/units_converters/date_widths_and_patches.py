# Unit-aware geometry with dates: bar widths as timedelta, Rectangle at a date, axvspan/axhline, fill_between and convert_xunits.
import datetime
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from matplotlib.patches import Rectangle

start = datetime.datetime(2024, 5, 1)
days = [start + datetime.timedelta(days=k) for k in range(7)]
fig, ax = plt.subplots(figsize=(6, 3))
bars = ax.bar(days, np.arange(1, 8), width=datetime.timedelta(hours=12), align="edge")
print([round(b.get_width(), 4) for b in bars[:2]], round(bars[0].get_x(), 4))
rect = ax.add_patch(Rectangle((days[2], 2), datetime.timedelta(days=2), 3, fc="none", ec="r"))
print(type(rect.get_x()).__name__, rect.get_width())
span = ax.axvspan(days[4], days[5], alpha=0.2)
line = ax.axvline(days[6], color="k")
fb = ax.fill_between(days, 0, np.linspace(0, 3, 7), alpha=0.3)
fig.canvas.draw()
print([round(v, 4) for v in ax.get_xlim()], type(span).__name__, [round(v, 4) for v in span.get_bbox().bounds])
print(type(line.get_xdata()[0]).__name__, [round(v, 4) for v in line.get_xydata()[0]])
print(round(ax.convert_xunits(days[3]), 4), ax.convert_xunits(3.0))
print(np.round(ax.convert_xunits(np.array(days[:2], dtype="datetime64[s]")), 4).tolist())
print(round(mdates.date2num(days[0]), 4), mdates.num2date(mdates.date2num(days[0])).isoformat())
print([t.get_text() for t in ax.get_xticklabels()][:3])
ax.set_xlim(days[1], days[5])
fig.canvas.draw()
print([round(v, 4) for v in ax.get_xlim()], [mdates.num2date(v).strftime("%m-%d") for v in ax.get_xlim()])
eb = ax.errorbar(days[:3], [1, 2, 3], xerr=datetime.timedelta(hours=6), fmt="o")
fig.canvas.draw()
print(len(eb.lines[2]), np.round(eb.lines[2][0].get_segments()[0], 4).tolist())
fig.savefig("date_widths.png")
