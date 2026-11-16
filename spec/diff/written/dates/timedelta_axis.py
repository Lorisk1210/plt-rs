# Lap times as durations: convert timedelta to minutes for plotting with a mm:ss formatter.
import datetime as dt
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

laps = [dt.timedelta(minutes=1, seconds=s) for s in (32.4, 30.1, 29.8, 31.0, 29.2, 28.9, 30.5)]
seconds = [lap.total_seconds() for lap in laps]
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.plot(range(1, len(laps) + 1), seconds, "s-")
ax.yaxis.set_major_formatter(FuncFormatter(lambda s, _: f"{int(s // 60)}:{s % 60:04.1f}"))
ax.set_xlabel("lap")
ax.set_ylabel("lap time")
best = min(range(len(laps)), key=lambda i: laps[i])
ax.annotate("best", (best + 1, seconds[best]), xytext=(best + 1.5, seconds[best] + 1),
            arrowprops=dict(arrowstyle="->"))
fig.canvas.draw()
print([t.get_text() for t in ax.get_yticklabels()])
fig.savefig("laps.png")
