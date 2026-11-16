# Restaurant analytics: covers per hour by weekday (lines), dish popularity bars with ratings as text, and tip % histogram.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(231)
hours = np.arange(11, 23)
fig, axs = plt.subplots(1, 3, figsize=(15, 4.5))
for d, name in enumerate(["Mon", "Fri", "Sat", "Sun"]):
    covers = 10 + 30 * np.exp(-((hours - 13) / 1.2) ** 2) + (20 + d * 8) * np.exp(-((hours - 19.5) / 1.5) ** 2)
    axs[0].plot(hours, covers + rng.normal(0, 2, hours.size), marker=".", label=name)
axs[0].legend()
dishes = {"pizza": (120, 4.5), "pasta": (95, 4.2), "salad": (40, 3.9), "steak": (60, 4.7), "soup": (30, 3.6)}
bars = axs[1].bar(list(dishes), [v[0] for v in dishes.values()], color="C1")
axs[1].bar_label(bars, labels=[f"★{v[1]}" for v in dishes.values()])
axs[2].hist(np.clip(rng.normal(12, 4, 300), 0, 30), bins=15, color="C2")
axs[2].set_xlabel("tip %")
fig.tight_layout()
print([t.get_text() for t in axs[1].texts], len(axs[0].lines))
fig.savefig("restaurant.png", dpi=50)
