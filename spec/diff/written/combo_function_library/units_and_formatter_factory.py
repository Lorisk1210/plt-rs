# Factory functions returning formatters for money, percent and duration, applied across several axes.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter, PercentFormatter


def money(symbol="€", scale=1e3, suffix="k"):
    return FuncFormatter(lambda v, _: f"{symbol}{v / scale:,.0f}{suffix}")


def duration():
    return FuncFormatter(lambda s, _: f"{int(s // 3600)}h{int(s % 3600 // 60):02d}")


fig, axs = plt.subplots(1, 3, figsize=(11, 3.2))
axs[0].plot([0, 1, 2], [20000, 45000, 90000])
axs[0].yaxis.set_major_formatter(money())
axs[1].plot([0, 1, 2], [0.1, 0.35, 0.6])
axs[1].yaxis.set_major_formatter(PercentFormatter(1, decimals=0))
axs[2].bar(["a", "b"], [5400, 9000])
axs[2].yaxis.set_major_formatter(duration())
fig.tight_layout()
fig.canvas.draw()
for ax in axs:
    print([t.get_text() for t in ax.get_yticklabels()][:4])
fig.savefig("formatters.png", dpi=50)
