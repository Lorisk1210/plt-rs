# Dates with time zones across a DST change: tz on locators/formatters, rcParams timezone, and labels in each zone.
import datetime
import zoneinfo
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

berlin = zoneinfo.ZoneInfo("Europe/Berlin")
utc = datetime.timezone.utc
times = [datetime.datetime(2024, 3, 30, 22, tzinfo=utc) + datetime.timedelta(hours=h) for h in range(10)]
fig, axs = plt.subplots(3, 1, figsize=(6, 5), dpi=60)
for ax, tz in zip(axs, (utc, berlin, zoneinfo.ZoneInfo("America/New_York"))):
    ax.plot(times, range(10))
    ax.xaxis.set_major_locator(mdates.HourLocator(interval=2, tz=tz))
    ax.xaxis.set_major_formatter(mdates.DateFormatter("%d %H:%M %Z", tz=tz))
fig.canvas.draw()
for ax in axs:
    print([t.get_text() for t in ax.get_xticklabels()])
print([mdates.num2date(mdates.date2num(t), tz=berlin).strftime("%H:%M %Z") for t in times[2:6]])
with plt.rc_context({"timezone": "Europe/Berlin"}):
    f2, a2 = plt.subplots(figsize=(6, 2), dpi=60)
    a2.plot(times, range(10))
    a2.xaxis.set_major_formatter(mdates.DateFormatter("%H:%M"))
    f2.canvas.draw()
    print([t.get_text() for t in a2.get_xticklabels()], str(mdates._get_tzinfo()))
naive = [datetime.datetime(2024, 3, 31, h) for h in range(4)]
print([round(v, 5) for v in mdates.date2num(naive)], [round(v, 5) for v in mdates.date2num([n.replace(tzinfo=berlin) for n in naive])])
print(mdates.num2date(19813.5, tz="Asia/Tokyo").isoformat())
fig.savefig("tz.png")
