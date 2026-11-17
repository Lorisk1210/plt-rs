"""
R and K, plotting methods: what every plotting method returns and what
its containers hold, made from ints (and from floats of other widths where
matplotlib keeps them).
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs.sankey import Sankey

#: An int signal for the spectra.
SIGNAL = np.round(10 * np.sin(np.arange(64) / 3)).astype(int)


def _message(call):
    """The message of the exception *call()* raises (None if it returns)."""
    try:
        call()
    except Exception as error:  # noqa: BLE001 - the message is the row
        return str(error)
    return None


def artist_numbers(rows, surface, artist):
    """Rows for the numbers an artist made by a plotting method holds."""
    for name in ("get_xdata", "get_ydata", "get_xy", "get_offsets", "get_segments",
                 "get_array", "get_height", "get_width", "get_x", "get_y", "get_extent",
                 "get_positions", "get_position"):
        method = getattr(artist, name, None)
        if method is not None:
            rows.row(surface, f"{type(artist).__name__}.{name}()", method)
    if hasattr(artist, "get_paths") and not hasattr(artist, "get_xy"):
        rows.row(surface, f"{type(artist).__name__}.get_paths()[0].vertices",
                 lambda: artist.get_paths()[0].vertices)


def histograms(rows):
    """hist, hist2d, hexbin and stairs of ints."""
    fig, ax = plt.subplots()
    n, bins, bars = ax.hist([1, 2, 2, 3], bins=3)
    rows.row("R hist ints", "n", lambda: n)
    rows.row("R hist ints", "bins", lambda: bins)
    artist_numbers(rows, "R hist ints patch", bars[0])
    nw, binw, _ = ax.hist([1, 2, 2, 3], bins=[0, 2, 4])
    rows.row("R hist bins=ints", "n", lambda: nw)
    rows.row("R hist bins=ints", "bins", lambda: binw)
    for label, kwargs in [("weights=ints", dict(weights=[1, 2, 3, 4])),
                          ("density", dict(density=True)),
                          ("cumulative", dict(cumulative=True)),
                          ("float32", dict(x=np.float32([1, 2, 2, 3]))),
                          ("step", dict(histtype="step")),
                          ("two sets", dict(x=[[1, 2, 2], [3, 3, 4]], stacked=True))]:
        data = kwargs.pop("x", [1, 2, 2, 3])
        result = ax.hist(data, bins=3, **kwargs)
        rows.row(f"R hist {label}", "n", lambda r=result: r[0])
        rows.row(f"R hist {label}", "bins", lambda r=result: r[1])
    rows.row("R hist2d", "counts dtype", lambda: ax.hist2d([1, 2], [1, 2], bins=2)[0])
    counts, xedges, yedges, image = ax.hist2d([1, 2, 3], [1, 2, 2], bins=[2, 3])
    rows.row("R hist2d ints", "counts, xedges, yedges", lambda: (counts, xedges, yedges))
    rows.row("R hist2d ints", "image get_array()", image.get_array)
    hx = ax.hexbin([1, 2, 3], [1, 2, 3], gridsize=2)
    rows.row("R hexbin", "get_array()", hx.get_array)
    rows.row("R hexbin", "get_offsets()", hx.get_offsets)
    hc = ax.hexbin([1, 2, 3], [1, 2, 3], C=[1, 2, 3], gridsize=2, reduce_C_function=np.sum)
    rows.row("R hexbin C=ints sum", "get_array()", hc.get_array)
    hm = ax.hexbin([1, 2, 3], [1, 2, 3], gridsize=2, mincnt=1)
    rows.row("R hexbin mincnt", "get_array()", hm.get_array)
    rows.row("R stairs", "get_data()", lambda: ax.stairs([1, 2], [0, 1, 2]).get_data())
    rows.row("R stairs float32", "get_data()",
             lambda: ax.stairs(np.float32([1, 2]), np.float32([0, 1, 2])).get_data())
    rows.row("R stairs baseline", "get_data().baseline",
             lambda: ax.stairs([1, 2], baseline=1).get_data().baseline)


def statistics(rows):
    """boxplot, violinplot, errorbar, ecdf, stem and eventplot of ints."""
    fig, ax = plt.subplots()
    bp = ax.boxplot([[1, 2, 3, 4, 10]])
    rows.row("K boxplot", "keys", lambda: sorted(bp))
    for key in ("medians", "fliers", "whiskers", "caps", "boxes", "means"):
        for name in ("get_xdata", "get_ydata"):
            rows.row("R boxplot", f"{key}[0].{name}()",
                     lambda k=key, n=name: getattr(bp[k][0], n)())
    bp2 = ax.boxplot([[1, 2, 3, 4, 10]], showmeans=True, meanline=True, notch=True,
                     positions=[2], widths=1)
    for key in ("means", "boxes", "medians"):
        rows.row("R boxplot notch meanline", f"{key}[0].get_ydata()",
                 lambda k=key: bp2[k][0].get_ydata())
    rows.row("R boxplot float32", "fliers[0].get_ydata()",
             lambda: ax.boxplot([np.float32([1, 2, 3, 4, 10])])["fliers"][0].get_ydata())
    vp = ax.violinplot([[1, 2, 3, 4]], showmedians=True, showmeans=True, showextrema=True)
    rows.row("K violinplot", "keys", lambda: sorted(vp))
    for key in ("cmedians", "cmeans", "cmins", "cmaxes", "cbars"):
        rows.row("R violinplot", f"{key}.get_segments()", lambda k=key: vp[k].get_segments())
    rows.row("R violinplot", "bodies[0] vertices", lambda: vp["bodies"][0].get_paths()[0].vertices)
    st = ax.stem([1, 2], [3, 4])
    rows.row("R stem", "markerline.get_ydata()", st.markerline.get_ydata)
    rows.row("R stem", "markerline.get_xdata()", st.markerline.get_xdata)
    rows.row("R stem", "baseline.get_xdata()", st.baseline.get_xdata)
    rows.row("R stem", "baseline.get_ydata()", st.baseline.get_ydata)
    rows.row("R stem", "stemlines.get_segments()", st.stemlines.get_segments)
    eb = ax.errorbar([1, 2], [3, 4], yerr=1, xerr=[1, 2], capsize=2)
    rows.row("R errorbar", "data line get_ydata()", lambda: eb.lines[0].get_ydata())
    rows.row("R errorbar", "caplines", lambda: [c.get_ydata() for c in eb.lines[1]])
    rows.row("R errorbar", "barlinecols segments",
             lambda: [c.get_segments() for c in eb.lines[2]])
    rows.row("R errorbar", "has_xerr, has_yerr", lambda: (eb.has_xerr, eb.has_yerr))
    ec = ax.ecdf([3, 1, 2, 2])
    rows.row("R ecdf ints", "get_xdata()", ec.get_xdata)
    rows.row("R ecdf ints", "get_ydata()", ec.get_ydata)
    rows.row("R ecdf weights", "get_ydata()", lambda: ax.ecdf([3, 1, 2], weights=[1, 2, 1]).get_ydata())
    ev = ax.eventplot([[1, 2]], lineoffsets=1, linelengths=1)
    rows.row("R eventplot ints", "get_positions(), lineoffset",
             lambda: (ev[0].get_positions(), ev[0].get_lineoffset()))
    rows.row("R eventplot ints", "get_segments()", ev[0].get_segments)
    ev2 = ax.eventplot([np.array([1, 2]), np.array([3])], lineoffsets=[1, 2], linelengths=0.5)
    rows.row("R eventplot arrays", "get_positions()", lambda: [e.get_positions() for e in ev2])
    rows.row("R eventplot arrays", "get_lineoffset()", lambda: [e.get_lineoffset() for e in ev2])


def shapes(rows):
    """pie, bars, fills and spans of ints."""
    fig, ax = plt.subplots()
    wedges, ptexts, autotexts = ax.pie([1, 2, 3], autopct="%1.0f")
    rows.row("R pie", "theta1, theta2, r of wedge 0",
             lambda: (wedges[0].theta1, wedges[0].theta2, wedges[0].r))
    rows.row("R pie", "center", lambda: wedges[0].center)
    rows.row("R pie", "texts[0].get_position()", ptexts[0].get_position)
    rows.row("R pie", "autotexts[0].get_position()", autotexts[0].get_position)
    pw = ax.pie([1, 2], radius=2, center=(1, 1), startangle=90, explode=[0, 1])[0]
    rows.row("R pie ints given", "theta1, r, center", lambda: (pw[0].theta1, pw[0].r, pw[0].center))
    rows.row("R pie float32", "theta2 of wedge 0",
             lambda: ax.pie(np.float32([1, 3]))[0][0].theta2)
    rows.row("R broken_barh ints", "paths[0].vertices",
             lambda: ax.broken_barh([(1, 2)], (0, 1)).get_paths()[0].vertices)
    rows.row("R axline", "_xy1", lambda: ax.axline((0, 0), (1, 1))._xy1)
    rows.row("R axline slope", "_slope", lambda: ax.axline((0, 0), slope=2)._slope)
    rows.row("R axhspan", "get_xy()", lambda: ax.axhspan(1, 2).get_xy())
    rows.row("R axvspan", "get_xy()", lambda: ax.axvspan(1, 2).get_xy())
    rows.row("R fill", "get_xy()", lambda: ax.fill([0, 1, 1], [0, 0, 1])[0].get_xy())
    rows.row("R step", "get_ydata()", lambda: ax.step([1, 2], [3, 4])[0].get_ydata())
    rows.row("R stackplot", "get_paths()[0].vertices",
             lambda: ax.stackplot([1, 2], [3, 4])[0].get_paths()[0].vertices)
    rows.row("R fill_between ints", "get_paths()[0].vertices",
             lambda: ax.fill_between([1, 2], [3, 4], 1).get_paths()[0].vertices)
    rows.row("R fill_betweenx ints", "get_paths()[0].vertices",
             lambda: ax.fill_betweenx([1, 2], [3, 4]).get_paths()[0].vertices)
    bars = ax.bar([1, 2], [3, 4], bottom=1)
    artist_numbers(rows, "R bar ints bottom", bars[0])
    rows.row("R bar np.int64", "get_height()", lambda: ax.bar([1], np.array([3]))[0].get_height())
    rows.row("R bar float32", "get_height(), get_y()",
             lambda: (lambda b: (b.get_height(), b.get_y()))(ax.bar([1], np.float32([2.5]))[0]))
    rows.row("R barh ints", "get_width(), get_y()",
             lambda: (lambda b: (b.get_width(), b.get_y()))(ax.barh([1], [3])[0]))
    rows.row("R hlines", "get_segments()", lambda: ax.hlines([1, 2], 0, 3).get_segments())
    rows.row("R vlines", "get_segments()", lambda: ax.vlines([1, 2], 0, 3).get_segments())
    rows.row("R scatter ints", "get_offsets()", lambda: ax.scatter([1, 2], [3, 4]).get_offsets())
    rows.row("R plot", "get_xdata() of a y-only plot", lambda: ax.plot([3, 4])[0].get_xdata())
    rows.row("R plot", "get_xydata() of a y-only plot", lambda: ax.plot([3, 4])[0].get_xydata())


def fields(rows):
    """Contours, meshes, images, quiver, streamplot, Sankey and voxels."""
    fig, ax = plt.subplots()
    cs = ax.contour(np.arange(16).reshape(4, 4), levels=[2, 5, 9])
    rows.row("R contour int levels", "levels", lambda: cs.levels)
    rows.row("R contour int levels", "layers", lambda: cs.layers)
    rows.row("R contour int data", "zmin, zmax", lambda: (cs.zmin, cs.zmax))
    rows.row("R contour", "find_nearest_contour", lambda: cs.find_nearest_contour(1, 1, pixel=False))
    rows.row("R contour auto levels", "levels", lambda: ax.contour(np.arange(16).reshape(4, 4)).levels)
    rows.row("R contour float32", "levels, zmax",
             lambda: (lambda c: (c.levels, c.zmax))(ax.contour(np.float32(np.arange(16).reshape(4, 4)))))
    rows.row("R contour", "get_paths()[1].vertices", lambda: cs.get_paths()[1].vertices)
    cf = ax.contourf(np.arange(16).reshape(4, 4), levels=3)
    rows.row("R contourf", "levels", lambda: cf.levels)
    rows.row("R contourf", "layers", lambda: cf.layers)
    rows.row("R contourf", "_levels", lambda: cf._levels)
    rows.row("R contourf", "cvalues", lambda: cf.cvalues)
    rows.row("R contourf", "legend_elements colours",
             lambda: [h.get_facecolor() for h in cf.legend_elements()[0]])
    rows.row("R contour", "legend_elements colours",
             lambda: [h.get_color() for h in cs.legend_elements()[0]])
    rows.row("R contourf", "legend_elements labels", lambda: cf.legend_elements()[1])
    rows.row("R tricontour int levels", "levels",
             lambda: ax.tricontour([0, 1, 0, 1], [0, 0, 1, 1], [0, 1, 2, 3], levels=[1, 2]).levels)
    rows.row("R tricontourf", "levels",
             lambda: ax.tricontourf([0, 1, 0, 1], [0, 0, 1, 1], [0, 1, 2, 3]).levels)
    rows.row("R clabel manual rotation", "get_rotation()",
             lambda: ax.clabel(cs, manual=[(1, 1)])[0].get_rotation())
    labels = ax.clabel(cs)
    rows.row("R clabel", "get_position()", lambda: labels[0].get_position())
    rows.row("R clabel", "get_rotation()", lambda: labels[0].get_rotation())
    rows.row("R clabel", "labelCValueList", lambda: cs.labelCValueList)
    rows.row("R matshow", "get_array()", lambda: ax.matshow(np.eye(2, dtype=int)).get_array())
    rows.row("R spy", "get_array()", lambda: plt.figure().add_subplot().spy(np.eye(2)).get_array())
    rows.row("R spy markers", "get_ydata()",
             lambda: plt.figure().add_subplot().spy(np.eye(2), marker="o").get_ydata())
    rows.row("R pcolor ints", "get_array()", lambda: ax.pcolor(np.arange(4).reshape(2, 2)).get_array())
    rows.row("R pcolormesh float32", "get_array()",
             lambda: ax.pcolormesh(np.float32(np.arange(4).reshape(2, 2))).get_array())
    rows.row("R imshow float32", "get_array()",
             lambda: ax.imshow(np.float32(np.arange(4).reshape(2, 2))).get_array())
    rows.row("R imshow uint8", "get_array(), get_clim()",
             lambda: (lambda i: (i.get_array(), i.get_clim()))(ax.imshow(np.uint8([[0, 9], [3, 4]]))))
    rows.row("R pcolorfast ints", "get_array()",
             lambda: ax.pcolorfast(np.arange(6).reshape(2, 3)).get_array())
    rows.row("R pcolorfast nonuniform", "get_array()",
             lambda: ax.pcolorfast([0, 1, 3], [0, 2, 3], np.arange(4).reshape(2, 2)).get_array())
    q = ax.quiver([1, 2], [1, 2], [1, 0], [0, 1])
    rows.row("R quiver ints", "U, V, X", lambda: (q.U, q.V, q.X))
    rows.row("R quiver", "N, scale", lambda: (q.N, q.scale))
    rows.row("R quiver", "get_offsets()", q.get_offsets)
    rows.row("R barbs ints", "u, v", lambda: (lambda b: (b.u, b.v))(ax.barbs([1], [1], [10], [5])))
    sp = ax.streamplot(np.arange(3), np.arange(3), np.ones((3, 3)), np.ones((3, 3)), density=0.3)
    rows.row("R streamplot", "lines.get_segments()[0]", lambda: sp.lines.get_segments()[0])
    rows.row("R streamplot", "start point outside: the message",
             lambda: _message(lambda: ax.streamplot(np.arange(3), np.arange(3), np.ones((3, 3)),
                                                    np.ones((3, 3)), start_points=[[10, 10]])))
    sk = Sankey(ax=plt.figure().add_subplot()).add(flows=[1, -1], orientations=[0, 0]).finish()
    rows.row("R Sankey ints", "flows", lambda: sk[0].flows)
    rows.row("R Sankey", "angles", lambda: sk[0].angles)
    rows.row("R Sankey", "tips", lambda: sk[0].tips)
    rows.row("R Sankey", "text get_position()", lambda: sk[0].text.get_position())
    vx = plt.figure().add_subplot(projection="3d").voxels(np.ones((2, 1, 1), bool))
    rows.row("K voxels", "keys", lambda: sorted(vx))
    rows.row("K voxels float coordinates", "keys",
             lambda: sorted(plt.figure().add_subplot(projection="3d").voxels(
                 *np.indices((2, 2, 2)).astype(float), np.ones((1, 1, 1), bool))))


def spectra(rows):
    """specgram, psd, csd, cohere, magnitude/angle/phase spectra, acorr
    and xcorr of int signals."""
    fig, ax = plt.subplots()
    x = SIGNAL
    y = np.roll(SIGNAL, 3)
    spec, freqs, t, im = ax.specgram(x, NFFT=16, noverlap=8)
    rows.row("R specgram ints", "spectrum, freqs, t", lambda: (spec, freqs, t))
    rows.row("R specgram ints", "im get_extent()", im.get_extent)
    for name, call in [("psd", lambda: ax.psd(x, NFFT=16)),
                       ("psd return_line", lambda: ax.psd(x, NFFT=16, return_line=True)[:2]),
                       ("csd", lambda: ax.csd(x, y, NFFT=16)),
                       ("cohere", lambda: ax.cohere(x, y, NFFT=16)),
                       ("magnitude_spectrum", lambda: ax.magnitude_spectrum(x)[:2]),
                       ("angle_spectrum", lambda: ax.angle_spectrum(x)[:2]),
                       ("phase_spectrum", lambda: ax.phase_spectrum(x)[:2])]:
        rows.row(f"R {name} ints", "returns", call)
    lags, c, line, base = ax.acorr(x.astype(float), maxlags=4)
    rows.row("R acorr", "lags, c", lambda: (lags, c))
    rows.row("R acorr", "line get_segments()[0]", lambda: line.get_segments()[0])
    rows.row("R acorr usevlines=False", "lags, c",
             lambda: ax.acorr(x.astype(float), maxlags=4, usevlines=False)[:2])
    rows.row("R xcorr", "lags, c", lambda: ax.xcorr(x.astype(float), y.astype(float), maxlags=3)[:2])


SCENES = (histograms, statistics, shapes, fields, spectra)
