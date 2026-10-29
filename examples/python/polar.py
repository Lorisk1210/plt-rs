"""Polar axes: a radar chart scoring three laptops on six criteria, each a
closed, shaded outline."""

import numpy as np

import pltrs.pyplot as plt

criteria = ["speed", "battery", "screen", "weight", "price", "keyboard"]
scores = {
    "Aero 14": [4.5, 3.0, 4.0, 4.5, 2.0, 3.5],
    "Bolt 16": [5.0, 2.0, 4.5, 2.0, 2.5, 4.0],
    "Civic 13": [3.0, 4.5, 3.0, 4.0, 4.5, 3.0],
}
angles = np.linspace(0, 2 * np.pi, len(criteria), endpoint=False)

fig, ax = plt.subplots(figsize=(6.4, 5.2), subplot_kw=dict(projection="polar"), layout="constrained")
ax.set_theta_offset(np.pi / 2)
ax.set_theta_direction(-1)
for name, values in scores.items():
    closed = np.append(angles, angles[0]), np.append(values, values[0])
    ax.plot(*closed, linewidth=2, label=name)
    ax.fill(*closed, alpha=0.15)
ax.set_xticks(angles, criteria)
ax.set_ylim(0, 5)
ax.set_yticks([1, 2, 3, 4, 5])
ax.set_rlabel_position(90)
ax.set_title("Laptop review scores")
ax.legend(loc="upper right", bbox_to_anchor=(1.25, 1.08))

fig.savefig("polar.png")
