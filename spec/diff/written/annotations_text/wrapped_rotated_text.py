# Long explanatory text wrapped inside the figure and rotated labels at different alignments.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
long = ("This note explains that the measurements on the left were taken with an older "
        "instrument and should be compared with care against the newer ones.")
ax.text(5, 8, long, ha="center", va="top", wrap=True, fontsize=10)
for angle, x in zip([0, 30, 60, 90], [1.5, 3.5, 5.5, 7.5]):
    ax.text(x, 3, f"{angle}°", rotation=angle, ha="left", va="bottom", rotation_mode="anchor")
    ax.plot(x, 3, "r.")
ax.text(9.5, 1, "bottom right", ha="right", va="bottom", rotation=-15, color="gray")
fig.savefig("text_layout.png")
print([round(t.get_rotation(), 1) for t in ax.texts])
