# Dumbbell chart comparing two years per country with hlines between points.
import numpy as np
import matplotlib.pyplot as plt

countries = ["Chile", "Kenya", "Norway", "India", "Peru", "Spain"]
y2010 = np.array([62, 48, 81, 55, 60, 72])
y2020 = np.array([70, 57, 84, 66, 63, 75])
order = np.argsort(y2020 - y2010)
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.hlines(range(6), y2010[order], y2020[order], color="0.7", lw=3)
ax.scatter(y2010[order], range(6), color="C0", s=60, label="2010", zorder=3)
ax.scatter(y2020[order], range(6), color="C1", s=60, label="2020", zorder=3)
ax.set_yticks(range(6), [countries[i] for i in order])
ax.legend(loc="lower right")
ax.set_xlabel("score")
print([countries[i] for i in order])
fig.savefig("dumbbell.png")
