# Year/Month/Week/Day/Hour/Minute/Second/Microsecond locators with by* arguments and intervals; minor tick counts per axis.
import datetime
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

cases = [
    (mdates.YearLocator(base=5, month=7, day=4), datetime.timedelta(days=365 * 30)),
    (mdates.MonthLocator(bymonth=[1, 4, 7, 10], bymonthday=15), datetime.timedelta(days=500)),
    (mdates.WeekdayLocator(byweekday=mdates.SU, interval=2), datetime.timedelta(days=60)),
    (mdates.DayLocator(bymonthday=[1, 15]), datetime.timedelta(days=90)),
    (mdates.HourLocator(byhour=[6, 18]), datetime.timedelta(days=3)),
    (mdates.MinuteLocator(byminute=range(0, 60, 20)), datetime.timedelta(hours=2)),
    (mdates.SecondLocator(bysecond=[0, 30]), datetime.timedelta(minutes=3)),
    (mdates.MicrosecondLocator(interval=250000), datetime.timedelta(seconds=2)),
]
start = datetime.datetime(2020, 3, 10, 8, 30)
fig, ax = plt.subplots(figsize=(7, 2), dpi=60)
for loc, span in cases:
    ax.set_xlim(start, start + span)
    ax.xaxis.set_major_locator(loc)
    ax.xaxis.set_major_formatter(mdates.AutoDateFormatter(loc))
    fig.canvas.draw()
    locs = ax.xaxis.get_majorticklocs()
    print(type(loc).__name__, len(locs), [mdates.num2date(v).strftime("%Y-%m-%d %H:%M:%S.%f")[:23] for v in locs[:2]])
ax.xaxis.set_major_locator(mdates.AutoDateLocator())
ax.xaxis.set_major_formatter(mdates.AutoDateFormatter(ax.xaxis.get_major_locator()))
ax.xaxis.set_minor_locator(mdates.DayLocator())
ax.set_xlim(start, start + datetime.timedelta(days=20))
fig.canvas.draw()
print(len(ax.xaxis.get_minorticklocs()))
print(mdates.YearLocator(4).base.step)
fig.savefig("locators.png")
