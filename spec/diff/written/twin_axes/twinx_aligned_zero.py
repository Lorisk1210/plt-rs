# Align zero on both y axes of a twinx plot (profit vs margin with negative values).
import numpy as np
import matplotlib.pyplot as plt

quarters = np.arange(8)
profit = np.array([-20, -5, 10, 25, 30, 18, 40, 55])
margin = np.array([-2.0, -0.5, 1.2, 3.0, 3.1, 2.0, 4.2, 5.5])
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.bar(quarters, profit, color=np.where(profit < 0, "salmon", "seagreen"))
ax2 = ax.twinx()
ax2.plot(quarters, margin, "k.-")
lo1, hi1 = ax.get_ylim()
lo2, hi2 = ax2.get_ylim()
ratio = max(-lo1 / hi1, -lo2 / hi2)
ax.set_ylim(-ratio * hi1, hi1)
ax2.set_ylim(-ratio * hi2, hi2)
ax.axhline(0, color="black", lw=0.8)
print(np.round(ax.get_ylim(), 3), np.round(ax2.get_ylim(), 3))
fig.savefig("aligned.png")
