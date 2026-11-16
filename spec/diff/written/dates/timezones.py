# Server events in UTC shown in Europe/Berlin time with a timezone-aware formatter.
import datetime as dt
from zoneinfo import ZoneInfo
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

utc = ZoneInfo("UTC")
berlin = ZoneInfo("Europe/Berlin")
events = [dt.datetime(2024, 3, 30, 20, 0, tzinfo=utc) + dt.timedelta(minutes=45 * k) for k in range(16)]
latency = [120, 135, 128, 160, 190, 175, 150, 140, 138, 142, 160, 170, 155, 150, 145, 130]
fig, ax = plt.subplots(figsize=(8, 3.5))
ax.plot(events, latency, "o-")
ax.xaxis.set_major_formatter(mdates.DateFormatter("%d %H:%M", tz=berlin))
ax.xaxis.set_major_locator(mdates.HourLocator(interval=2, tz=berlin))
ax.set_xlabel("Berlin time (DST change on 31 March)")
fig.autofmt_xdate()
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
fig.savefig("latency.png")
