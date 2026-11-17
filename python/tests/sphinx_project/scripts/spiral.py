"""A spiral, the whole script run by the plot directive."""

import numpy as np
from matplotlib import pyplot as plt

theta = np.linspace(0, 6 * np.pi, 300)
fig, ax = plt.subplots(subplot_kw={'projection': 'polar'})
ax.plot(theta, theta / (6 * np.pi))
ax.set_title('A spiral')
plt.show()
