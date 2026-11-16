# Grouped bars for two metrics on two y scales (side-by-side bars on twin axes with offsets).
import numpy as np
import matplotlib.pyplot as plt

stores = ["North", "South", "East", "West"]
revenue = [120, 95, 140, 80]
customers = [3100, 2800, 3600, 1900]
x = np.arange(len(stores))
w = 0.38
fig, ax1 = plt.subplots(figsize=(6.5, 4))
b1 = ax1.bar(x - w / 2, revenue, w, color="tab:blue", label="revenue (k$)")
ax2 = ax1.twinx()
b2 = ax2.bar(x + w / 2, customers, w, color="tab:orange", label="customers")
ax1.set_xticks(x, stores)
ax1.set_ylabel("revenue (k$)")
ax2.set_ylabel("customers")
ax1.legend(handles=[b1, b2], loc="upper right")
ax1.bar_label(b1, fmt="%d")
ax2.bar_label(b2, fmt="{:,.0f}", fontsize=8)
print([r.get_height() for r in b2])
fig.savefig("stores.png")
