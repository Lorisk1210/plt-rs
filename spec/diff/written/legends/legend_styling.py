# Heavily styled legend: shadow, rounded frame, marker scale, font size, frame colours, label colour.
import numpy as np
import matplotlib.pyplot as plt

np.random.seed(1)
fig, ax = plt.subplots(figsize=(5.5, 4))
for i, m in enumerate("osD^"):
    ax.scatter(np.random.rand(10), np.random.rand(10), marker=m, s=20, label=f"group {i}")
leg = ax.legend(shadow=True, fancybox=True, markerscale=2, fontsize="small", framealpha=1,
                facecolor="ivory", edgecolor="darkgoldenrod", labelcolor="markerfacecolor",
                title="Groups", title_fontsize=11, borderpad=1.0, labelspacing=0.8,
                handletextpad=0.3, loc="upper left", bbox_to_anchor=(1, 1))
leg.get_frame().set_linewidth(1.5)
fig.savefig("styled.png", bbox_inches="tight")
print(leg.get_frame().get_linewidth(), leg.get_texts()[0].get_color())
