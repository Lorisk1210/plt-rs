# Print date axis values: limits as floats, num2date back to datetimes, tick labels, and converter types.
import datetime as dt
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

fig, ax = plt.subplots(figsize=(6, 3))
ax.plot([dt.date(2024, 3, 1), dt.date(2024, 3, 20)], [1, 2])
fig.canvas.draw()
lo, hi = ax.get_xlim()
print(round(lo, 4), round(hi, 4), mdates.num2date(lo).isoformat(), type(mdates.num2date(hi)).__name__)
print([t.get_text() for t in ax.get_xticklabels()])
print(type(ax.xaxis.get_major_locator()).__name__, type(ax.xaxis.get_major_formatter()).__name__)
print(type(ax.xaxis.converter).__name__, ax.xaxis.get_units())
print(mdates.date2num(dt.datetime(2024, 3, 10, 12)), mdates.get_epoch())
fig.savefig("dates.png", dpi=50)
