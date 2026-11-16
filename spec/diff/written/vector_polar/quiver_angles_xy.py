# Gradient vectors drawn in data units with angles='xy', scale_units='xy', scale=1 over contours.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-2, 2, 11)
X, Y = np.meshgrid(x, x)
Z = X ** 2 - Y ** 2
dZy, dZx = np.gradient(Z, x, x)
fig, ax = plt.subplots(figsize=(5, 5))
ax.contour(X, Y, Z, levels=12, cmap="coolwarm")
ax.quiver(X, Y, dZx, dZy, angles="xy", scale_units="xy", scale=8, color="k", pivot="mid")
ax.set_aspect("equal")
ax.set_title("saddle gradient")
print(round(float(dZx.max()), 3))
fig.savefig("saddle.png")
