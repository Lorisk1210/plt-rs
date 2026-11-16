# Hourly sensor readings over ten days with ConciseDateFormatter and AutoDateLocator.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from datetime import datetime, timedelta

start = datetime(2024, 5, 1)
times = [start + timedelta(hours=h) for h in range(240)]
values = 20 + 5 * np.sin(np.arange(240) * 2 * np.pi / 24)
fig, ax = plt.subplots(figsize=(8, 3.5), layout="constrained")
ax.plot(times, values)
locator = mdates.AutoDateLocator(minticks=4, maxticks=10)
ax.xaxis.set_major_locator(locator)
ax.xaxis.set_major_formatter(mdates.ConciseDateFormatter(locator))
ax.set_ylabel("°C")
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
print(ax.xaxis.get_offset_text().get_text())
fig.savefig("hourly.png")
