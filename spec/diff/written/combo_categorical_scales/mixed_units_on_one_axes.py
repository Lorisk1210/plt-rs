# Datetime x with a twin category y and a secondary axis converting dates to day numbers.
import datetime as dt
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

start = dt.datetime(2024, 1, 1)
times = [start + dt.timedelta(days=int(d)) for d in np.arange(0, 60, 3)]
fig, ax = plt.subplots(figsize=(9, 4))
ax.plot(times, np.sqrt(np.arange(len(times))), "o-")
sec = ax.secondary_xaxis("top", functions=(lambda n: n - mdates.date2num(start), lambda d: d + mdates.date2num(start)))
sec.set_xlabel("day number")
ax2 = ax.twinx()
ax2.plot(times[::4], ["A", "B", "A", "C", "B"], "rs")
fig.autofmt_xdate()
fig.canvas.draw()
print([t.get_text() for t in sec.xaxis.get_ticklabels()][:4], [t.get_text() for t in ax2.get_yticklabels()])
fig.savefig("mixed_units.png", dpi=50)
