# Change fonts: family, size, weight, style, via keywords, FontProperties and rcParams.
import matplotlib.pyplot as plt
from matplotlib.font_manager import FontProperties

plt.rcParams["font.size"] = 11
plt.rcParams["font.family"] = "DejaVu Sans"
fig, ax = plt.subplots(figsize=(6, 4))
ax.plot([1, 2, 3], [2, 4, 3])
ax.set_title("Bold title", fontweight="bold", fontsize=16)
ax.set_xlabel("italic label", fontstyle="italic")
ax.set_ylabel("serif label", family="serif")
fp = FontProperties(family="monospace", size=12, weight="light")
ax.text(1.5, 3.5, "monospace text", fontproperties=fp)
ax.text(2.2, 2.4, "small caps?", fontvariant="small-caps", fontsize="large")
ax.tick_params(labelsize="small")
print(fp.get_family(), fp.get_size_in_points(), ax.title.get_fontweight())
fig.savefig("fonts.png")
