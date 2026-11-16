# Operations dashboard: latency percentiles with twin error rate, status heat map, a table and gauges.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.patches import Wedge

rng = np.random.default_rng(2)
t = pd.date_range("2024-05-01", periods=96, freq="15min")
lat = pd.DataFrame({q: np.exp(rng.normal(np.log(b), 0.2, 96)) for q, b in [("p50", 80), ("p95", 200), ("p99", 400)]}, index=t)
errors = pd.Series(np.clip(rng.normal(0.5, 0.3, 96), 0, None), index=t)
fig = plt.figure(figsize=(12, 8), layout="constrained")
gs = fig.add_gridspec(3, 3)
ax_lat = fig.add_subplot(gs[0, :2])
for col in lat:
    ax_lat.plot(lat.index, lat[col], label=col)
ax_err = ax_lat.twinx()
ax_err.fill_between(errors.index, errors, color="red", alpha=0.15, label="errors %")
ax_lat.legend(loc="upper left", ncols=3)
ax_lat.set_ylabel("ms")
ax_err.set_ylabel("errors %")
ax_heat = fig.add_subplot(gs[1, :2])
status = (lat["p99"].values.reshape(4, 24) > 450).astype(int) + (errors.values.reshape(4, 24) > 0.9)
ax_heat.imshow(status, cmap="RdYlGn_r", vmin=0, vmax=2, aspect="auto")
ax_heat.set_yticks(range(4), ["00-06", "06-12", "12-18", "18-24"])
ax_heat.set_title("incidents per hour block")
ax_tab = fig.add_subplot(gs[2, :2])
ax_tab.axis("off")
summary = lat.describe().loc[["mean", "50%", "max"]].round(1)
ax_tab.table(cellText=summary.values, rowLabels=summary.index, colLabels=summary.columns, loc="center")
for k, (name, val) in enumerate([("uptime", 99.2), ("SLO", 87.0), ("budget", 40.0)]):
    ax = fig.add_subplot(gs[k, 2])
    ax.add_patch(Wedge((0, 0), 1, 0, 180, width=0.3, color="0.9"))
    ax.add_patch(Wedge((0, 0), 1, 180 - val * 1.8, 180, width=0.3, color="C2" if val > 90 else "C1"))
    ax.text(0, 0.1, f"{val:.1f}%", ha="center", fontsize=14)
    ax.set_title(name)
    ax.set_xlim(-1.1, 1.1)
    ax.set_ylim(-0.2, 1.1)
    ax.set_aspect("equal")
    ax.axis("off")
print(summary.to_dict())
print(ax_lat.get_legend_handles_labels()[1], ax_err.get_ylim()[0])
fig.savefig("ops.png", dpi=60)
