# Switch layout engines on an existing figure and query their settings.
import matplotlib.pyplot as plt
from matplotlib.layout_engine import ConstrainedLayoutEngine, TightLayoutEngine

fig, axs = plt.subplots(1, 2, figsize=(6, 3))
axs[0].plot([1, 2, 3])
axs[1].plot([3, 2, 1])
axs[0].set_ylabel("a fairly long y label")
print(fig.get_layout_engine())
fig.set_layout_engine("tight")
print(type(fig.get_layout_engine()).__name__)
fig.savefig("tight_engine.png")
fig.set_layout_engine(TightLayoutEngine(pad=3))
fig.savefig("tight_pad3.png")
fig.set_layout_engine("none")
print(fig.get_layout_engine())
fig2, ax2 = plt.subplots(layout=ConstrainedLayoutEngine(h_pad=0.2))
ax2.plot([1, 4, 9])
print(fig2.get_layout_engine().get())
fig2.savefig("constrained_engine.png")
