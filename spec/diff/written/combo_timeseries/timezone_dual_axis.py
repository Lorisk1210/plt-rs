# Events plotted in UTC with a second top axis showing New York time labels, plus a categorical event type marker.
import datetime as dt
from zoneinfo import ZoneInfo
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

utc, ny = ZoneInfo("UTC"), ZoneInfo("America/New_York")
events = [(dt.datetime(2024, 3, 10, h, tzinfo=utc), kind) for h, kind in [(1, "deploy"), (4, "alert"), (7, "deploy"), (9, "alert"), (13, "rollback"), (18, "deploy")]]
kinds = sorted({k for _, k in events})
fig, ax = plt.subplots(figsize=(9, 3.5))
for t, k in events:
    ax.plot(t, kinds.index(k), "o", ms=10, color=f"C{kinds.index(k)}")
ax.set_yticks(range(len(kinds)), kinds)
ax.xaxis.set_major_locator(mdates.HourLocator(interval=3, tz=utc))
ax.xaxis.set_major_formatter(mdates.DateFormatter("%H:%M UTC", tz=utc))
top = ax.secondary_xaxis("top")
top.xaxis.set_major_locator(mdates.HourLocator(interval=3, tz=utc))
top.xaxis.set_major_formatter(mdates.DateFormatter("%H:%M NY", tz=ny))
ax.set_ylim(-0.5, len(kinds) - 0.5)
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()][:3], [t.get_text() for t in top.xaxis.get_ticklabels()][:3])
fig.savefig("timezones.png", dpi=50)
