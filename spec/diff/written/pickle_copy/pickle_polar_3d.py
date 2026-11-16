# Pickle figures with a polar axes and a 3-D axes and save the copies.
import pickle
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(8, 4))
p = fig.add_subplot(1, 2, 1, projection="polar")
p.plot(np.linspace(0, 2 * np.pi, 50), np.linspace(0, 1, 50))
d = fig.add_subplot(1, 2, 2, projection="3d")
d.plot([0, 1], [0, 1], [0, 1])
d.view_init(25, 40)
copy_fig = pickle.loads(pickle.dumps(fig))
print([a.name for a in copy_fig.axes], copy_fig.axes[1].elev, copy_fig.axes[1].azim)
copy_fig.savefig("polar_3d_copy.png")
fig.savefig("polar_3d.png")
