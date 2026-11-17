# Dates on x and categories on y of one axes, then a twin with a custom unit; converters per axis and what update_units does.
import datetime
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

days = [datetime.date(2024, 3, d) for d in (1, 5, 9, 13)]
fig, ax = plt.subplots(figsize=(6, 3))
ax.plot(days, ["low", "mid", "high", "mid"], "o-")
fig.canvas.draw()
print(type(ax.xaxis.get_converter()).__name__, type(ax.yaxis.get_converter()).__name__)
print([t.get_text() for t in ax.get_yticklabels()], ax.yaxis.get_units()._mapping if ax.yaxis.get_units() else None)
print([t.get_text() for t in ax.get_xticklabels()][:3])
ax.plot([datetime.date(2024, 3, 20)], ["extreme"], "rs")
fig.canvas.draw()
print([t.get_text() for t in ax.get_yticklabels()], [round(v, 2) for v in ax.get_ylim()])
print([round(v, 2) for v in ax.get_xlim()])
try:
    ax.plot([1.5], [2.5], "k+")
    fig.canvas.draw()
    print("numbers accepted", [round(v, 2) for v in ax.get_xlim()])
except Exception as e:
    print(type(e).__name__, str(e)[:60])
tw = ax.twinx()
tw.plot(days, np.array([1, 2, 3, 4]) * 1.5)
fig.canvas.draw()
print(type(tw.xaxis.get_converter()).__name__, tw.yaxis.get_converter())
print(ax.xaxis.have_units(), ax.yaxis.have_units(), tw.yaxis.have_units())
ax.xaxis.update_units([np.datetime64("2024-03-02")])
print(type(ax.xaxis.get_units()).__name__ if ax.xaxis.get_units() is not None else None)
fig.savefig("mixed.png")
