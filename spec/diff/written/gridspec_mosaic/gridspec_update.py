# Adjust an existing GridSpec's spacing after plotting with update(), as answers suggest.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.gridspec as gridspec

fig = plt.figure(figsize=(6, 4))
gs = gridspec.GridSpec(2, 2)
axes = [fig.add_subplot(gs[i, j]) for i in range(2) for j in range(2)]
for k, ax in enumerate(axes):
    ax.plot(np.arange(5), np.arange(5) * k)
    ax.set_title(f"panel {k}")
gs.update(left=0.1, right=0.95, wspace=0.05, hspace=0.45)
print(gs.get_subplot_params(fig).wspace)
print([np.round(a.get_position().x0, 3) for a in axes])
fig.savefig("updated.png")
