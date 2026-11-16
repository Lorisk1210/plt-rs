# Temperature in Celsius with a secondary Fahrenheit axis on the right.
import numpy as np
import matplotlib.pyplot as plt

hours = np.arange(24)
celsius = 12 + 8 * np.sin((hours - 9) / 24 * 2 * np.pi)
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.plot(hours, celsius, "o-", ms=3)
ax.set_xlabel("hour")
ax.set_ylabel("°C")
ax.set_xticks(range(0, 24, 3))
secax = ax.secondary_yaxis("right", functions=(lambda c: c * 9 / 5 + 32, lambda f: (f - 32) * 5 / 9))
secax.set_ylabel("°F")
secax.yaxis.set_major_locator(plt.MultipleLocator(5))
ax.grid(True, alpha=0.3)
print(ax.get_ylim())
fig.savefig("celsius.png")
