# Transit ridership: line ridership over a week by hour (heat map), station boardings bubble map, and mode share donut.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(213)
hours = np.arange(24)
profile = 200 + 800 * (np.exp(-((hours - 8) / 1.5) ** 2) + np.exp(-((hours - 17.5) / 1.8) ** 2))
week = np.array([profile * (0.6 if d >= 5 else 1) + rng.normal(0, 30, 24) for d in range(7)])
fig = plt.figure(figsize=(12, 7))
a = fig.add_subplot(2, 1, 1)
im = a.imshow(week, aspect="auto", cmap="viridis")
a.set_yticks(range(7), ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"])
fig.colorbar(im, ax=a, label="riders")
b = fig.add_subplot(2, 2, 3)
st = rng.uniform(0, 10, (15, 2)); board = rng.lognormal(6, 0.7, 15)
b.scatter(st[:, 0], st[:, 1], s=board / 10, alpha=0.6, c=board, cmap="plasma")
b.plot(*st[np.argsort(st[:, 0])].T, "k-", lw=0.5)
c = fig.add_subplot(2, 2, 4)
c.pie([45, 30, 15, 10], labels=["bus", "metro", "tram", "ferry"], wedgeprops=dict(width=0.4), autopct="%d%%")
fig.tight_layout()
print(round(float(week.max()), 2), int(board.sum()))
fig.savefig("transit.png", dpi=50)
