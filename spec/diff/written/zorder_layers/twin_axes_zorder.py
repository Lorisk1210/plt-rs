# Twin axes draw on top of the host; users raise the host with zorder and hide its patch.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(10)
fig, ax = plt.subplots(figsize=(6, 4))
ax.plot(x, x ** 2, "o-", color="C0", lw=3, label="host line")
twin = ax.twinx()
twin.bar(x, 10 - x, color="C1", alpha=0.8, label="twin bars")
ax.set_zorder(twin.get_zorder() + 1)
ax.patch.set_visible(False)
print(ax.get_zorder(), twin.get_zorder(), ax.patch.get_visible())
fig.savefig("twin_order.png")
