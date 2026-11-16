# Probabilities near 0 and 1 on a logit axis for a calibration curve.
import numpy as np
import matplotlib.pyplot as plt

p = np.linspace(0.001, 0.999, 300)
fig, ax = plt.subplots(figsize=(5, 4))
ax.plot(p, p, "k--", lw=0.8, label="ideal")
ax.plot(p, p ** 1.3 / (p ** 1.3 + (1 - p) ** 1.3), label="model")
ax.set_xscale("logit")
ax.set_yscale("logit")
ax.set_xlim(0.001, 0.999)
ax.set_ylim(0.001, 0.999)
ax.legend()
fig.canvas.draw()
print([t.get_text() for t in ax.get_yticklabels()][:5])
fig.savefig("logit.png")
