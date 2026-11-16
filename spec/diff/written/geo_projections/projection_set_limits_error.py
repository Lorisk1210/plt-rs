# Users try to zoom a Mollweide map with set_xlim; it raises, so they catch it and go on.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(7, 4))
ax = fig.add_subplot(projection="mollweide")
ax.plot(np.linspace(-1, 1, 10), np.linspace(-0.5, 0.5, 10))
try:
    ax.set_xlim(-1, 1)
except TypeError as err:
    print("TypeError:", err)
print([round(v, 4) for v in ax.get_xlim()])
ax.set_title("full map only")
ax.grid(True)
fig.savefig("no_zoom.png")
