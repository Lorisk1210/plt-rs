# Grid of polar plots, one per month, of hourly activity (radial clock plots).
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(9)
theta = np.linspace(0, 2 * np.pi, 24, endpoint=False)
fig, axs = plt.subplots(2, 3, figsize=(9, 6), subplot_kw=dict(projection="polar"))
for k, ax in enumerate(axs.flat):
    activity = 5 + 4 * np.sin(theta - k / 2) + rng.uniform(0, 1, 24)
    ax.bar(theta, activity, width=2 * np.pi / 24, color=plt.cm.twilight(k / 6), alpha=0.8)
    ax.set_theta_zero_location("N")
    ax.set_theta_direction(-1)
    ax.set_xticks(theta[::6], ["0h", "6h", "12h", "18h"])
    ax.set_yticklabels([])
    ax.set_title(["Jan", "Feb", "Mar", "Apr", "May", "Jun"][k], pad=12)
fig.tight_layout()
print(axs.size)
fig.savefig("clocks.png")
