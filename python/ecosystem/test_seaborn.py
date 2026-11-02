"""seaborn's figure-level and axes-level functions, and its objects
interface, on pltrs as matplotlib."""

import numpy as np
import pytest

sns = pytest.importorskip("seaborn")
pytest.importorskip("scipy")


def test_relplot(check, frame):
    check(sns.relplot(data=frame, x="x", y="y", hue="cat", size="size"))


def test_relplot_lines(check, frame):
    check(sns.relplot(data=frame, x="x", y="y", hue="hue", kind="line"))


def test_lineplot_error_bands(check, frame):
    check(sns.lineplot(data=frame.assign(xr=frame.x.round()), x="xr", y="y",
                       hue="hue", seed=0))


def test_scatterplot_styles(check, frame):
    check(sns.scatterplot(data=frame, x="x", y="y", hue="cat", style="hue",
                          size="size"))


def test_displot(check, frame):
    check(sns.displot(data=frame, x="y", hue="cat"))


def test_displot_kde(check, frame):
    check(sns.displot(data=frame, x="y", kind="kde", hue="cat"))


def test_displot_ecdf(check, frame):
    check(sns.displot(data=frame, x="y", kind="ecdf"))


def test_histplot_stacked(check, frame):
    check(sns.histplot(data=frame, x="y", hue="cat", multiple="stack"))


def test_kdeplot_bivariate(check, frame):
    check(sns.kdeplot(data=frame, x="y", y="z", fill=True))


def test_rugplot(check, frame):
    sns.kdeplot(data=frame, x="y")
    check(sns.rugplot(data=frame, x="y"))


@pytest.mark.parametrize("kind", ["strip", "swarm", "box", "violin", "bar",
                                  "count", "boxen", "point"])
def test_catplot(check, frame, kind):
    options = {"seed": 0} if kind in ("bar", "point") else {}
    y = None if kind == "count" else "y"
    check(sns.catplot(data=frame, x="cat", y=y, kind=kind, **options))


def test_catplot_with_hue(check, frame):
    check(sns.catplot(data=frame, x="cat", y="y", kind="box", hue="hue"))


def test_violinplot_split(check, frame):
    check(sns.violinplot(data=frame, x="cat", y="y", hue="hue", split=True))


def test_barplot_palette(check):
    check(sns.barplot(x=list("abcd"), y=[3, 4, 2, 5], hue=list("abcd"),
                      palette="husl", legend=False))


def test_heatmap(check, small):
    check(sns.heatmap(small, annot=True))


def test_heatmap_masked(check, frame):
    corr = np.corrcoef(frame[["y", "z", "w", "x"]].T.values)
    check(sns.heatmap(corr, mask=np.triu(np.ones((4, 4), bool)), cmap="vlag",
                      center=0, annot=True, fmt=".2f"))


def test_clustermap(check, frame):
    check(sns.clustermap(frame[["y", "z", "w"]].iloc[:20]))


def test_pairplot(check, frame):
    check(sns.pairplot(frame[["y", "z", "w", "cat"]], hue="cat"))


def test_pairgrid(check, frame):
    grid = sns.PairGrid(frame[["y", "z", "w"]])
    check(grid.map_upper(sns.scatterplot).map_lower(sns.kdeplot)
          .map_diag(sns.histplot))


def test_facetgrid(check, frame):
    check(sns.FacetGrid(frame, col="cat", row="hue").map(sns.histplot, "y"))


def test_lmplot(check, frame):
    check(sns.lmplot(data=frame, x="x", y="y", hue="hue", seed=0))


def test_residplot(check, frame):
    check(sns.residplot(data=frame, x="x", y="y"))


@pytest.mark.parametrize("kind", ["scatter", "kde", "hex", "reg"])
def test_jointplot(check, frame, kind):
    options = {"joint_kws": {"seed": 0}} if kind == "reg" else {}
    check(sns.jointplot(data=frame, x="y", y="z", kind=kind, **options))


def test_set_theme(check, frame):
    sns.set_theme()
    try:
        check(sns.lineplot(data=frame, x="x", y="y"))
    finally:
        sns.reset_orig()


def test_objects_interface(check, frame):
    import seaborn.objects as so
    plot = (so.Plot(frame, x="x", y="y", color="hue")
            .add(so.Dot()).add(so.Line(), so.PolyFit()))
    check(plot.plot())
