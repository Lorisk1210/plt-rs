# Reuse one figure for twelve monthly reports: clear, redraw bars and a line, retitle, save each month.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(13)
fig = plt.figure(figsize=(6, 3.5))
months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
totals = []
for m, month in enumerate(months):
    fig.clf()
    ax = fig.add_subplot()
    daily = rng.poisson(20 + m, 28)
    ax.bar(range(1, 29), daily, color="C0")
    ax.plot(range(1, 29), np.convolve(daily, np.ones(7) / 7, mode="same"), color="C1")
    ax.set_title(f"{month}: {daily.sum()} orders")
    ax.set_ylim(0, 50)
    fig.savefig(f"report_{m + 1:02d}.png", dpi=40)
    totals.append(int(daily.sum()))
print(totals, len(fig.axes), fig.axes[0].get_title())
