"""Line plot with a legend: the step response of a spring-mass system for
three damping ratios."""

import numpy as np

import pltrs.pyplot as plt

t = np.linspace(0, 12, 400)
omega = 1.5  # natural frequency (rad/s)

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
for zeta in [0.15, 0.4, 0.7]:
    damped = omega * np.sqrt(1 - zeta**2)
    response = 1 - np.exp(-zeta * omega * t) * (
        np.cos(damped * t) + zeta / np.sqrt(1 - zeta**2) * np.sin(damped * t)
    )
    ax.plot(t, response, label=f"ζ = {zeta}")
ax.axhline(1, color="0.5", linestyle="--", linewidth=1)
ax.set_title("Step response of a damped oscillator")
ax.set_xlabel("time (s)")
ax.set_ylabel("displacement")
ax.set_xlim(0, 12)
ax.legend(title="damping ratio")

fig.savefig("line_plot.png")
