# A tongue-in-cheek xkcd-style dashboard with bars, a pie, annotations and a line in one figure.
import numpy as np
import matplotlib.pyplot as plt

with plt.xkcd():
    fig, axs = plt.subplots(1, 3, figsize=(12, 4))
    axs[0].bar(["coffee", "tea", "water"], [8, 2, 1], color=["sienna", "olive", "skyblue"])
    axs[0].set_title("cups per day")
    axs[1].pie([70, 20, 10], labels=["meetings", "email", "code"])
    axs[1].set_title("my week")
    t = np.arange(10)
    axs[2].plot(t, 10 - t * 0.8, label="energy")
    axs[2].annotate("friday", xy=(9, 2.8), xytext=(4, 2), arrowprops=dict(arrowstyle="->"))
    axs[2].legend()
    fig.tight_layout()
    fig.savefig("xkcd_dash.png", dpi=50)
    print(plt.rcParams["path.sketch"], [p.get_label() for p in axs[0].patches][:1])
print(plt.rcParams["path.sketch"])
