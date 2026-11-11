"""
matplotlib's `~.Axes.hexbin`, for the arguments pltrs (Rust) does not
take as they are (see `.bins2d`): its counting with numpy, raising and
warning where and as it raises and warns, over the package's
`.PolyCollection` (matplotlib 3.11's code, nearly as it is).
"""

import math

import numpy as np

from ... import _api, cbook, collections as mcoll, colors as mcolors
from ... import rcParams, transforms as mtransforms


def hexbin(axes, x, y, C, gridsize, bins, xscale, yscale, extent, cmap, norm,
           vmin, vmax, alpha, linewidths, edgecolors, reduce_C_function,
           mincnt, marginals, colorizer, kwargs):
    """`~.Axes.hexbin` of *axes* as matplotlib computes it, its units
    already processed."""
    x, y, C = cbook.delete_masked_points(x, y, C)

    # Set the size of the hexagon grid
    if np.iterable(gridsize):
        nx, ny = gridsize
    else:
        nx = gridsize
        ny = int(nx / math.sqrt(3))
    # Count the number of data in each hexagon
    x = np.asarray(x, float)
    y = np.asarray(y, float)

    # Will be log()'d if necessary, and then rescaled.
    tx = x
    ty = y

    if xscale == 'log':
        if np.any(x <= 0.0):
            raise ValueError(
                "x contains non-positive values, so cannot be log-scaled")
        tx = np.log10(tx)
    if yscale == 'log':
        if np.any(y <= 0.0):
            raise ValueError(
                "y contains non-positive values, so cannot be log-scaled")
        ty = np.log10(ty)
    if extent is not None:
        xmin, xmax, ymin, ymax = extent
        if xmin > xmax:
            raise ValueError("In extent, xmax must be greater than xmin")
        if ymin > ymax:
            raise ValueError("In extent, ymax must be greater than ymin")
    else:
        xmin, xmax = (tx.min(), tx.max()) if len(x) else (0, 1)
        ymin, ymax = (ty.min(), ty.max()) if len(y) else (0, 1)

        # to avoid issues with singular data, expand the min/max pairs
        xmin, xmax = mtransforms._nonsingular(xmin, xmax, expander=0.1)
        ymin, ymax = mtransforms._nonsingular(ymin, ymax, expander=0.1)

    nx1 = nx + 1
    ny1 = ny + 1
    nx2 = nx
    ny2 = ny
    n = nx1 * ny1 + nx2 * ny2

    # In the x-direction, the hexagons exactly cover the region from
    # xmin to xmax. Need some padding to avoid roundoff errors.
    padding = 1.e-9 * (xmax - xmin)
    xmin -= padding
    xmax += padding
    sx = (xmax - xmin) / nx
    sy = (ymax - ymin) / ny
    # Positions in hexagon index coordinates.
    ix = (tx - xmin) / sx
    iy = (ty - ymin) / sy
    ix1 = np.round(ix).astype(int)
    iy1 = np.round(iy).astype(int)
    ix2 = np.floor(ix).astype(int)
    iy2 = np.floor(iy).astype(int)
    # flat indices, plus one so that out-of-range points go to position 0.
    i1 = np.where((0 <= ix1) & (ix1 < nx1) & (0 <= iy1) & (iy1 < ny1),
                  ix1 * ny1 + iy1 + 1, 0)
    i2 = np.where((0 <= ix2) & (ix2 < nx2) & (0 <= iy2) & (iy2 < ny2),
                  ix2 * ny2 + iy2 + 1, 0)

    d1 = (ix - ix1) ** 2 + 3.0 * (iy - iy1) ** 2
    d2 = (ix - ix2 - 0.5) ** 2 + 3.0 * (iy - iy2 - 0.5) ** 2
    bdist = (d1 < d2)

    if C is None:  # [1:] drops out-of-range points.
        counts1 = np.bincount(i1[bdist], minlength=1 + nx1 * ny1)[1:]
        counts2 = np.bincount(i2[~bdist], minlength=1 + nx2 * ny2)[1:]
        accum = np.concatenate([counts1, counts2]).astype(float)
        if mincnt is not None:
            accum[accum < mincnt] = np.nan
        C = np.ones(len(x))
    else:
        # store the C values in a list per hexagon index
        Cs_at_i1 = [[] for _ in range(1 + nx1 * ny1)]
        Cs_at_i2 = [[] for _ in range(1 + nx2 * ny2)]
        for i in range(len(x)):
            if bdist[i]:
                Cs_at_i1[i1[i]].append(C[i])
            else:
                Cs_at_i2[i2[i]].append(C[i])
        if mincnt is None:
            mincnt = 1
        accum = np.array(
            [reduce_C_function(acc) if len(acc) >= mincnt else np.nan
             for Cs_at_i in [Cs_at_i1, Cs_at_i2]
             for acc in Cs_at_i[1:]],  # [1:] drops out-of-range points.
            float)

    good_idxs = ~np.isnan(accum)

    offsets = np.zeros((n, 2), float)
    offsets[:nx1 * ny1, 0] = np.repeat(np.arange(nx1), ny1)
    offsets[:nx1 * ny1, 1] = np.tile(np.arange(ny1), nx1)
    offsets[nx1 * ny1:, 0] = np.repeat(np.arange(nx2) + 0.5, ny2)
    offsets[nx1 * ny1:, 1] = np.tile(np.arange(ny2), nx2) + 0.5
    offsets[:, 0] *= sx
    offsets[:, 1] *= sy
    offsets[:, 0] += xmin
    offsets[:, 1] += ymin
    # remove accumulation bins with no data
    offsets = offsets[good_idxs, :]
    accum = accum[good_idxs]

    polygon = [sx, sy / 3] * np.array(
        [[.5, -.5], [.5, .5], [0., 1.], [-.5, .5], [-.5, -.5], [0., -1.]])

    if linewidths is None:
        linewidths = [rcParams['patch.linewidth']]

    if xscale == 'log' or yscale == 'log':
        polygons = np.expand_dims(polygon, 0)
        if xscale == 'log':
            polygons[:, :, 0] = 10.0 ** polygons[:, :, 0]
            xmin = 10.0 ** xmin
            xmax = 10.0 ** xmax
            axes.set_xscale(xscale)
        if yscale == 'log':
            polygons[:, :, 1] = 10.0 ** polygons[:, :, 1]
            ymin = 10.0 ** ymin
            ymax = 10.0 ** ymax
            axes.set_yscale(yscale)
    else:
        polygons = [polygon]

    collection = mcoll.PolyCollection(
        polygons,
        edgecolors=edgecolors,
        linewidths=linewidths,
        offsets=offsets,
        offset_transform=mtransforms.AffineDeltaTransform(axes.transData)
    )

    # Set normalizer if bins is 'log'
    if cbook._str_equal(bins, 'log'):
        if norm is not None:
            _api.warn_external("Only one of 'bins' and 'norm' arguments "
                               f"can be supplied, ignoring {bins=}")
        else:
            norm = mcolors.LogNorm(vmin=vmin, vmax=vmax)
            vmin = vmax = None
        bins = None

    if bins is not None:
        if not np.iterable(bins):
            minimum, maximum = min(accum), max(accum)
            bins -= 1  # one less edge than bins
            bins = minimum + (maximum - minimum) * np.arange(bins) / bins
        bins = np.sort(bins)
        accum = bins.searchsorted(accum)

    if colorizer:
        collection._set_colorizer_check_keywords(colorizer, cmap=cmap,
                                                 norm=norm, vmin=vmin,
                                                 vmax=vmax)
    else:
        collection.set_cmap(cmap)
        collection.set_norm(norm)
    collection.set_array(accum)
    collection.set_alpha(alpha)
    collection._made_with(kwargs)
    collection._scale_norm(norm, vmin, vmax)

    # autoscale the norm with current accum values if it hasn't been set
    if norm is not None:
        if collection.norm.vmin is None and collection.norm.vmax is None:
            collection.norm.autoscale()

    corners = ((xmin, ymin), (xmax, ymax))
    axes.update_datalim(corners)
    axes._request_autoscale_view(tight=True)

    # add the collection last
    axes.add_collection(collection, autolim=False)
    if not marginals:
        return collection

    # Process marginals
    bars = []
    for zname, z, zmin, zmax, zscale, nbins in [
            ("x", x, xmin, xmax, xscale, nx),
            ("y", y, ymin, ymax, yscale, 2 * ny),
    ]:

        if zscale == "log":
            bin_edges = np.geomspace(zmin, zmax, nbins + 1)
        else:
            bin_edges = np.linspace(zmin, zmax, nbins + 1)

        verts = np.empty((nbins, 4, 2))
        verts[:, 0, 0] = verts[:, 1, 0] = bin_edges[:-1]
        verts[:, 2, 0] = verts[:, 3, 0] = bin_edges[1:]
        verts[:, 0, 1] = verts[:, 3, 1] = .00
        verts[:, 1, 1] = verts[:, 2, 1] = .05
        if zname == "y":
            verts = verts[:, :, ::-1]  # Swap x and y.

        # Sort z-values into bins defined by bin_edges.
        bin_idxs = np.searchsorted(bin_edges, z) - 1
        values = np.empty(nbins)
        for i in range(nbins):
            # Get C-values for each bin, and compute bin value with
            # reduce_C_function.
            ci = C[bin_idxs == i]
            values[i] = reduce_C_function(ci) if len(ci) > 0 else np.nan

        mask = ~np.isnan(values)
        verts = verts[mask]
        values = values[mask]

        trans = getattr(axes, f"get_{zname}axis_transform")(which="grid")
        bar = mcoll.PolyCollection(
            verts, transform=trans, edgecolors="face")
        bar.set_array(values)
        bar.set_cmap(cmap)
        bar.set_norm(norm)
        bar.set_alpha(alpha)
        bar._made_with(kwargs)
        bars.append(axes.add_collection(bar, autolim=False))

    collection.hbar, collection.vbar = bars

    def on_changed(collection):
        collection.hbar.set_cmap(collection.get_cmap())
        collection.hbar.set_cmap(collection.get_cmap())
        collection.vbar.set_clim(collection.get_clim())
        collection.vbar.set_clim(collection.get_clim())

    collection.callbacks.connect('changed', on_changed)

    return collection
