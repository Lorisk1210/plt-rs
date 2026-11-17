# Rotated tick labels with rotation_mode anchor/default, ha choices, autofmt_xdate and label overlap tested via extents.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

labels = [f"category number {k}" for k in range(8)]
for rot, mode, ha in [(0, "default", "center"), (45, "default", "right"), (45, "anchor", "right"), (90, "default", "center"),
                      (30, "xtick", "right")]:
    fig, ax = plt.subplots(figsize=(5, 3), dpi=72)
    ax.bar(labels, range(8))
    if mode == "xtick":
        ax.tick_params(axis="x", labelrotation=rot, rotation_mode="xtick")
    else:
        plt.setp(ax.get_xticklabels(), rotation=rot, ha=ha, rotation_mode=mode)
    fig.canvas.draw()
    r = fig.canvas.get_renderer()
    bbs = [t.get_window_extent(r) for t in ax.get_xticklabels()]
    overlaps = sum(bbs[i].overlaps(bbs[i + 1]) for i in range(len(bbs) - 1))
    print(rot, mode, ha, overlaps, [round(v, 1) for v in bbs[0].bounds], round(ax.get_tightbbox(r).y0, 1))
    plt.close(fig)
fig, ax = plt.subplots(figsize=(5, 3), dpi=72)
ax.bar(labels, range(8))
fig.autofmt_xdate(rotation=60, ha="right", bottom=0.3)
fig.canvas.draw()
print(ax.get_xticklabels()[0].get_rotation(), ax.get_xticklabels()[0].get_ha(), round(ax.get_position().y0, 3))
fig.savefig("rotated.png")
ax.tick_params(axis="y", labelrotation=45)
fig.canvas.draw()
print(ax.get_yticklabels()[1].get_rotation(), [round(v, 1) for v in ax.get_yticklabels()[1].get_window_extent().bounds])
print(len(ax.get_xticklabels()), ax.xaxis.get_ticklabels()[0].get_rotation_mode())
