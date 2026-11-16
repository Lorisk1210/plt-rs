# Relative font sizes ('xx-small'...'xx-large', 'smaller', 'larger') and points under font.size=14.
import matplotlib.pyplot as plt

plt.rcParams["font.size"] = 14
fig, ax = plt.subplots(figsize=(6, 4))
sizes = ["xx-small", "x-small", "small", "medium", "large", "x-large", "xx-large", "smaller", "larger", 9, 20.5]
for i, s in enumerate(sizes):
    t = ax.text(0.05, 1 - (i + 1) * 0.085, f"size={s!r}", fontsize=s, transform=ax.transAxes)
    print(repr(s), t.get_fontsize())
ax.axis("off")
fig.savefig("sizes.png")
fig.savefig("fontsize_names_scaling_small.png", dpi=50)
print(fig.get_size_inches())
