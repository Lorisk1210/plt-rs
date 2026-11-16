# One colour spec travels through cmap sampling, to_rgba, to_hex, a bar, a legend patch and back via get_facecolor.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from matplotlib.patches import Patch

cmap = plt.get_cmap("viridis")
c = cmap(0.35)
fig, ax = plt.subplots(figsize=(5, 4))
bar = ax.bar([0], [1], color=c)[0]
ax.legend(handles=[Patch(color=mcolors.to_hex(c), label="v")])
patch = ax.get_legend().get_patches()[0]
print(np.round(c, 6).tolist(), mcolors.to_hex(c), mcolors.to_hex(bar.get_facecolor()), mcolors.to_hex(patch.get_facecolor()))
print(mcolors.same_color(bar.get_facecolor(), patch.get_facecolor()), np.round(mcolors.to_rgb(mcolors.to_hex(c)), 6).tolist())
fig.savefig("pipeline.png", dpi=50)
