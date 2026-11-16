# Font sizes from rcParams, style sheet and keywords layered in one figure with titles, labels, legend and ticks.
import matplotlib as mpl
import matplotlib.pyplot as plt

mpl.rcParams.update({"font.size": 9, "axes.titlesize": "xx-large", "legend.fontsize": "small", "xtick.labelsize": 7})
fig, axs = plt.subplots(1, 2, figsize=(9, 3.5))
with plt.style.context("seaborn-v0_8-poster"):
    fig2, ax2 = plt.subplots(figsize=(6, 4))
    ax2.plot([0, 1], label="poster")
    ax2.legend()
    ax2.set_title("poster title")
    print("poster", ax2.title.get_fontsize(), ax2.get_legend().get_texts()[0].get_fontsize(), ax2.xaxis.get_ticklabels()[0].get_fontsize())
    fig2.savefig("poster.png", dpi=40)
for ax in axs:
    ax.plot([0, 1], [1, 0], label="line")
    ax.legend()
axs[0].set_title("rc sizes")
axs[1].set_title("explicit", fontsize=8)
axs[1].tick_params(labelsize=12)
print("rc", axs[0].title.get_fontsize(), axs[0].get_legend().get_texts()[0].get_fontsize(), axs[0].xaxis.get_ticklabels()[0].get_fontsize())
print("explicit", axs[1].title.get_fontsize(), axs[1].xaxis.get_ticklabels()[0].get_fontsize())
fig.savefig("fonts.png", dpi=50)
