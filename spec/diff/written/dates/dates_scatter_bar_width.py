# Bars on a date axis need a width in days; users also scatter with dates and set date limits as strings.
import datetime as dt
import numpy as np
import matplotlib.pyplot as plt

days = [dt.date(2024, 7, d) for d in range(1, 15)]
rain = [0, 2, 5, 0, 0, 12, 8, 0, 1, 0, 3, 7, 0, 0]
fig, (a1, a2) = plt.subplots(2, 1, figsize=(7, 5), sharex=True)
a1.bar(days, rain, width=0.8, color="steelblue")
a1.set_ylabel("mm")
a2.scatter(days, np.cumsum(rain), c=rain, cmap="Blues", edgecolors="k")
a2.set_ylabel("cumulative mm")
a2.set_xlim(np.datetime64("2024-06-30"), np.datetime64("2024-07-15"))
fig.autofmt_xdate()
print(a2.get_xlim())
fig.savefig("rain.png")
