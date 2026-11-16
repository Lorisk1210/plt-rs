# A year of monthly figures with MonthLocator, DateFormatter('%b') and minor week ticks.
import datetime as dt
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

dates = [dt.date(2023, m, 15) for m in range(1, 13)]
spend = [1200, 980, 1100, 1300, 1250, 1600, 1750, 1700, 1400, 1300, 1500, 2100]
fig, ax = plt.subplots(figsize=(7, 3.5))
ax.bar(dates, spend, width=20, color="peru")
ax.xaxis.set_major_locator(mdates.MonthLocator())
ax.xaxis.set_major_formatter(mdates.DateFormatter("%b"))
ax.xaxis.set_minor_locator(mdates.WeekdayLocator(byweekday=mdates.MO))
ax.set_xlim(dt.date(2023, 1, 1), dt.date(2023, 12, 31))
ax.set_title("Monthly spend 2023")
print(mdates.date2num(dates[0]), ax.get_xlim())
fig.savefig("spend.png")
