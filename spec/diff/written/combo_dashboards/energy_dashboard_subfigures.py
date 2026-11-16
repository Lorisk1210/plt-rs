# Energy dashboard in subfigures: generation mix stackplot, a donut of the latest day, and a price heat map.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(4)
hours = pd.date_range("2024-06-01", periods=24 * 7, freq="h")
solar = np.clip(np.sin((hours.hour.to_numpy() - 6) / 12 * np.pi), 0, None) * 40 * rng.uniform(0.7, 1, len(hours))
wind = 20 + 10 * np.sin(np.arange(len(hours)) / 17) + rng.normal(0, 3, len(hours))
gas = np.clip(90 - solar - wind, 10, None)
fig = plt.figure(figsize=(12, 7), layout="constrained")
top, bottom = fig.subfigures(2, 1, height_ratios=[1.3, 1])
ax_stack = top.subplots()
polys = ax_stack.stackplot(hours, solar, wind, gas, labels=["solar", "wind", "gas"], colors=["gold", "skyblue", "gray"])
ax_stack.legend(loc="upper left", ncols=3)
ax_stack.set_ylabel("GW")
top.suptitle("Generation mix")
left, right = bottom.subplots(1, 2, width_ratios=[1, 2])
last = slice(-24, None)
totals = [solar[last].sum(), wind[last].sum(), gas[last].sum()]
left.pie(totals, colors=[p.get_facecolor() for p in polys], wedgeprops=dict(width=0.4), labels=["solar", "wind", "gas"])
left.set_title("last day")
price = (gas.reshape(7, 24) * 1.5 + rng.normal(0, 5, (7, 24)))
im = right.imshow(price, aspect="auto", cmap="magma")
right.set_yticks(range(7), [d.strftime("%a") for d in hours[::24]])
right.set_xlabel("hour")
bottom.colorbar(im, ax=right, label="€/MWh")
print([round(t, 1) for t in totals])
print([tuple(np.round(p.get_facecolor()[0], 3)) for p in polys])
fig.savefig("energy.png", dpi=60)
