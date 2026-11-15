"""
Scenes of figures that libraries make: one typical call of each plotting
library of the ecosystem's venvs (``cargo xtask ecosystem setup``), the
figure it made drawn, saved and dropped as the harness's own scenes are.
A web service rendering seaborn, pandas or plotnine figures in a loop is
where a leak hurts most, and these libraries subclass, patch and cache.

A library scene runs in the venv its library is installed in: under
pltrs that venv (which has no matplotlib: ``python/alias`` gives pltrs
as matplotlib), under matplotlib the venv beside it holding matplotlib
(``reference-venv``). Its statements name pltrs, as the harness's own
do; the library imports matplotlib by its own name.
"""

import pathlib

from .scenes import ROOT, Scene

#: The ecosystem's venvs (``cargo xtask ecosystem setup``).
ECOSYSTEM = ROOT / "target" / "ecosystem"

#: The package metadata of matplotlib that pltrs's runs of the libraries
#: put on ``PYTHONPATH`` (some read matplotlib's version from it).
METADATA = ECOSYSTEM / "metadata"

#: The directory of the venvs each library is installed in, under
#: `ECOSYSTEM` ("": the shared ones).
VENVS = {
    **dict.fromkeys(("seaborn", "pandas", "xarray", "plotnine", "statsmodels", "networkx",
                     "geopandas", "cartopy", "astropy", "mplfinance", "missingno",
                     "matplotlib-venn", "scikit-learn", "nilearn", "sympy",
                     "mpl-scatter-density", "shap"), ""),
    **dict.fromkeys(("ultraplot", "mpltern", "metpy", "windrose", "squarify",
                     "pycirclize", "mplhep", "matplotlib-label-lines", "matplotlib-scalebar",
                     "python-ternary", "mlxtend", "scienceplots", "matplot2tikz"),
                    "venvs/addons"),
    **dict.fromkeys(("corner", "mplsoccer", "mplstereonet", "pywaffle", "cmocean"),
                    "venvs/fields"),
    "lifelines": "venvs/lifelines",
    "plottable": "venvs/plottable",
    "yellowbrick": "venvs/yellowbrick",
}


def interpreter(library, reference):
    """The Python of the venv *library* is installed in, with matplotlib
    (*reference*) or without."""
    venv = "reference-venv" if reference else "venv"
    return ECOSYSTEM / VENVS[library] / venv / "bin" / "python"


def available(library):
    """Whether the venvs of *library* are set up."""
    return all(pathlib.Path(interpreter(library, reference)).exists()
               for reference in (False, True))


def _scene(library, packages, *statements):
    """A dropped scene of *library*, whose objects of *packages* must go
    with its figure."""
    return Scene(statements, library=library, packages=packages)


#: A frame of numbers, as the pandas-based libraries take it.
_FRAME = ("import pandas as pd",
          "frame = pd.DataFrame({'a': np.linspace(0, 1, 20), 'b': np.cos(np.linspace(0, 6, 20)),"
          " 'c': np.repeat(['x', 'y'], 10)})")

