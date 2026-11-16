# List the registered projections and create one Axes of each by name in a grid.
import matplotlib.pyplot as plt
from matplotlib import projections

names = projections.get_projection_names()
print(names)
fig = plt.figure(figsize=(10, 6))
plain = [n for n in names if n not in ("3d",)]
for i, name in enumerate(plain, start=1):
    ax = fig.add_subplot(2, 4, i, projection=name)
    ax.set_title(name, fontsize=9, pad=10)
    print(name, type(ax).__name__)
fig.tight_layout()
fig.savefig("all_projections.png")
