# Check which converters are registered for common types and plot datetime.time-of-day by hand.
import datetime as dt
import matplotlib.pyplot as plt
import matplotlib.units as munits
from matplotlib.ticker import FuncFormatter

print(sorted(t.__name__ for t in munits.registry if isinstance(t, type))[:8])
times = [dt.time(8, 30), dt.time(9, 15), dt.time(12, 0), dt.time(17, 45)]
minutes = [t.hour * 60 + t.minute for t in times]
fig, ax = plt.subplots(figsize=(5, 3))
ax.plot(minutes, [3, 5, 2, 4], "o-")
ax.xaxis.set_major_formatter(FuncFormatter(lambda m, _: f"{int(m // 60):02d}:{int(m % 60):02d}"))
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
fig.savefig("time_of_day.png")
