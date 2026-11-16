# Colour bar with hand-set ticks and text labels ("low", "mid", "high") and minor ticks on.
import numpy as np
import matplotlib.pyplot as plt

risk = np.linspace(0, 1, 100).reshape(10, 10)
fig, ax = plt.subplots(figsize=(5, 4))
im = ax.imshow(risk, cmap="YlOrRd", vmin=0, vmax=1)
cb = fig.colorbar(im)
cb.set_ticks([0, 0.5, 1], labels=["low", "mid", "high"])
cb.minorticks_on()
cb.ax.tick_params(which="minor", length=2)
ax.set_title("Risk score")
fig.savefig("risk.png")
cb.minorticks_off()
cb.update_normal(im)
print([t.get_text() for t in cb.ax.get_yticklabels()])
fig.savefig("risk_no_minor.png")
