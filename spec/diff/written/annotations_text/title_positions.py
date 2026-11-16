# Titles left, centre and right on one axes, with a title pad and a y position tweak.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 3.5))
ax.plot([0, 1, 2, 3], [1, 3, 2, 4])
ax.set_title("Left title", loc="left", fontsize=10, color="gray")
ax.set_title("Main title", pad=15)
ax.set_title("n=4", loc="right", fontstyle="italic")
fig.savefig("titles.png")
ax.set_title("Main title (lower)", y=0.9)
fig.savefig("titles_lower.png")
print(ax.get_title(), ax.get_title(loc="left"), ax.get_title("right"))
print(ax.title.get_position())
ax.set_xlabel("step")
