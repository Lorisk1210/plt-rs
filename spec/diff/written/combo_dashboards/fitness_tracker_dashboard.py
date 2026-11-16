# Fitness tracker summary: steps bars with goal line, heart rate zones, sleep stacked bars, and a polar weekday clock.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(6)
days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
steps = rng.integers(4000, 14000, 7)
fig = plt.figure(figsize=(11, 8))
ax1 = fig.add_subplot(2, 2, 1)
colors = ["seagreen" if s >= 10000 else "lightgray" for s in steps]
ax1.bar(days, steps, color=colors)
ax1.axhline(10000, color="k", ls="--", lw=1)
ax1.text(6.4, 10200, "goal", ha="right")
ax2 = fig.add_subplot(2, 2, 2)
zones = {"rest": 600, "fat burn": 180, "cardio": 60, "peak": 15}
ax2.barh(list(zones), list(zones.values()), color=["C0", "C2", "C1", "C3"])
ax2.invert_yaxis()
ax2.set_xlabel("minutes")
ax3 = fig.add_subplot(2, 2, 3)
deep, light, rem = rng.uniform(1, 2, 7), rng.uniform(3, 4.5, 7), rng.uniform(1, 2, 7)
ax3.bar(days, deep, label="deep")
ax3.bar(days, light, bottom=deep, label="light")
ax3.bar(days, rem, bottom=deep + light, label="REM")
ax3.legend(ncols=3, loc="lower center", bbox_to_anchor=(0.5, 1.0))
ax4 = fig.add_subplot(2, 2, 4, projection="polar")
theta = np.linspace(0, 2 * np.pi, 7, endpoint=False)
ax4.bar(theta, steps / 1000, width=0.8, color=colors)
ax4.set_xticks(theta, days)
ax4.set_theta_zero_location("N")
ax4.set_theta_direction(-1)
fig.tight_layout()
print(steps.tolist(), [p.get_facecolor() for p in ax1.patches][:2])
fig.savefig("fitness.png", dpi=60)
