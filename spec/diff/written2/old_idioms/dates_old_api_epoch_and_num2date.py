# Old date idioms: date2num/num2date, datestr2num, drange, epoch2num replacements, set_epoch, and plot_date's successor.
import datetime
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

d = datetime.datetime(2024, 3, 15, 12, 30)
n = mdates.date2num(d)
print(round(n, 6), mdates.num2date(n).isoformat(), mdates.num2date(n, tz=datetime.timezone.utc).tzname())
print(np.round(mdates.datestr2num(["2024-01-01", "2024-01-02 06:00"]), 4).tolist())
r = mdates.drange(datetime.datetime(2024, 1, 1), datetime.datetime(2024, 1, 2), datetime.timedelta(hours=6))
print(np.round(r, 4).tolist())
print(mdates.get_epoch())
print(np.round(mdates.date2num(np.array(["2024-01-01T00", "2024-06-01T12"], dtype="datetime64[h]")), 4).tolist())
print(mdates.num2timedelta(1.5), mdates.num2timedelta([0.25, 2]))
fig, ax = plt.subplots(figsize=(5, 2.5))
days = mdates.drange(datetime.date(2024, 1, 1), datetime.date(2024, 1, 15), datetime.timedelta(days=1))
ax.plot(days, np.arange(len(days)), "o-")
ax.xaxis_date()
fig.autofmt_xdate()
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()][:3], [round(t.get_rotation(), 1) for t in ax.get_xticklabels()][:1])
print(type(ax.xaxis.get_major_locator()).__name__, type(ax.xaxis.get_major_formatter()).__name__)
ax.xaxis.set_major_locator(mdates.DayLocator(interval=3))
ax.xaxis.set_major_formatter(mdates.DateFormatter("%b %d"))
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
print(mdates.DateFormatter("%Y")(n), mdates.AutoDateFormatter(mdates.AutoDateLocator())(n))
fig.savefig("old_dates.png")
