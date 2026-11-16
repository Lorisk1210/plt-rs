# Load duration curve, daily load profiles for seasons in a polar 'clock' view, and a capacity stack on cartesian axes.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(181)
hours = np.arange(8760)
load = 50 + 15 * np.sin(2 * np.pi * (hours % 24 - 7) / 24) + 10 * np.cos(2 * np.pi * hours / 8760) + rng.normal(0, 3, 8760)
fig = plt.figure(figsize=(12, 4.5))
a = fig.add_subplot(1, 3, 1)
a.plot(np.sort(load)[::-1]); a.set(xlabel="hours", ylabel="GW", title="duration curve")
p = fig.add_subplot(1, 3, 2, projection="polar")
theta = np.linspace(0, 2 * np.pi, 24, endpoint=False)
for season, start in [("winter", 0), ("summer", 4380)]:
    prof = load[start:start + 24 * 90].reshape(-1, 24).mean(axis=0)
    p.plot(np.r_[theta, theta[:1]], np.r_[prof, prof[:1]], label=season)
p.set_theta_zero_location("N"); p.set_theta_direction(-1); p.legend(fontsize=7)
c = fig.add_subplot(1, 3, 3)
c.bar(["base", "mid", "peak"], [45, 15, 12], bottom=[0, 45, 60], color=["0.4", "0.6", "0.8"])
c.axhline(load.max(), color="r", ls="--")
fig.tight_layout()
print(round(float(load.max()), 3), round(float(np.percentile(load, 50)), 3))
fig.savefig("grid_load.png", dpi=50)
