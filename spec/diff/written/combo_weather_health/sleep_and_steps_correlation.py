# Sleep vs steps: scatter with regression line and CI band, marginal histograms with GridSpec, weekday colours.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(153)
days = 120
weekday = np.arange(days) % 7
steps = rng.normal(8000, 2500, days) + (weekday >= 5) * 2000
sleep = 7 + 0.0001 * (steps - 8000) + rng.normal(0, 0.6, days)
coef = np.polyfit(steps, sleep, 1)
fig = plt.figure(figsize=(7, 7))
gs = fig.add_gridspec(2, 2, width_ratios=[4, 1], height_ratios=[1, 4], hspace=0.05, wspace=0.05)
ax = fig.add_subplot(gs[1, 0])
sc = ax.scatter(steps, sleep, c=weekday, cmap="tab10", vmin=0, vmax=9, s=15)
xs = np.linspace(steps.min(), steps.max(), 50)
ax.plot(xs, np.polyval(coef, xs), "k")
ax.fill_between(xs, np.polyval(coef, xs) - 0.3, np.polyval(coef, xs) + 0.3, color="k", alpha=0.1)
fig.add_subplot(gs[0, 0], sharex=ax).hist(steps, bins=20, color="0.6")
fig.add_subplot(gs[1, 1], sharey=ax).hist(sleep, bins=20, orientation="horizontal", color="0.6")
ax.legend(*sc.legend_elements(), title="weekday", fontsize=7, ncols=2)
print(np.round(coef, 7).tolist(), round(float(np.corrcoef(steps, sleep)[0, 1]), 4))
fig.savefig("sleep_steps.png", dpi=50)
