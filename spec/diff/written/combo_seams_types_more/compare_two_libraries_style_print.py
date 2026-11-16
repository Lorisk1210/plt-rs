# Print the default artist properties a fresh plot gets (what users compare between versions), then draw the figure.
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
line, = ax.plot([0, 1])
bar = ax.bar([0], [1])[0]
sc = ax.scatter([0], [0])
txt = ax.set_title("t")
leg = ax.legend(["a", "b"])
checks = {"line lw": line.get_linewidth(), "line color": line.get_color(), "line ms": line.get_markersize(),
          "bar ec": bar.get_edgecolor(), "bar lw": bar.get_linewidth(), "scatter size": sc.get_sizes().tolist(),
          "scatter lw": sc.get_linewidths().tolist(), "title size": txt.get_fontsize(), "title weight": txt.get_fontweight(),
          "legend fontsize": leg.get_texts()[0].get_fontsize(), "legend frame alpha": leg.get_frame().get_alpha(),
          "axes fc": ax.get_facecolor(), "xtick size": ax.xaxis.get_major_ticks()[0].tick1line.get_markersize()}
for k, v in checks.items():
    print(k, repr(v))
fig.savefig("defaults.png", dpi=40)
