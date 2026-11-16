# Nested (sunburst-like) pie: categories in the outer ring, subcategories in the inner ring.
import numpy as np
import matplotlib.pyplot as plt

outer = np.array([60, 32, 8])
inner = np.array([35, 25, 20, 12, 5, 3])
cmap = plt.colormaps["tab20c"]
fig, ax = plt.subplots(figsize=(5, 5))
ax.pie(outer, radius=1, colors=cmap([0, 4, 8]), wedgeprops=dict(width=0.3, edgecolor="w"),
       labels=["hardware", "software", "services"], labeldistance=1.05)
ax.pie(inner, radius=0.7, colors=cmap([1, 2, 5, 6, 9, 10]), wedgeprops=dict(width=0.3, edgecolor="w"))
ax.set(aspect="equal", title="Revenue mix")
print(len(ax.patches))
fig.savefig("nested.png")
