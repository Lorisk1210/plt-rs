# Geometry lesson: triangle with labelled vertices, angle arcs with degree labels, circumcircle and side lengths.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Polygon, Circle, Arc

A, B, C = np.array([0, 0]), np.array([4, 0]), np.array([1.2, 3])
fig, ax = plt.subplots(figsize=(6, 6))
ax.add_patch(Polygon([A, B, C], fill=False, lw=2))
d = 2 * (A[0] * (B[1] - C[1]) + B[0] * (C[1] - A[1]) + C[0] * (A[1] - B[1]))
ux = ((A @ A) * (B[1] - C[1]) + (B @ B) * (C[1] - A[1]) + (C @ C) * (A[1] - B[1])) / d
uy = ((A @ A) * (C[0] - B[0]) + (B @ B) * (A[0] - C[0]) + (C @ C) * (B[0] - A[0])) / d
R = np.hypot(*(A - [ux, uy]))
ax.add_patch(Circle((ux, uy), R, fill=False, ls="--", color="gray"))
for P, Q, S, name in [(A, B, C, "A"), (B, C, A, "B"), (C, A, B, "C")]:
    v1, v2 = Q - P, S - P
    a1, a2 = np.degrees(np.arctan2(v1[1], v1[0])), np.degrees(np.arctan2(v2[1], v2[0]))
    lo, hi = sorted((a1, a2))
    if hi - lo > 180:
        lo, hi = hi, lo + 360
    ax.add_patch(Arc(P, 0.8, 0.8, theta1=lo, theta2=hi, color="C1"))
    ang = np.degrees(np.arccos(v1 @ v2 / np.linalg.norm(v1) / np.linalg.norm(v2)))
    ax.text(*(P + 0.25 * (v1 / np.linalg.norm(v1) + v2 / np.linalg.norm(v2))), f"{ang:.1f}°", fontsize=8)
    ax.text(*(P - 0.25 * (v1 + v2) / np.linalg.norm(v1 + v2)), name, fontsize=14, ha="center", va="center")
for P, Q in [(A, B), (B, C), (C, A)]:
    ax.text(*((P + Q) / 2), f"{np.linalg.norm(P - Q):.2f}", color="C0", ha="center")
ax.set_aspect("equal"); ax.set_xlim(-1.5, 5.5); ax.set_ylim(-2.5, 4.5); ax.axis("off")
print(round(float(R), 4), round(float(ux), 4), round(float(uy), 4))
fig.savefig("triangle.png", dpi=50)
