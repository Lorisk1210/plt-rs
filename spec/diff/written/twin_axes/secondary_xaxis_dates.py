# Day-of-year axis at the bottom with a secondary top axis labelled by month names.
import numpy as np
import matplotlib.pyplot as plt

doy = np.arange(1, 366)
daylight = 12 + 4.5 * np.sin((doy - 80) / 365 * 2 * np.pi)
fig, ax = plt.subplots(figsize=(8, 3.5))
ax.plot(doy, daylight)
ax.set_xlabel("day of year")
ax.set_ylabel("hours of daylight")
ax.set_xlim(1, 365)
top = ax.secondary_xaxis("top")
starts = [1, 32, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335]
top.set_xticks(starts, ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"])
top.tick_params(labelsize=8)
print(len(top.get_xticks()))
fig.savefig("daylight.png")
