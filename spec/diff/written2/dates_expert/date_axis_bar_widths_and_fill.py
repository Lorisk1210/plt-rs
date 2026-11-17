# Bars, fill_betweenx, axvspan and errorbars on date axes with day/hour widths; ConciseDateConverter registered for datetime.
import datetime
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
import matplotlib.units as munits

converter = mdates.ConciseDateConverter()
munits.registry[datetime.date] = converter
munits.registry[datetime.datetime] = converter
munits.registry[np.datetime64] = converter
days = [datetime.date(2024, 7, d) for d in range(1, 11)]
fig, (a1, a2) = plt.subplots(2, 1, figsize=(6, 4), dpi=60)
bars = a1.bar(days, np.arange(10) % 4 + 1, width=0.8)
a1.axvspan(days[2], days[4], alpha=0.2)
a1.errorbar(days[::3], [2, 3, 2, 1], xerr=datetime.timedelta(hours=12), fmt="k.")
hours = np.datetime64("2024-07-01T00") + np.arange(24).astype("timedelta64[h]")
fb = a2.fill_betweenx(np.arange(24), hours, hours + np.timedelta64(90, "m"), alpha=0.4)
fig.canvas.draw()
print([round(b.get_x(), 3) for b in bars[:2]], round(bars[0].get_width(), 3))
print([t.get_text() for t in a1.get_xticklabels()], a1.xaxis.get_offset_text().get_text())
print([t.get_text() for t in a2.get_xticklabels()][:4], a2.xaxis.get_offset_text().get_text())
print(type(a1.xaxis.get_major_formatter()).__name__, type(a2.xaxis.get_converter()).__name__)
print([round(v, 3) for v in a2.get_xlim()])
for t in (datetime.date, datetime.datetime, np.datetime64):
    munits.registry[t] = mdates._SwitchableDateConverter()
fig.savefig("date_bars.png")
print(type(munits.registry[datetime.date]).__name__)
