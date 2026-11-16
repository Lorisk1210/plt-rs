# Plot numpy datetime64 arrays directly and shade weekends with axvspan.
import numpy as np
import matplotlib.pyplot as plt

days = np.arange("2024-02-01", "2024-03-01", dtype="datetime64[D]")
orders = 40 + 10 * np.cos(np.arange(days.size) / 3)
fig, ax = plt.subplots(figsize=(8, 3.5))
ax.plot(days, orders, marker=".")
weekday = (days.astype("datetime64[D]").view("int64") - 4) % 7
for d, wd in zip(days, weekday):
    if wd >= 5:
        ax.axvspan(d, d + np.timedelta64(1, "D"), color="0.9", zorder=0)
ax.set_title("Orders in February 2024")
fig.autofmt_xdate()
print(days[0], days[-1], int((weekday >= 5).sum()))
fig.savefig("orders.png")
