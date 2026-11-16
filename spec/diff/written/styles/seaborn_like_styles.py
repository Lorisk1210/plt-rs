# Compare the seaborn-v0_8 family of sheets (paper, notebook, poster) in one figure via context.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 2 * np.pi, 50)
contexts = ["seaborn-v0_8-paper", "seaborn-v0_8-notebook", "seaborn-v0_8-poster"]
for ctx in contexts:
    with plt.style.context(["seaborn-v0_8-ticks", ctx]):
        fig, ax = plt.subplots(figsize=(4, 3))
        ax.plot(x, np.sin(x), label="sin")
        ax.scatter(x[::5], np.cos(x[::5]), label="cos")
        ax.legend()
        ax.set_title(ctx.split("-")[-1])
        fig.savefig(f"{ctx}.png")
        print(ctx, plt.rcParams["font.size"], plt.rcParams["lines.linewidth"])
        plt.close(fig)
