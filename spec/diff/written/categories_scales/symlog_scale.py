# Profit and loss spanning large positive and negative values on a symlog axis with linthresh.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-50, 50, 401)
y = x ** 3
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
a.plot(x, y)
a.set_yscale("symlog", linthresh=100, linscale=0.5)
a.set_title("symlog y")
a.grid(True, alpha=0.3)
b.plot(y, x)
b.set_xscale("symlog", linthresh=10, base=10)
b.set_title("symlog x")
fig.tight_layout()
print(a.get_yscale(), a.yaxis.get_transform().linthresh)
print(np.round(a.get_yticks()[:5], 1))
fig.savefig("symlog.png")
