# Run a normal plotting script with warnings turned into errors; which call fails first, and the same script with filters.
import warnings
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def script():
    fig, ax = plt.subplots(figsize=(4, 3))
    x = np.linspace(0, 1, 20)
    ax.plot(x, x ** 2, label="square")
    ax.set_yscale("log")
    ax.legend()
    ax.set_xlim(0.5, 0.5)
    fig.tight_layout()
    fig.savefig("strict.png")
    plt.close(fig)
    return "finished"


for mode in ("default", "error", "ignore"):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter(mode)
        try:
            print(mode, script())
        except Exception as e:
            print(mode, type(e).__name__, str(e).splitlines()[0][:60])
    print(mode, len(caught), sorted({w.category.__name__ for w in caught}))
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    warnings.filterwarnings("ignore", message="Attempting to set identical")
    print(script(), [str(w.message)[:40] for w in caught])
with warnings.catch_warnings():
    warnings.filterwarnings("error", category=UserWarning, module="matplotlib")
    try:
        print(script())
    except UserWarning as e:
        print("module filter", str(e)[:50])
