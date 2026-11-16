# Switch the current axes with plt.sca and draw with pyplot functions into a chosen subplot.
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 3, figsize=(9, 3))
for i, ax in enumerate(axs):
    plt.sca(ax)
    plt.plot([0, 1], [0, i + 1])
    plt.title(f"panel {i}")
plt.sca(axs[0])
plt.xlabel("first")
print(plt.gca() is axs[0], plt.gcf() is fig)
plt.subplot(1, 3, 2)
plt.ylabel("second (subplot reselects)")
print(plt.gca() is axs[1], len(fig.axes))
plt.savefig("switched.png")
