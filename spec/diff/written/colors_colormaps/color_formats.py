# Every colour format users pass: names, hex, short hex, RGB tuple, RGBA, gray string, CN, xkcd/tab names.
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors

specs = ["red", "#1f77b4", "#0f0", "#ff000080", (0.2, 0.4, 0.6), (0.2, 0.4, 0.6, 0.3), "0.5", "C3",
         "xkcd:sky blue", "tab:orange", ("green", 0.3), "none"]
fig, ax = plt.subplots(figsize=(7, 3))
for i, c in enumerate(specs):
    ax.bar(i, 1, color=c, edgecolor="k")
    print(repr(c), mcolors.to_hex(mcolors.to_rgba(c), keep_alpha=True))
ax.set_xticks(range(len(specs)), [str(s)[:10] for s in specs], rotation=60, fontsize=7)
fig.tight_layout()
fig.savefig("formats.png")
print(mcolors.is_color_like("notacolor"), mcolors.same_color("r", "red"))
