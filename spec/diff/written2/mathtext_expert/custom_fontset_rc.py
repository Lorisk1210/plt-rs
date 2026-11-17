# mathtext.fontset "custom" with mathtext.rm/it/bf/sf/tt/cal set to font families, fallback set to each choice; extents compared.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

base = {"mathtext.fontset": "custom", "mathtext.rm": "DejaVu Serif", "mathtext.it": "DejaVu Serif:italic",
        "mathtext.bf": "DejaVu Serif:bold", "mathtext.sf": "DejaVu Sans", "mathtext.tt": "DejaVu Sans Mono",
        "mathtext.cal": "DejaVu Sans:italic"}
expr = r"$\mathrm{rm}\ \mathit{it}\ \mathbf{bf}\ \mathsf{sf}\ \mathtt{tt}\ \mathcal{CAL}\ x^2 \alpha \sum$"
for fallback in ("cm", "stix", "stixsans", None):
    rc = dict(base, **{"mathtext.fallback": fallback})
    with plt.rc_context(rc):
        fig, ax = plt.subplots(figsize=(6, 1.5))
        t = ax.text(0.02, 0.4, expr, fontsize=14)
        fig.canvas.draw()
        e = t.get_window_extent()
        print(fallback, round(e.width, 2), round(e.height, 2), plt.rcParams["mathtext.fallback"])
        fig.savefig(f"custom_{fallback}.png")
        plt.close(fig)
with plt.rc_context({"mathtext.fontset": "custom", "mathtext.rm": "STIXGeneral"}):
    fig, ax = plt.subplots(figsize=(3, 1))
    t = ax.text(0.1, 0.4, r"$\mathrm{STIX}\ 123$", fontsize=12)
    fig.canvas.draw()
    print(round(t.get_window_extent().width, 2))
    plt.close(fig)
for bad in ("nope", "DejaVu"):
    try:
        matplotlib.rcParams["mathtext.fallback"] = bad
    except ValueError as err:
        print("ValueError", str(err)[:70])
print(matplotlib.rcParams["mathtext.fallback"], matplotlib.rcParams["mathtext.default"])
