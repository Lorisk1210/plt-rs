# Orbit simulation figure: trajectory coloured by speed, energy drift on a twin axis, and a polar view of the orbit.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection

dt, n = 0.01, 3000
pos = np.array([1.0, 0.0]); vel = np.array([0.0, 1.2])
traj, speeds, energy = [], [], []
for _ in range(n):
    r = np.linalg.norm(pos)
    acc = -pos / r ** 3
    vel = vel + acc * dt
    pos = pos + vel * dt
    traj.append(pos.copy()); speeds.append(np.linalg.norm(vel)); energy.append(0.5 * vel @ vel - 1 / np.linalg.norm(pos))
traj = np.array(traj); speeds = np.array(speeds); energy = np.array(energy)
fig = plt.figure(figsize=(11, 4))
ax1 = fig.add_subplot(1, 3, 1)
segs = np.stack([traj[:-1], traj[1:]], axis=1)
lc = LineCollection(segs, array=speeds[:-1], cmap="viridis", linewidths=1.5)
ax1.add_collection(lc)
ax1.autoscale()
ax1.set_aspect("equal")
fig.colorbar(lc, ax=ax1, label="speed")
ax2 = fig.add_subplot(1, 3, 2)
ax2.plot(np.arange(n) * dt, energy - energy[0])
ax2.set_ylabel("energy drift")
ax3 = ax2.twinx()
ax3.plot(np.arange(n) * dt, np.hypot(*traj.T), color="C1", alpha=0.6)
ax3.set_ylabel("radius")
ax4 = fig.add_subplot(1, 3, 3, projection="polar")
ax4.plot(np.arctan2(traj[:, 1], traj[:, 0]), np.hypot(*traj.T), lw=0.8)
fig.tight_layout()
print(round(float(energy[0]), 5), round(float(np.ptp(energy)), 6), round(float(speeds.max()), 4))
fig.savefig("orbit.png", dpi=50)
