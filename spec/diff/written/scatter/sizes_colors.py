# Scatter of cars: weight vs mpg, point size by horsepower, colour by cylinders, with colour bar.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(1)
cyl = rng.choice([4, 6, 8], 60)
weight = 1.8 + 0.25 * cyl + rng.normal(0, 0.25, 60)
mpg = 50 - 7 * weight + rng.normal(0, 2, 60)
hp = 40 + 18 * cyl + rng.normal(0, 10, 60)
fig, ax = plt.subplots(figsize=(6, 4.5))
sc = ax.scatter(weight, mpg, s=hp, c=cyl, cmap="viridis", alpha=0.7, edgecolors="k", linewidths=0.5)
cb = fig.colorbar(sc, ax=ax, ticks=[4, 6, 8])
cb.set_label("cylinders")
ax.set_xlabel("weight [t]")
ax.set_ylabel("mpg")
print(sc.get_sizes()[:3].round(2), sc.get_array()[:5])
fig.savefig("cars.png")
