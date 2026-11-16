# Create figures with layout-related keywords (frameon, linewidth, edgecolor) users copy from answers.
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(5, 3), dpi=90, facecolor="lightyellow", edgecolor="navy",
                 linewidth=4, frameon=True)
ax = fig.add_subplot(111)
ax.plot([1, 3, 2, 4])
ax.set_title("Framed figure")
fig.savefig("framed.png", edgecolor=fig.get_edgecolor())

fig2 = plt.figure(figsize=(5, 3), frameon=False)
ax2 = fig2.add_subplot()
ax2.bar([1, 2, 3], [3, 1, 2])
fig2.savefig("frameless.png")
print(fig.get_linewidth(), fig.get_frameon(), fig2.get_frameon())
