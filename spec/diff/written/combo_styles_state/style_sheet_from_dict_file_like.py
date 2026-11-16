# Apply a style from a dict built by parsing a small .mplstyle-like text in the script, then compare with defaults.
import matplotlib.pyplot as plt

style_text = """
axes.facecolor: f5f5f5
axes.edgecolor: 444444
axes.titleweight: bold
grid.linestyle: --
axes.grid: True
lines.markersize: 8
"""
style = {}
for line in style_text.strip().splitlines():
    key, value = [p.strip() for p in line.split(":", 1)]
    style[key] = value if not value.replace("f", "").isalnum() else value
with plt.style.context(style):
    fig, ax = plt.subplots(figsize=(5, 3.5))
    ax.plot([1, 2, 3], [3, 1, 2], "o-")
    ax.set_title("custom")
    print(ax.get_facecolor(), ax.title.get_fontweight(), ax.lines[0].get_markersize())
    fig.savefig("custom_style.png", dpi=50)
fig2, ax2 = plt.subplots(figsize=(5, 3.5))
ax2.plot([1, 2, 3], [3, 1, 2], "o-")
print(ax2.get_facecolor(), ax2.lines[0].get_markersize())
fig2.savefig("default_style.png", dpi=50)
