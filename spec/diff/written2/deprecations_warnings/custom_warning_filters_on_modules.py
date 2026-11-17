# warnings.filterwarnings by module regex and message for matplotlib submodules; once/default actions and repeated calls counted.
import warnings
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
for action in ("once", "default", "always", "module"):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter(action)
        for k in range(3):
            ax.set_xlim(k, k)
        for k in range(2):
            ax.legend()
    print(action, len(caught), sorted({str(w.message)[:30] for w in caught}))
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    warnings.filterwarnings("ignore", module=r"matplotlib\.axes.*")
    ax.set_xlim(5, 5)
    ax.legend()
print("module filter", len(caught), [w.category.__name__ for w in caught])
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    warnings.filterwarnings("ignore", message="No artists with labels")
    ax.legend()
    ax.set_xlim(6, 6)
print("message filter", [str(w.message)[:30] for w in caught])
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    import logging
    logging.getLogger("matplotlib").setLevel(logging.ERROR)
    ax.set_xlim(7, 7)
print("logging level", len(caught))
print(matplotlib._api.warn_external.__module__, callable(matplotlib._api.warn_deprecated))
