# Two-link robot arm: arm poses drawn as lines with joint circles, workspace boundary rings, and joint-angle plots over time.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Circle

l1, l2 = 1.0, 0.7
t = np.linspace(0, 2 * np.pi, 9)
q1, q2 = 0.5 + 0.6 * np.sin(t), 1.0 + 0.5 * np.cos(t)
fig, (a, b) = plt.subplots(1, 2, figsize=(12, 5))
for k, (a1, a2) in enumerate(zip(q1, q2)):
    j = np.array([l1 * np.cos(a1), l1 * np.sin(a1)])
    e = j + [l2 * np.cos(a1 + a2), l2 * np.sin(a1 + a2)]
    a.plot([0, j[0], e[0]], [0, j[1], e[1]], "-o", color=plt.cm.viridis(k / 8), ms=4)
a.add_patch(Circle((0, 0), l1 + l2, fill=False, ls="--"))
a.add_patch(Circle((0, 0), l1 - l2, fill=False, ls="--"))
a.set_aspect("equal"); a.set_xlim(-2, 2); a.set_ylim(-2, 2)
tt = np.linspace(0, 2 * np.pi, 200)
b.plot(tt, np.degrees(0.5 + 0.6 * np.sin(tt)), label="q1")
b.plot(tt, np.degrees(1.0 + 0.5 * np.cos(tt)), label="q2")
b.legend(); b.set(xlabel="t", ylabel="angle [°]")
print(len(a.lines), round(float(np.degrees(q2.max())), 3))
fig.savefig("robot.png", dpi=50)
