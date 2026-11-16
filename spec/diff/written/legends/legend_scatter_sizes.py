# Bubble chart with a size legend made from PathCollection.legend_elements.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(12)
gdp = rng.uniform(1, 60, 25)
life = 55 + 0.4 * gdp + rng.normal(0, 3, 25)
pop = rng.uniform(1, 300, 25)
fig, ax = plt.subplots(figsize=(6, 4.5))
sc = ax.scatter(gdp, life, s=pop, c=gdp, cmap="viridis", alpha=0.6, edgecolors="k")
h_size, l_size = sc.legend_elements(prop="sizes", num=4, alpha=0.6, fmt="{x:.0f} M")
leg1 = ax.legend(h_size, l_size, loc="lower right", title="population")
ax.add_artist(leg1)
h_col, l_col = sc.legend_elements(prop="colors", num=3)
ax.legend(h_col, l_col, loc="upper left", title="GDP")
print(l_size, l_col)
fig.savefig("bubbles.png")
