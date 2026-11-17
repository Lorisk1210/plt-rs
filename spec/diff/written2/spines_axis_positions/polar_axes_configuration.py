# Polar axes configuration: theta offset/direction/zero location, thetamin/max, rorigin, rlabel position, rscale log; tick labels.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(8, 3), dpi=50)
axs = [fig.add_subplot(1, 3, k + 1, projection="polar") for k in range(3)]
for ax in axs:
    ax.plot(np.linspace(0, 2 * np.pi, 50), np.linspace(1, 10, 50))
axs[0].set_theta_zero_location("N", offset=10)
axs[0].set_theta_direction(-1)
axs[0].set_rlabel_position(135)
axs[1].set_thetamin(-45)
axs[1].set_thetamax(90)
axs[1].set_rorigin(-3)
axs[1].set_rmin(1)
axs[2].set_rscale("log")
axs[2].set_rlim(1, 10)
axs[2].set_thetagrids([0, 90, 180, 270], labels=["E", "N", "W", "S"])
axs[2].set_rgrids([2, 5, 8], labels=["two", "five", "eight"], angle=200)
fig.canvas.draw()
for ax in axs:
    print(round(ax.get_theta_offset(), 4), ax.get_theta_direction(), round(ax.get_rlabel_position(), 2),
          [round(v, 3) for v in (ax.get_thetamin(), ax.get_thetamax())], [round(v, 3) for v in ax.get_ylim()],
          round(ax.get_rorigin(), 3), ax.get_yscale())
    print([t.get_text() for t in ax.get_xticklabels()][:4], [t.get_text() for t in ax.get_yticklabels()][:3])
print(axs[0].format_coord(np.pi / 4, 5))
fig.savefig("polar_config.png")
axs[0].set_rticks([2, 4, 6])
axs[0].set_xticks(np.radians([0, 120, 240]))
fig.canvas.draw()
print([t.get_text() for t in axs[0].get_xticklabels()], [t.get_text() for t in axs[0].get_yticklabels()])
