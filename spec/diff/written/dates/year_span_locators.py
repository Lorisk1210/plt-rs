# Ten years of quarterly data: YearLocator majors, quarter minors, and a %Y formatter.
import datetime as dt
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

quarters = [dt.datetime(2014 + q // 4, 3 * (q % 4) + 1, 1) for q in range(40)]
gdp = [100 * 1.005 ** q for q in range(40)]
gdp[24:26] = [gdp[23] * 0.92, gdp[23] * 0.95]
fig, ax = plt.subplots(figsize=(8, 3.5))
ax.plot(quarters, gdp, drawstyle="steps-post")
ax.xaxis.set_major_locator(mdates.YearLocator(2))
ax.xaxis.set_minor_locator(mdates.MonthLocator(bymonth=[1, 4, 7, 10]))
ax.xaxis.set_major_formatter(mdates.DateFormatter("%Y"))
ax.set_ylabel("index")
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
fig.savefig("gdp.png")
