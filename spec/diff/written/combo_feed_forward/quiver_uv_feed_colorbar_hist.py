# Read quiver U/V back (q.U, q.V) to compute speeds for a colour bar mapping and a speed histogram.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[-1:1:12j, -1:1:12j]
fig, (a, b) = plt.subplots(1, 2, figsize=(10, 4))
q = a.quiver(x, y, -y, x * 0.5)
speed = np.hypot(q.U, q.V)
q.set_array(speed.ravel())
q.set_cmap("plasma")
fig.colorbar(q, ax=a)
b.hist(np.asarray(speed).ravel(), bins=12)
print(q.N, round(float(np.max(speed)), 4), type(q.U).__name__)
fig.savefig("quiver_speed.png", dpi=50)
