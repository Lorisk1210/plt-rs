"""The parts of ``matplotlib.cbook`` pltrs reproduces, ported from
matplotlib's own ``tests/test_cbook.py``: box plot statistics, step
conversions, contiguous regions, comment stripping in rc files, the
mathtext stripping of formatters, the label formats of ``bar_label`` and
the patch perimeters of ``plot_surface``.

Area ``mpl_cbook``.

Ops
---
``cbook.boxplot_stats``  {X, X_ndarray?, whis?, labels?, autorange?} ->
    one dict of statistics per dataset (``mean, iqr, cilo, cihi, whishi,
    whislo, fliers, q1, med, q3`` plus ``label`` when labels are given);
    ``X`` is a list of datasets, or with ``X_ndarray`` a 2-D array whose
    columns are the datasets. ``ValueError`` when the labels do not match
    the datasets.
``cbook.pts_to_prestep`` / ``pts_to_poststep`` / ``pts_to_midstep``  {x,
    ys} -> ``f(x, *ys)`` as rows ``[x_out, y1_out, ...]``; ``ValueError``
    for mismatched lengths.
``cbook.contiguous_regions``  {mask} -> the ``[start, stop]`` index ranges
    of the runs of ``True``.
``cbook._strip_comment``  {line} -> the line without its comment (from the
    first ``#`` outside double quotes), stripped; ``ValueError`` for an
    unclosed double quote.
``cbook.strip_math``  {s} -> ``s`` without the ``$...$`` wrapper and the
    common TeX commands.
``cbook._auto_format_str``  {fmt, value} -> ``fmt % value``, or
    ``fmt.format(value)`` where ``%`` formatting fails.
``cbook._array_patch_perimeters``  {rows, cols, rstride, cstride} ->
    ``_array_patch_perimeters(x, rstride, cstride)`` for ``x =
    np.arange(rows * cols).reshape(rows, cols)``.

Cases
-----
Named after the upstream tests. ``Test_boxplot_stats`` draws its data from
``spec_support.rng`` (lognormal: ``exp(1.5 + 1.75 * normal)``, 37 x 4) and
labels with strings only; ``test_step_fails`` keeps the length mismatch;
``test_array_patch_perimeters`` probes every stride pair of its grids.

Left out
--------
``test_results_bootstrapped`` (numpy's global random generator),
``test_bad_dims`` and ``test_reshape2d*`` (pltrs takes datasets as a typed
slice of sequences, so there is no shape normalisation to port; the
``stats`` probes cover the flat and nested conventions),
``Test_delete_masked_points`` (generic over Python sequence types; pltrs
drops non-finite points inside quiver, hexbin and scatter, covered by
their probes), ``Test_callback_registry`` and the ``callbackregistry``
tests (pltrs' event registry is keyed by a typed event kind and holds
Rust closures: no weak references, pickling, exception handlers, signal
names or blocking contexts), ``test_normalize_kwargs_*`` (Python keyword
aliases; pltrs uses typed property builders), ``test_grouper*`` (pltrs'
grouper is internal to axis sharing, which the ``axes_behaviour`` probes
cover), ``test_resize_sequence`` (contour colours are cycled inline),
``test_warn_external*``, ``test_sanitize_sequence``, ``test_flatiter``,
``test__safe_first_finite*``, ``test_safe_first_element_*``,
``test_setattr_cm``, ``test_unpack_to_numpy_*`` (Python utilities without a
counterpart), ``test_format_approx`` (only used by the Type 1 font code,
which pltrs does not have), the pandas and xarray tests.
"""

import numpy as np
from matplotlib import cbook

from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_cbook")


def _boxplot_stats(X, X_ndarray=False, **kw):
    return [dict(s) for s in cbook.boxplot_stats(np.asarray(X) if X_ndarray else X, **kw)]


def bxp(**args):
    probes.probe("cbook.boxplot_stats", _boxplot_stats, **args)


def _lognormal_data():
    return np.exp(1.5 + 1.75 * rng(937).normal(37 * 4)).reshape(37, 4).tolist()


