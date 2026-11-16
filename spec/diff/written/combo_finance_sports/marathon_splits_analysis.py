# Marathon splits: pace per km with a moving average, cumulative time vs target line, and split distribution histogram.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

rng = np.random.default_rng(163)
km = np.arange(1, 43)
pace = 285 + np.clip(km - 30, 0, None) * 4 + rng.normal(0, 6, 42)
mmss = FuncFormatter(lambda s, _: f"{int(s // 60)}:{int(s % 60):02d}")
fig, axs = plt.subplots(1, 3, figsize=(13, 4))
axs[0].bar(km, pace, color=np.where(pace > 300, "C3", "C0"))
axs[0].plot(km, np.convolve(pace, np.ones(5) / 5, mode="same"), "k")
axs[0].yaxis.set_major_formatter(mmss)
axs[0].set_ylim(240, 360)
cum = np.cumsum(pace)
axs[1].plot(km, cum / 3600, label="actual")
axs[1].plot(km, km * 285 / 3600, "--", label="3:20 target")
axs[1].legend()
axs[2].hist(pace, bins=12)
axs[2].xaxis.set_major_formatter(mmss)
fig.tight_layout()
fig.canvas.draw()
print(f"{int(cum[-1] // 3600)}:{int(cum[-1] % 3600 // 60):02d}", [t.get_text() for t in axs[0].get_yticklabels()][:3])
fig.savefig("marathon.png", dpi=50)
