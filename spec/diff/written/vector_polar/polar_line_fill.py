# Polar plot of an antenna pattern in dB with theta zero at the top and clockwise direction.
import numpy as np
import matplotlib.pyplot as plt

theta = np.linspace(0, 2 * np.pi, 360)
pattern = np.abs(np.cos(2 * theta)) ** 3 + 0.05
db = 10 * np.log10(pattern)
fig, ax = plt.subplots(subplot_kw={"projection": "polar"}, figsize=(5, 5))
ax.plot(theta, db - db.min(), color="C3")
ax.fill(theta, db - db.min(), alpha=0.2, color="C3")
ax.set_theta_zero_location("N")
ax.set_theta_direction(-1)
ax.set_rlabel_position(45)
ax.set_title("Radiation pattern", va="bottom")
print(ax.get_theta_offset(), ax.get_theta_direction(), [round(v, 2) for v in ax.get_ylim()])
fig.savefig("antenna.png")