def test_boxplot_stats():
    data = _lognormal_data()
    # Test_boxplot_stats.test_form_main_list, test_form_each_dict,
    # test_form_dict_keys and test_results_baseline
    bxp(X=data, X_ndarray=True)
    # test_results_whiskers_float
    bxp(X=data, X_ndarray=True, whis=3)
    # test_results_whiskers_range
    bxp(X=data, X_ndarray=True, whis=[0, 100])
    # test_results_whiskers_percentiles
    bxp(X=data, X_ndarray=True, whis=[5, 95])
    # test_results_withlabels
    bxp(X=data, X_ndarray=True, labels=["Test1", "2", "Aardvark", "4"])
    # test_label_error
    bxp(X=data, X_ndarray=True, labels=["1", "2"])


def test_boxplot_stats_autorange_false():
    x = [-25.0] + [0.0] * 140 + [25.0]
    for autorange in (False, True):
        bxp(X=x, autorange=autorange)


def _steps(name):
    fn = getattr(cbook, name)
    return lambda x, ys: np.asarray(fn(x, *ys)).tolist()


def _probe_steps(name):
    x = list(range(4))
    y1 = list(range(4))
    for ys in ([y1, y1[::-1]], [y1]):
        probes.probe(f"cbook.{name}", _steps(name), x=x, ys=ys)


def _probe_empty_steps(name):
    for ys in ([], [[]]):
        probes.probe(f"cbook.{name}", _steps(name), x=[], ys=ys)


def test_to_prestep():
    _probe_steps("pts_to_prestep")


def test_to_prestep_empty():
    _probe_empty_steps("pts_to_prestep")


def test_to_poststep():
    _probe_steps("pts_to_poststep")


def test_to_poststep_empty():
    _probe_empty_steps("pts_to_poststep")


def test_to_midstep():
    _probe_steps("pts_to_midstep")


def test_to_midstep_empty():
    _probe_empty_steps("pts_to_midstep")


def test_step_fails():
    probes.probe("cbook.pts_to_prestep", _steps("pts_to_prestep"),
                 x=list(range(12)), ys=[list(range(3))])


def test_contiguous_regions():
    a, b, c, d, e = 3, 4, 5, 6, 7
    mask = [True] * a + [False] * b + [True] * c
    for case in (mask, mask + [False] * e, [False] * d + mask, [False] * d + mask + [False] * e,
                 [False] * 5, []):
        probes.probe("cbook.contiguous_regions",
                     lambda mask: [list(r) for r in cbook.contiguous_regions(mask)], mask=case)


def _strip_comment(line):
    return cbook._strip_comment(line)


def test_strip_comment():
    for line in ['a : no_comment', 'a : "quoted str"', 'a : "quoted str" # comment',
                 'a : "#000000"', 'a : "#000000" # comment', 'a : ["#000000", "#FFFFFF"]',
                 'a : ["#000000", "#FFFFFF"] # comment', 'a : val  # a comment "with quotes"',
                 '# only comment "with quotes" xx',
                 # Single quotes do not protect a '#'.
                 "a : cycler('color', ['#E24A33'])", "  padded : value  ", ""]:
        probes.probe("cbook._strip_comment", _strip_comment, line=line)


def test_strip_comment_invalid():
    probes.probe("cbook._strip_comment", _strip_comment, line='grid.color: "aa')


def test_strip_math():
    for s in [r"1 \times 2", r"$1 \times 2$", r"$\rm{hi}$", r"$\mathdefault{10^{3}}$", "$",
              "$$", "a$b$"]:
        probes.probe("cbook.strip_math", cbook.strip_math, s=s)


def test_auto_format_str():
    for fmt, value in [("%.2f m", 0.2), ("{:.2f} m", 0.2), ("{} m", 0.2), ("const", 0.2),
                       ("%d or {}", 0.2), ("{{{:,.0f}}}", 2e5), ("{:.2%}", 2 / 3),
                       ("$%g", 2.54)]:
        probes.probe("cbook._auto_format_str", cbook._auto_format_str, fmt=fmt, value=value)


def _perimeters(rows, cols, rstride, cstride):
    x = np.arange(rows * cols).reshape(rows, cols)
    return cbook._array_patch_perimeters(x, rstride=rstride, cstride=cstride).tolist()


def test_array_patch_perimeters():
    def divisors(n):
        return [i for i in range(1, n + 1) if n % i == 0]

    for rows, cols in [(5, 5), (7, 14), (13, 9)]:
        for rstride in divisors(rows - 1):
            for cstride in divisors(cols - 1):
                probes.probe("cbook._array_patch_perimeters", _perimeters, rows=rows,
                             cols=cols, rstride=rstride, cstride=cstride)
