# Depth profile: depth increases downward (inverted y), and a ranking chart with inverted x.
import numpy as np
import matplotlib.pyplot as plt

depth = np.linspace(0, 200, 60)
temp = 4 + 16 * np.exp(-depth / 40)
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 4))
a.plot(temp, depth)
a.invert_yaxis()
a.set_xlabel("temperature [°C]")
a.set_ylabel("depth [m]")
a.xaxis.set_label_position("top")
a.xaxis.tick_top()
b.barh(["A", "B", "C"], [3, 7, 5])
b.set_xlim(10, 0)
print(a.yaxis_inverted(), b.xaxis_inverted(), b.get_xlim())
fig.tight_layout()
fig.savefig("profile.png")
