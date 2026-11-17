# Snapshot of matplotlib's registries as an expert inspects them: scales, projections, colormaps, color sequences, styles.
import matplotlib as mpl
mpl.use("agg")
from matplotlib import scale, projections, style
import matplotlib.markers as mmarkers
import matplotlib.patches as mpatches
import matplotlib.lines as mlines

print(sorted(scale.get_scale_names()))
print(sorted(projections.get_projection_names()))
print(len(list(mpl.colormaps)) > 150, sorted(list(mpl.colormaps))[:5])
print(sorted(mpl.color_sequences)[:6])
print(sorted(style.available)[:5], len(style.available) >= 28)
print(sorted(mpatches.BoxStyle.get_styles())[:6])
print(sorted(mpatches.ArrowStyle.get_styles())[:8])
print(sorted(mpatches.ConnectionStyle.get_styles()))
print(len(mmarkers.MarkerStyle.markers), sorted(k for k in mmarkers.MarkerStyle.markers if isinstance(k, str) and len(k) == 1)[:12])
print(sorted(mmarkers.MarkerStyle.fillstyles))
print(sorted(mlines.Line2D.lineStyles), sorted(mlines.Line2D.drawStyles))
print(mpl.rcParams["axes.prop_cycle"].by_key()["color"][:3])
from matplotlib.backends import backend_registry, BackendFilter
print(mpl.get_backend().lower(), sorted(backend_registry.list_builtin(BackendFilter.NON_INTERACTIVE))[:5])
print("agg" in backend_registry.list_builtin(), backend_registry.is_valid_backend("svg"), backend_registry.is_valid_backend("nope"))
from matplotlib import ticker
print(sorted(n for n in ticker.__all__ if n.endswith("Locator"))[:6])
from matplotlib import colors
print(len(colors.CSS4_COLORS), len(colors.TABLEAU_COLORS), len(colors.XKCD_COLORS), colors.BASE_COLORS["r"])
print(sorted(mpl.rcParams["axes.prop_cycle"].keys))
print(type(mpl.colormaps).__name__, type(mpl.color_sequences).__name__)
print(sorted(n for n in dir(mpatches) if n.endswith("Style"))[:5])
print(style.library["ggplot"]["axes.facecolor"])
