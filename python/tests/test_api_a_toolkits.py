"""
Smoke tests of what pltrs (Python) does as matplotlib does in the
toolkits and the artists next to them: axisartist's parts as the
`.Line2D` and `.Text` subclasses they are, `TriMesh`, the settings of
every image kind, the geographic axes, drawing with another figure's
renderer, axes_grid1's locators, aux-transform boxes and parasites, and
mplot3d's messages. The expected values were recorded from matplotlib
3.11.2 running the same code.
"""

import io
import pickle

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import collections as mcoll, image as mimage, lines as mlines
from pltrs import offsetbox, text as mtext, transforms as mtransforms, tri as mtri
from pltrs.mpl_toolkits.axes_grid1 import inset_locator, parasite_axes
from pltrs.mpl_toolkits.axisartist import Axes as AxisArtistAxes

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

RESET = ("Can not reset the Axes. You are probably trying to reuse an artist "
         "in more than one Axes which is not supported")


def test_axis_artist_parts_are_lines_and_texts():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=AxisArtistAxes)
    axis = ax.axis["bottom"]
    assert isinstance(axis.major_ticks, mlines.Line2D)
    assert isinstance(axis.major_ticklabels, mtext.Text)
    assert isinstance(axis.label, mtext.Text)
    axis.label.set_text("xl")
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    assert (axis.major_ticks.get_ticksize(), axis.minor_ticks.get_ticksize()) == (3.5, 2.0)
    assert np.round(axis.label.get_window_extent(renderer).bounds, 3).tolist() == [
        322.0, 11.797, 12.0, 14.337]
    extents = axis.major_ticklabels.get_window_extents(renderer)
    assert len(extents) == 6
    assert np.round(extents[0].bounds, 3).tolist() == [69.0, 29.741, 22.0, 13.889]
    sizes = axis.major_ticklabels.get_texts_widths_heights_descents(renderer)
    assert np.round(sizes, 3).tolist()[:2] == [[22.0, 10.0, 0.0], [22.0, 10.0, 0.0]]
    axis.set_axis_direction("top")
    assert (axis.label.get_rotation(), axis.major_ticklabels.get_va(),
            axis.major_ticklabels.get_ha()) == (180.0, "baseline", "center")


def test_axis_artists_pickle_with_their_axes():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=AxisArtistAxes)
    ax.axis["y=0"] = ax.new_floating_axis(nth_coord=0, value=0)
    restored = pickle.loads(pickle.dumps(fig))
    axis = restored.axes[0].axis
    assert sorted(axis.keys()) == ["bottom", "left", "right", "top", "y=0"]
    assert type(axis["y=0"]).__name__ == "AxisArtist"


def test_trimesh_of_a_triangulation():
    triangulation = mtri.Triangulation([0, 1, 0, 1], [0, 0, 1, 1],
                                       [[3, 2, 0], [0, 1, 3]])
    mesh = mcoll.TriMesh(triangulation)
    assert len(mesh.get_paths()) == 2
    assert mesh.get_paths()[0].vertices.tolist() == [[1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]
    assert mesh._bbox.bounds == (0.0, 0.0, 1.0, 1.0)
    fig, ax = plt.subplots()
    mesh = ax.add_collection(mcoll.TriMesh(triangulation))
    mesh.set_array(np.array([0., 1., 2., 3.]))
    fig.canvas.draw()
    mesh.set_array(None)
    mesh.set_facecolor("r")
    with pytest.raises(IndexError, match="index 3 is out of bounds for axis 0 with size 1"):
        fig.canvas.draw()


def test_every_image_kind_keeps_the_image_settings():
    fig, ax = plt.subplots()
    pcolor = mimage.PcolorImage(ax, [0, 1, 2], [0, 1], np.ones((1, 2)))
    bbox = mimage.BboxImage(mtransforms.Bbox.unit())
    for image in (pcolor, bbox):
        image.set_interpolation("bicubic")
        image.set_resample(True)
        assert (image.get_interpolation(), image.get_resample(),
                image.get_filternorm(), image.get_filterrad()) == ("bicubic", True, True, 4.0)
        with pytest.raises(ValueError, match="The filter radius must be a positive number"):
            image.set_filterrad(0)
    nonuniform = mimage.NonUniformImage(ax)
    nonuniform.set_interpolation("bilinear")
    assert nonuniform.get_interpolation() == "bilinear"
    with pytest.raises(NotImplementedError, match="Only nearest neighbor and bilinear"):
        nonuniform.set_interpolation("bicubic")
    with pytest.raises(AttributeError, match="_filterrad"):
        nonuniform.get_filterrad()


def test_geographic_axes_refuse_limits_and_scales_at_the_call():
    ax = plt.figure().add_subplot(projection="aitoff")
    message = ("Changing axes limits of a geographic projection is not "
               "supported.  Please consider using Cartopy.")
    for set_lim in (ax.set_xlim, ax.set_ylim):
        with pytest.raises(TypeError) as refused:
            set_lim(0, 1)
        assert str(refused.value) == message
    for set_scale in (ax.set_xscale, ax.set_yscale):
        with pytest.raises(NotImplementedError):
            set_scale("log")


def test_an_artist_draws_with_another_figures_renderer():
    _, ax = plt.subplots()
    line, = ax.plot([0, 1], [0, 1])
    other = plt.figure(dpi=50)
    other.canvas.draw()
    assert line.draw(other.canvas.get_renderer()) is None


def test_inset_locators_take_the_axes_they_are_called_with():
    fig, ax = plt.subplots()
    inset = inset_locator.inset_axes(ax, width="30%", height=1)
    locator = inset.get_axes_locator()
    bbox = locator(inset, fig.canvas.get_renderer())
    assert type(bbox).__name__ == "TransformedBbox"
    assert np.round(bbox.bounds, 6).tolist() == [0.656649, 0.657199, 0.2325, 0.208333]
    fig.canvas.draw()
    assert locator.axes is inset
    with pytest.raises(ValueError) as refused:
        locator(ax, None)
    assert str(refused.value) == RESET

    fig, ax = plt.subplots()
    locator = inset_locator.inset_axes(ax, width="30%", height=1).get_axes_locator()
    locator(ax, None)
    with pytest.raises(ValueError) as refused:
        fig.canvas.draw()
    assert str(refused.value) == RESET


def test_an_empty_aux_transform_box_fails_where_it_is_drawn():
    fig, ax = plt.subplots()
    box = offsetbox.AnchoredOffsetbox(
        "center", child=offsetbox.AuxTransformBox(ax.transData))
    ax.add_artist(box)
    with pytest.raises(ValueError, match="'bboxes' cannot be empty"):
        fig.canvas.draw()
    box.set_animated(True)
    fig.canvas.draw()
    with pytest.raises(ValueError, match="'bboxes' cannot be empty"):
        fig.savefig(io.BytesIO(), format="png")


def test_a_parasite_of_no_axes_class_is_refused_by_object():
    _, ax = plt.subplots()
    with pytest.raises(TypeError) as refused:
        parasite_axes.ParasiteAxesBase(ax)
    assert str(refused.value) == (
        "object.__init__() takes exactly one argument (the instance to initialize)")


def test_mplot3d_messages():
    ax = plt.figure().add_subplot(projection="3d")
    with pytest.raises(ValueError) as refused:
        ax.plot_trisurf([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4], color="notacolor")
    assert str(refused.value) == "Invalid RGBA argument: 'notacolor'"
    with pytest.raises(ValueError) as refused:
        ax.bar3d([], [], [], 1, 1, 1)
    assert str(refused.value) == (
        "zero-size array to reduction operation minimum which has no identity")
