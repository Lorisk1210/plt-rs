# A Locator subclass placing ticks at the data's quartiles (min, q1, median, q3, max).
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import Locator


class QuartileLocator(Locator):
    def __init__(self, data):
        self.data = np.asarray(data)

    def __call__(self):
        return np.percentile(self.data, [0, 25, 50, 75, 100])


rng = np.random.default_rng(1)
data = rng.gamma(2, 2, 200)
fig, ax = plt.subplots(figsize=(6, 3))
ax.hist(data, bins=30)
ax.xaxis.set_major_locator(QuartileLocator(data))
ax.xaxis.set_major_formatter("{x:.1f}")
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
fig.savefig("quartiles.png")
