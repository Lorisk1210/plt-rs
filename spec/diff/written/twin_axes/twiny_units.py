# Show the same x data in two units: kilometres on the bottom and miles on a twiny top axis.
import numpy as np
import matplotlib.pyplot as plt

km = np.linspace(0, 42.195, 100)
pace = 4.5 + 0.02 * km + 0.3 * np.sin(km / 5)
fig, ax = plt.subplots(figsize=(7, 3.5))
ax.plot(km, pace)
ax.set_xlabel("distance [km]")
ax.set_ylabel("pace [min/km]")
ax.set_xlim(0, 42.195)
top = ax.twiny()
top.set_xlim(np.array(ax.get_xlim()) / 1.609344)
top.set_xlabel("distance [mi]")
print(np.round(top.get_xlim(), 3))
fig.tight_layout()
fig.savefig("marathon.png")
