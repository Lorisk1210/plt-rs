# Set the default figure size and dpi once with rcParams for a whole notebook of plots.
import numpy as np
import matplotlib as mpl
import matplotlib.pyplot as plt

mpl.rcParams["figure.figsize"] = (7, 3)
mpl.rcParams["figure.dpi"] = 110
mpl.rcParams["savefig.dpi"] = 200

for i, freq in enumerate([1, 2]):
    fig = plt.figure()
    t = np.linspace(0, 1, 300)
    plt.plot(t, np.sin(2 * np.pi * freq * t))
    plt.title(f"{freq} Hz")
    print(fig.get_size_inches(), fig.dpi)
    fig.savefig(f"wave_{freq}.png")

print(mpl.rcParams["savefig.dpi"])
