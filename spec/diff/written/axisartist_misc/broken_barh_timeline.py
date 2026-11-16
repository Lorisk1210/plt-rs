# Timeline of releases with event markers, vertical stems and alternating label positions.
import datetime as dt
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

releases = [("v1.0", dt.date(2023, 1, 15)), ("v1.1", dt.date(2023, 4, 2)), ("v1.2", dt.date(2023, 6, 20)),
            ("v2.0", dt.date(2023, 10, 1)), ("v2.1", dt.date(2024, 1, 10)), ("v2.2", dt.date(2024, 4, 5))]
names, dates = zip(*releases)
levels = [1.5 if i % 2 == 0 else -1.5 for i in range(len(dates))]
fig, ax = plt.subplots(figsize=(8, 3), layout="constrained")
ax.vlines(dates, 0, levels, color="tab:red")
ax.axhline(0, c="black")
ax.plot(dates, [0] * len(dates), "ko", mfc="white")
for d, l, n in zip(dates, levels, names):
    ax.annotate(n, xy=(d, l), xytext=(-3, 3 if l > 0 else -3), textcoords="offset points",
                ha="right", va="bottom" if l > 0 else "top")
ax.xaxis.set_major_locator(mdates.MonthLocator(interval=3))
ax.xaxis.set_major_formatter(mdates.DateFormatter("%b %Y"))
ax.yaxis.set_visible(False)
ax.spines[["left", "top", "right"]].set_visible(False)
ax.margins(y=0.2)
fig.savefig("releases.png")
print(len(names))
