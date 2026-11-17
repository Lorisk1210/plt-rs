# RRuleLocator with rrulewrapper (weekly on Mon/Thu, monthly by setpos), tick values over fixed spans and formatted labels.
import datetime
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

start = datetime.datetime(2024, 1, 1)
end = datetime.datetime(2024, 4, 1)
rules = {
    "mon-thu": mdates.rrulewrapper(mdates.WEEKLY, byweekday=[mdates.MO, mdates.TH], interval=2),
    "last weekday": mdates.rrulewrapper(mdates.MONTHLY, byweekday=[mdates.MO, mdates.TU, mdates.WE, mdates.TH, mdates.FR],
                                        bysetpos=-1),
    "every 10 days": mdates.rrulewrapper(mdates.DAILY, interval=10),
    "quarter hours": mdates.rrulewrapper(mdates.MINUTELY, byminute=[0, 15, 30, 45]),
}
for name, rule in rules.items():
    loc = mdates.RRuleLocator(rule)
    hi = end if name != "quarter hours" else start + datetime.timedelta(hours=2)
    vals = loc.tick_values(start, hi)
    print(name, len(vals), [mdates.num2date(v).strftime("%Y-%m-%d %H:%M %a") for v in vals[:4]])
fig, ax = plt.subplots(figsize=(6, 2), dpi=60)
ax.plot([start, end], [0, 1])
ax.xaxis.set_major_locator(mdates.RRuleLocator(rules["last weekday"]))
ax.xaxis.set_major_formatter(mdates.DateFormatter("%a %d %b"))
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
w = mdates.rrulewrapper(mdates.YEARLY, bymonth=6, bymonthday=15)
w.set(interval=2)
print([mdates.num2date(v).date().isoformat() for v in mdates.RRuleLocator(w).tick_values(datetime.datetime(2020, 1, 1),
                                                                                          datetime.datetime(2030, 1, 1))])
print(mdates.RRuleLocator.get_unit_generic(mdates.WEEKLY), mdates.RRuleLocator.get_unit_generic(mdates.HOURLY))
fig.savefig("rrule.png")
