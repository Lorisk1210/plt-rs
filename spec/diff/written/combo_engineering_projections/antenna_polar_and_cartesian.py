# Antenna patterns: polar dB plots (zero at north, clockwise) next to the same patterns on cartesian axes with -3 dB marks.
import numpy as np
import matplotlib.pyplot as plt

theta = np.linspace(-np.pi, np.pi, 721)
patterns = {"dipole": np.abs(np.cos(np.pi / 2 * np.sin(theta)) / np.maximum(np.abs(np.cos(theta)), 1e-6)),
            "4-element": np.abs(np.sinc(2 * np.sin(theta)))}
fig = plt.figure(figsize=(12, 4.5))
p = fig.add_subplot(1, 2, 1, projection="polar")
c = fig.add_subplot(1, 2, 2)
for name, g in patterns.items():
    db = 20 * np.log10(np.clip(g / g.max(), 1e-3, None))
    p.plot(theta, np.clip(db, -40, 0) + 40, label=name)
    line, = c.plot(np.degrees(theta), db, label=name)
    bw = np.degrees(theta[db > -3])
    c.axvspan(bw.min(), bw.max(), color=line.get_color(), alpha=0.1)
p.set_theta_zero_location("N"); p.set_theta_direction(-1)
p.set_rticks([10, 20, 30, 40], ["-30", "-20", "-10", "0 dB"])
c.set_ylim(-40, 2); c.legend()
print([round(v, 2) for v in c.get_xlim()], len(c.patches))
fig.savefig("antenna.png", dpi=50)
