# Draw circles that must look round: aspect 'equal' with adjustable 'datalim' vs 'box'.
import matplotlib.pyplot as plt
from matplotlib.patches import Circle

fig, axs = plt.subplots(1, 3, figsize=(10, 3.5))
for ax in axs:
    ax.add_patch(Circle((0, 0), 1, fc="lightblue", ec="navy"))
    ax.plot([-2, 2], [0, 0], "k", lw=0.5)
    ax.set_xlim(-2, 2)
    ax.set_ylim(-1.2, 1.2)
axs[0].set_title("auto aspect")
axs[1].set_aspect("equal", adjustable="box")
axs[1].set_title("equal, box")
axs[2].set_aspect("equal", adjustable="datalim")
axs[2].set_title("equal, datalim")
fig.savefig("circles.png")
for ax in axs:
    print(ax.get_aspect(), [round(v, 3) for v in ax.get_xlim()])