#: The library scenes, by name.
LIBRARY_SCENES = {
    "lib_seaborn_relplot": _scene(
        "seaborn", ("seaborn",), *_FRAME, "import seaborn as sns",
        "grid = sns.relplot(data=frame, x='a', y='b', hue='c')"),
    "lib_seaborn_heatmap": _scene(
        "seaborn", ("seaborn",), "import seaborn as sns", "fig, ax = plt.subplots()",
        "sns.heatmap(field, ax=ax, annot=False)"),
    "lib_pandas_plot": _scene(
        "pandas", ("pandas",), *_FRAME, "ax = frame.plot(x='a', y='b')"),
    "lib_pandas_hist": _scene(
        "pandas", ("pandas",), *_FRAME, "axes_ = frame.hist(column='b')"),
    "lib_xarray_plot": _scene(
        "xarray", ("xarray",), "import xarray as xr",
        "array = xr.DataArray(field, dims=('y', 'x'), name='field')",
        "made = array.plot()"),
    "lib_plotnine": _scene(
        "plotnine", ("plotnine",), *_FRAME,
        "from plotnine import ggplot, aes, geom_point",
        "fig = (ggplot(frame, aes('a', 'b', color='c')) + geom_point()).draw()"),
    "lib_statsmodels_qqplot": _scene(
        "statsmodels", ("statsmodels",), "import statsmodels.api as sm",
        "fig = sm.qqplot(np.sin(np.linspace(0, 9, 40)), line='s')"),
    "lib_networkx_draw": _scene(
        "networkx", ("networkx",), "import networkx as nx", "fig, ax = plt.subplots()",
        "nx.draw(nx.cycle_graph(6), pos=nx.circular_layout(nx.cycle_graph(6)), ax=ax)"),
    "lib_geopandas_plot": _scene(
        "geopandas", ("geopandas",), "import geopandas as gpd",
        "from shapely.geometry import Point",
        "frame = gpd.GeoDataFrame({'v': values}, geometry=[Point(a, b) for a, b in zip(x, y)])",
        "ax = frame.plot(column='v')"),
    "lib_cartopy_axes": _scene(
        "cartopy", ("cartopy",), "import cartopy.crs as ccrs", "fig = plt.figure()",
        "ax = fig.add_subplot(projection=ccrs.Robinson())",
        "ax.plot(x * 20, y * 20, transform=ccrs.PlateCarree())", "ax.gridlines()",
        "ax.set_global()"),
    "lib_astropy_wcsaxes": _scene(
        "astropy", ("astropy",), "from astropy.wcs import WCS",
        "wcs = WCS(naxis=2)", "wcs.wcs.ctype = ['RA---TAN', 'DEC--TAN']",
        "fig = plt.figure()", "ax = fig.add_subplot(projection=wcs)", "ax.imshow(field)"),
    "lib_mplfinance": _scene(
        "mplfinance", ("mplfinance",), "import pandas as pd", "import mplfinance as mpf",
        "days = pd.date_range('2024-01-01', periods=12, freq='D')",
        "bars = pd.DataFrame({'Open': np.linspace(1, 2, 12), 'High': np.linspace(1.5, 2.5, 12),"
        " 'Low': np.linspace(0.5, 1.5, 12), 'Close': np.linspace(1.2, 2.2, 12)}, index=days)",
        "fig, axes_ = mpf.plot(bars, type='candle', returnfig=True)"),
    "lib_missingno": _scene(
        "missingno", ("missingno",), "import pandas as pd", "import missingno as msno",
        "frame = pd.DataFrame({'a': [1.0, None, 3.0, 4.0], 'b': [None, 2.0, None, 4.0]})",
        "ax = msno.matrix(frame)"),
    "lib_matplotlib_venn": _scene(
        "matplotlib-venn", ("matplotlib_venn",), "from matplotlib_venn import venn2",
        "fig, ax = plt.subplots()", "diagram = venn2((10, 6, 3), ax=ax)"),
    "lib_sklearn_confusion": _scene(
        "scikit-learn", ("sklearn",), "from sklearn.metrics import ConfusionMatrixDisplay",
        "display = ConfusionMatrixDisplay(np.array([[5, 1], [2, 7]])).plot()"),
    "lib_nilearn_matrix": _scene(
        "nilearn", ("nilearn",), "from nilearn import plotting",
        "display = plotting.plot_matrix(np.eye(4) * 0.5 + 0.1)"),
    "lib_sympy_plot": _scene(
        "sympy", ("sympy",), "import sympy",
        "made = sympy.plot(sympy.Symbol('s') ** 2, show=False)",
        "made.save(io.BytesIO())"),
    "lib_mpl_scatter_density": _scene(
        "mpl-scatter-density", ("mpl_scatter_density",), "import mpl_scatter_density",
        "fig = plt.figure()", "ax = fig.add_subplot(projection='scatter_density')",
        "ax.scatter_density(np.linspace(0, 1, 500), np.sin(np.linspace(0, 9, 500)))"),
    "lib_shap_bar": _scene(
        "shap", ("shap",), "import shap",
        "explanation = shap.Explanation(values=np.outer(np.ones(5), values[:3]),"
        " feature_names=['p', 'q', 'r'])",
        "shap.plots.bar(explanation, show=False)"),
    "lib_ultraplot": _scene(
        "ultraplot", ("ultraplot",), "import ultraplot as uplt",
        "fig, axs = uplt.subplots(ncols=2)", "axs.plot(x, y)", "axs.format(title='t')"),
    "lib_mpltern": _scene(
        "mpltern", ("mpltern",), "import mpltern", "fig = plt.figure()",
        "ax = fig.add_subplot(projection='ternary')",
        "ax.plot([0.2, 0.5, 0.3], [0.3, 0.2, 0.5], [0.5, 0.3, 0.2])"),
    "lib_metpy_skewt": _scene(
        "metpy", ("metpy",), "from metpy.plots import SkewT", "fig = plt.figure()",
        "skew = SkewT(fig)", "skew.plot(np.linspace(1000, 300, 8), np.linspace(20, -40, 8), 'r')"),
    "lib_windrose": _scene(
        "windrose", ("windrose",), "from windrose import WindroseAxes",
        "ax = WindroseAxes.from_ax()",
        "ax.bar(np.linspace(0, 350, 36), np.linspace(1, 10, 36), normed=True)"),
    "lib_squarify": _scene(
        "squarify", ("squarify",), "import squarify", "fig, ax = plt.subplots()",
        "squarify.plot(sizes=[5, 3, 2, 1], label=list('abcd'), ax=ax)"),
    "lib_scienceplots": _scene(
        "scienceplots", ("scienceplots",), "import scienceplots",
        "with plt.style.context(['science', 'no-latex']):\n"
        "    fig, ax = plt.subplots()\n    ax.plot(x, y, label='sin')\n    ax.legend()"),
    "lib_pycirclize": _scene(
        "pycirclize", ("pycirclize",), "from pycirclize import Circos",
        "circos = Circos({'A': 10, 'B': 15})",
        "[sector.text(sector.name) for sector in circos.sectors]",
        "fig = circos.plotfig()"),
    "lib_mplhep": _scene(
        "mplhep", ("mplhep",), "import mplhep", "fig, ax = plt.subplots()",
        "mplhep.histplot(values, np.arange(8), ax=ax)"),
    "lib_label_lines": _scene(
        "matplotlib-label-lines", ("labellines",), "from labellines import labelLines",
        "fig, ax = plt.subplots()", "ax.plot(x, y, label='one')", "ax.plot(x, -y, label='two')",
        "labelLines(ax.get_lines())"),
    "lib_scalebar": _scene(
        "matplotlib-scalebar", ("matplotlib_scalebar",),
        "from matplotlib_scalebar.scalebar import ScaleBar", "fig, ax = plt.subplots()",
        "ax.imshow(field)", "ax.add_artist(ScaleBar(0.5, 'um'))"),
    "lib_python_ternary": _scene(
        "python-ternary", ("ternary",), "import ternary",
        "fig, tax = ternary.figure(scale=10)", "tax.boundary()",
        "tax.plot([(1, 2, 7), (3, 3, 4)])"),
    "lib_mlxtend_regions": _scene(
        "mlxtend", ("mlxtend",), "from mlxtend.plotting import plot_decision_regions",
        "class Model:\n    def predict(self, X):\n        return (X[:, 0] > 0.5).astype(int)",
        "points = np.column_stack([np.linspace(0, 1, 10), np.linspace(1, 0, 10)])",
        "ax = plot_decision_regions(points, (points[:, 0] > 0.5).astype(int), clf=Model())"),
    "lib_matplot2tikz": _scene(
        "matplot2tikz", ("matplot2tikz",), "import matplot2tikz", "fig, ax = plt.subplots()",
        "ax.plot(x, y)", "code = matplot2tikz.get_tikz_code(fig)"),
    "lib_corner": _scene(
        "corner", ("corner",), "import corner",
        "fig = corner.corner(np.column_stack([np.sin(np.linspace(0, 9, 200)),"
        " np.cos(np.linspace(0, 7, 200))]))"),
    "lib_mplsoccer": _scene(
        "mplsoccer", ("mplsoccer",), "from mplsoccer import Pitch",
        "fig, ax = Pitch().draw()"),
    "lib_mplstereonet": _scene(
        "mplstereonet", ("mplstereonet",), "import mplstereonet",
        "fig, ax = mplstereonet.subplots()", "ax.plane(315, 30)", "ax.grid()"),
    "lib_pywaffle": _scene(
        "pywaffle", ("pywaffle",), "from pywaffle import Waffle",
        "fig = plt.figure(FigureClass=Waffle, rows=4, values=[6, 3, 2])"),
    "lib_cmocean": _scene(
        "cmocean", ("cmocean",), "import cmocean", "fig, ax = plt.subplots()",
        "ax.imshow(field, cmap=cmocean.cm.thermal)"),
    "lib_lifelines": _scene(
        "lifelines", ("lifelines",), "from lifelines import KaplanMeierFitter",
        "ax = KaplanMeierFitter().fit(np.linspace(1, 20, 20)).plot_survival_function()"),
    "lib_plottable": _scene(
        "plottable", ("plottable",), *_FRAME, "from plottable import Table",
        "fig, ax = plt.subplots()", "table_ = Table(frame.head(5), ax=ax)"),
    "lib_yellowbrick": _scene(
        "yellowbrick", ("yellowbrick",), "from yellowbrick.features import Rank1D",
        "fig, ax = plt.subplots()", "ranks = Rank1D(ax=ax)",
        "ranks.fit_transform(np.column_stack([np.linspace(0, 1, 20), np.cos(np.linspace(0, 6, 20))]))",
        "ranks.finalize()"),
}


def library_scenes():
    """The library scenes, by name."""
    return dict(LIBRARY_SCENES)
