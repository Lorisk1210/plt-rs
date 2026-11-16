# Change many rcParams, draw, reset with rcdefaults, draw again; compare properties of artists from both figures.
import matplotlib as mpl
import matplotlib.pyplot as plt

mpl.rcParams.update({"lines.linewidth": 4, "lines.color": "red", "axes.facecolor": "lightyellow", "axes.grid": True,
                     "figure.facecolor": "gray", "patch.facecolor": "green", "font.size": 14})
fig1, ax1 = plt.subplots(figsize=(4, 3))
l1, = ax1.plot([0, 1], color=None)
b1 = ax1.bar([0], [1])[0]
mpl.rcdefaults()
fig2, ax2 = plt.subplots(figsize=(4, 3))
l2, = ax2.plot([0, 1])
b2 = ax2.bar([0], [1])[0]
print(l1.get_linewidth(), l2.get_linewidth(), ax1.get_facecolor(), ax2.get_facecolor())
print(fig1.get_facecolor(), fig2.get_facecolor(), b1.get_facecolor(), b2.get_facecolor())
print(ax1.title.get_fontsize() if ax1.title.get_text() else ax1.xaxis.get_ticklabels()[0].get_fontsize())
fig1.savefig("before_reset.png", dpi=40)
fig2.savefig("after_reset.png", dpi=40)
