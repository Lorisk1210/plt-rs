# Scatter coloured by a categorical column using a dict lookup and a legend from unique values.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(7)
species = rng.choice(["Adelie", "Gentoo", "Chinstrap"], 120)
palette = {"Adelie": "darkorange", "Gentoo": "teal", "Chinstrap": "mediumorchid"}
flipper = np.array([{"Adelie": 190, "Gentoo": 217, "Chinstrap": 196}[s] for s in species]) + rng.normal(0, 6, 120)
mass = flipper * 22 - 400 + rng.normal(0, 250, 120)
fig, ax = plt.subplots(figsize=(6, 4))
ax.scatter(flipper, mass, c=[palette[s] for s in species], s=20)
for name, color in palette.items():
    ax.scatter([], [], c=color, label=name)
ax.legend(title="species")
ax.set_xlabel("flipper length [mm]")
ax.set_ylabel("body mass [g]")
print(sorted(set(species)))
fig.savefig("penguins.png")
