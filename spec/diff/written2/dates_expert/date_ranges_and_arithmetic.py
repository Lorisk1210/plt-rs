# Date arithmetic through matplotlib.dates: num2timedelta, drange edge cases, datestr2num with defaults, and epoch changes.
import datetime
import numpy as np
import matplotlib.dates as mdates

print(mdates.num2timedelta(0.5), mdates.num2timedelta([1, 1.25, -0.5]))
r = mdates.drange(datetime.datetime(2024, 1, 1), datetime.datetime(2024, 1, 1, 3), datetime.timedelta(minutes=50))
print(len(r), [mdates.num2date(v).strftime("%H:%M") for v in r])
r2 = mdates.drange(datetime.date(2024, 1, 1), datetime.date(2024, 1, 10), datetime.timedelta(days=3))
print(len(r2), [mdates.num2date(v).date().isoformat() for v in r2])
print(np.round(mdates.datestr2num("March 3 2024 4pm"), 4), np.round(mdates.datestr2num("12:00", default=datetime.datetime(2000, 1, 1)), 4))
print(np.round(mdates.datestr2num(["2024-01-01", "2024-01-01T06:00"]), 4).tolist())
dt = datetime.datetime(2024, 5, 5, 5, 5, 5)
n = mdates.date2num(dt)
print(round(n, 8), mdates.num2date(n).replace(tzinfo=None) == dt)
print(mdates.num2date([n, n + 1.5])[1].isoformat())
d64 = np.datetime64("2024-05-05T05:05:05")
print(round(float(mdates.date2num(d64)), 8), mdates.date2num(d64) == n)
print([round(v, 4) for v in mdates.date2num([datetime.date(2024, 1, 1), datetime.datetime(2024, 1, 1, 12)])])
print(mdates.date2num(datetime.datetime(1, 1, 1)), mdates.num2date(-719162.0).isoformat())
try:
    mdates.num2date(1e9)
except (ValueError, OverflowError) as e:
    print(type(e).__name__, str(e)[:60])
print(mdates.num2date(2932896.0).isoformat())
for epoch_days in (0, 1, 365.25):
    print(epoch_days, mdates.num2date(epoch_days).isoformat())
print([d.isoformat() for d in mdates.num2date(np.array([0.0, 0.25]))])
print(round(mdates.date2num(datetime.datetime(2024, 1, 1, tzinfo=datetime.timezone(datetime.timedelta(hours=5)))), 6))
print(mdates.num2date(19723.75).strftime("%Y-%m-%d %H:%M"), round(mdates.date2num(datetime.date(2024, 1, 1)), 3))
