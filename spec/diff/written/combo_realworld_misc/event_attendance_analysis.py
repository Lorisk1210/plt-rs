# Conference attendance: registrations over time (cumulative), session attendance heat map by room and slot, and a ratings violin.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(235)
days = pd.date_range("2024-01-01", "2024-04-30", freq="D")
regs = rng.poisson(np.linspace(2, 25, len(days)))
fig = plt.figure(figsize=(12, 7))
a = fig.add_subplot(2, 1, 1)
a.plot(days, np.cumsum(regs))
a.axvline(pd.Timestamp("2024-03-01"), ls=":", color="r")
a.text(pd.Timestamp("2024-03-02"), 100, "early bird ends", color="r")
b = fig.add_subplot(2, 2, 3)
att = rng.integers(20, 200, (4, 6))
im = b.imshow(att, cmap="Blues")
b.set_yticks(range(4), ["A", "B", "C", "D"]); b.set_xticks(range(6), ["9", "10", "11", "14", "15", "16"])
fig.colorbar(im, ax=b)
c = fig.add_subplot(2, 2, 4)
c.violinplot([rng.normal(m, 0.8, 80).clip(1, 5) for m in (3.5, 4.1, 4.4)], showmedians=True)
c.set_xticks([1, 2, 3], ["day 1", "day 2", "day 3"])
fig.tight_layout()
print(int(regs.sum()), int(att.max()))
fig.savefig("conference.png", dpi=50)
