# Major and minor grids with different styles, grid behind data, and only y gridlines on a second axes.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 100)
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
a.plot(x, np.sqrt(x))
a.minorticks_on()
a.grid(which="major", linestyle="-", linewidth=0.8, color="0.6")
a.grid(which="minor", linestyle=":", linewidth=0.5, color="0.8")
a.set_axisbelow(True)
b.bar(range(5), [4, 7, 3, 8, 5], zorder=3)
b.yaxis.grid(True, color="lightgray", ls="--")
b.set_axisbelow(True)
print(a.xaxis._major_tick_kw.get("gridOn"), b.get_axisbelow())
fig.savefig("grids.png")
