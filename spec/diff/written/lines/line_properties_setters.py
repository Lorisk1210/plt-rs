# Change line properties after creation: set_data, set_color, set_dashes, set_alpha, zorder.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 1, 100)
fig, ax = plt.subplots(figsize=(6, 3.5))
line, = ax.plot(x, x ** 2)
other, = ax.plot(x, x, color="gray", lw=4)
line.set_data(x, np.sqrt(x))
line.set_color("crimson")
line.set_linewidth(2.5)
line.set_dashes([6, 2, 2, 2])
line.set_zorder(5)
other.set_alpha(0.4)
line.set_label("sqrt")
other.set(label="identity", solid_capstyle="round")
ax.legend()
ax.relim()
ax.autoscale_view()
print(line.get_linestyle(), other.get_alpha(), line.get_zorder())
fig.savefig("setters.png")
