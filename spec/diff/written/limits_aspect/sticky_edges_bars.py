# Bars and images stick to the axes edges (sticky edges) while lines get margins.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 3, figsize=(10, 3))
axs[0].bar(range(5), [3, 1, 4, 1, 5])
axs[1].imshow(np.arange(12).reshape(3, 4))
axs[2].fill_between(np.arange(5), [3, 1, 4, 1, 5])
for ax in axs:
    print([round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_ylim()])
axs[2].use_sticky_edges = False
axs[2].autoscale_view()
print([round(v, 3) for v in axs[2].get_ylim()])
fig.savefig("sticky.png")
