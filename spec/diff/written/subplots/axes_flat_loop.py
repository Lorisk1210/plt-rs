# Loop over axes.flat to draw a small multiple per city and delete the unused last panel.
import numpy as np
import matplotlib.pyplot as plt

cities = ["Oslo", "Rome", "Cairo", "Lima", "Tokyo"]
base = [5, 16, 22, 19, 15]
amp = [10, 8, 7, 3, 10]
months = np.arange(1, 13)
fig, axes = plt.subplots(2, 3, figsize=(9, 5), sharey=True)
for ax, city, b, a in zip(axes.flat, cities, base, amp):
    temps = b + a * np.sin((months - 4) / 12 * 2 * np.pi)
    ax.bar(months, temps, color=np.where(temps > 15, "tomato", "steelblue"))
    ax.set_title(city)
    ax.set_xticks(months[::2])
fig.delaxes(axes.flat[-1])
fig.suptitle("Average monthly temperature")
fig.tight_layout()
print(len(fig.axes))
fig.savefig("cities.png")
