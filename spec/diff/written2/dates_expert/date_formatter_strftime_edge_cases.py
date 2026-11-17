# DateFormatter with unusual strftime codes (%j, %U, %f, %%, %z, %-d), usetex off, and AutoDateFormatter scaled dict edits.
import datetime
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

d = datetime.datetime(2024, 2, 29, 7, 5, 9, 123456, tzinfo=datetime.timezone.utc)
n = mdates.date2num(d)
for code in ("%Y-%m-%d", "%j", "%U %W", "%f", "%%", "%z %Z", "%a %A %b %B", "%I %p", "%y%m%d", "%H:%M:%S.%f", "%c", "%x", "%X"):
    print(repr(code), repr(mdates.DateFormatter(code, tz=datetime.timezone.utc)(n)))
auto = mdates.AutoDateFormatter(mdates.AutoDateLocator(), defaultfmt="%Y")
print(sorted(auto.scaled.items())[:4])
auto.scaled[1 / 24] = "%H h"
auto.scaled[1.0] = lambda x, pos=None: "day:" + mdates.num2date(x).strftime("%d")
fig, ax = plt.subplots(figsize=(6, 2), dpi=60)
ax.xaxis.set_major_locator(auto._locator)
ax.xaxis.set_major_formatter(auto)
for span in (datetime.timedelta(hours=10), datetime.timedelta(days=10), datetime.timedelta(days=1000)):
    ax.set_xlim(d, d + span)
    fig.canvas.draw()
    print(span, [t.get_text() for t in ax.get_xticklabels()][:4])
df = mdates.DateFormatter("%b %d")
df.set_tzinfo(datetime.timezone(datetime.timedelta(hours=-12)))
print(df(n))
print(mdates.DateFormatter("%Y")(0), mdates.DateFormatter("%Y")(-1000))
fig.savefig("strftime.png")
print([mdates.DateFormatter("%H:%M")(n + k / 24) for k in range(3)])
print(mdates.DateFormatter("%Y-%m-%d").format_ticks([n, n + 1]))
print(mdates.DateFormatter("%d/%m", usetex=False)(n), mdates.DateFormatter("%d/%m", usetex=True)(n))
