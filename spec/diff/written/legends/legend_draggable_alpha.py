# Legend made draggable (no effect when saving), semi-transparent, and positioned by a float tuple.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-3, 3, 100)
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot(x, np.tanh(x), label="tanh")
ax.plot(x, 1 / (1 + np.exp(-x)), label="sigmoid")
ax.plot(x, np.maximum(0, x) / 3, label="relu / 3")
leg = ax.legend(loc=(0.05, 0.6), framealpha=0.5)
leg.set_draggable(True)
print(leg.get_draggable())
leg.set_title("activation")
leg.set_loc("lower right")
fig.savefig("activations.png")
print(leg._loc)
