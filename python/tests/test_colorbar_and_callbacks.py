"""
Colorbar internals, Python callbacks given arrays, font fallback logs and
invalid log/logit limits, as matplotlib 3.11 has them: a colorbar's
``dividers`` is a `.LineCollection`, its ``outline`` a spine of its Axes
and its Axes placed by matplotlib's locator; the transforms of function
scales, secondary axes and curvilinear grids receive arrays; the font
manager logs about families it cannot find and weights it substitutes;
an aspect that would make the limits NaN raises matplotlib's errors.
Every expected value was recorded from matplotlib.
"""

import io
import logging

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, spines


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _image_colorbar(**kwargs):
    fig, ax = plt.subplots()
    image = ax.imshow(np.arange(12).reshape(3, 4))
    return fig, fig.colorbar(image, **kwargs)


def test_colorbar_dividers_are_a_line_collection():
    fig, cbar = _image_colorbar(extend='both')
    assert type(cbar.dividers) is collections.LineCollection
    assert cbar.dividers.get_segments() == []
    assert cbar.dividers.axes is cbar.ax
    assert not cbar.dividers.get_clip_on()
    assert list(cbar.dividers.get_linewidth()) == [0.4]
    fig2, edged = _image_colorbar(drawedges=True)
    segments = edged.dividers.get_segments()
    assert len(segments) == 255
    np.testing.assert_allclose(segments[0], [[0, 0.04296875], [1, 0.04296875]])
    edged.ax.set_ylim(2, 4)
    assert len(edged.dividers.get_segments()) == 47


def test_colorbar_outline_is_a_spine_of_its_axes():
    fig, cbar = _image_colorbar(extend='both')
    outline = cbar.outline
    assert type(outline).__name__ == '_ColorbarSpine'
    assert isinstance(outline, spines.Spine)
    assert cbar.ax.spines['outline'] is outline
    assert list(cbar.ax.spines.keys()) == ['left', 'right', 'bottom', 'top',
                                           'outline']
    assert outline.spine_type == 'colorbar' and outline.axis is None
    assert outline.get_zorder() == 2.5
    np.testing.assert_allclose(
        outline.get_path().vertices,
        [[0, 0], [0.5, -0.05], [1, 0], [1, 1], [0.5, 1.05], [0, 1], [0, 0]])
    outline.set_color('red')
    assert outline.get_edgecolor() == (1.0, 0.0, 0.0, 1.0)
    with pytest.raises(ValueError, match="unknown spine spine_type: 'colorbar'"):
        outline.get_spine_transform()
    assert [type(child).__name__ for child in cbar.ax.get_children()] == [
        'LineCollection', 'PathPatch', 'PathPatch', 'QuadMesh', 'Spine',
        'Spine', 'Spine', 'Spine', '_ColorbarSpine', 'XAxis', 'YAxis', 'Text',
        'Text', 'Text', 'Rectangle']
    assert outline not in cbar.ax.patches
    cbar.ax.set_frame_on(False)
    fig.canvas.draw()


def test_colorbar_axes_locator():
    fig, cbar = _image_colorbar(extend='both')
    locator = cbar.ax.get_axes_locator()
    assert type(locator).__name__ == '_ColorbarAxesLocator'
    assert locator._cbar is cbar
    original = cbar.ax.get_position(original=True).bounds
    # Before a draw places it, the Axes is where it was made.
    np.testing.assert_allclose(cbar.ax.get_position().bounds, original)
    np.testing.assert_allclose(original, (0.78375, 0.11, 0.11625, 0.77))
    fig.canvas.draw()
    np.testing.assert_allclose(cbar.ax.get_position().bounds,
                               (0.78375, 0.145, 0.028875, 0.7))
    assert cbar.ax.get_box_aspect() == pytest.approx(18.181818181818183)
    np.testing.assert_allclose(locator(cbar.ax, None).bounds,
                               (0.78375, 0.145, 0.11625, 0.7))
    cbar.ax.set_axes_locator(locator)
    fig.canvas.draw()
    np.testing.assert_allclose(cbar.ax.get_position().bounds,
                               (0.78375, 0.145, 0.028875, 0.7))


def _counting(function, shapes):
    def counted(values):
        shapes.append(np.shape(values))
        return function(values)
    return counted


def test_function_scales_get_arrays():
    shapes = []
    fig, ax = plt.subplots()
    ax.plot(np.arange(400), np.linspace(0.002, 1, 400))
    ax.set_yscale('function', functions=(_counting(np.sqrt, shapes),
                                         _counting(np.square, [])))
    fig.canvas.draw()
    assert all(isinstance(shape, tuple) and len(shape) >= 1 for shape in shapes)
    assert max(int(np.prod(shape)) for shape in shapes) == 400
    # matplotlib calls the transform once per path, tick set or limit.
    assert len(shapes) < 200


def test_secondary_axis_functions_get_arrays():
    shapes = []
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    ax.secondary_xaxis('top', functions=(_counting(lambda x: 2 * x, shapes),
                                         _counting(lambda x: x / 2, shapes)))
    fig.canvas.draw()
    assert shapes and all(len(shape) >= 1 for shape in shapes)


def test_program_transforms_get_point_arrays():
    from pltrs import transforms
    shapes = []

    class Swap(transforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            shapes.append(np.shape(values))
            return np.asarray(values)[:, ::-1]

    fig, ax = plt.subplots()
    ax.plot(np.arange(50), np.arange(50), transform=Swap() + ax.transData)
    fig.canvas.draw()
    assert shapes and all(len(shape) == 2 and shape[1] == 2 for shape in shapes)
    assert max(shape[0] for shape in shapes) == 50


def test_curvilinear_grid_functions_get_arrays():
    from pltrs.mpl_toolkits.axisartist import Axes, GridHelperCurveLinear
    shapes = []

    def tr(x, y):
        shapes.append(np.shape(x))
        return x, y - x

    def inv_tr(x, y):
        return x, y + x

    fig = plt.figure()
    fig.add_subplot(axes_class=Axes,
                    grid_helper=GridHelperCurveLinear((tr, inv_tr)))
    fig.canvas.draw()
    assert max(int(np.prod(shape)) for shape in shapes) >= 100
    assert len(shapes) < 1000


def test_logit_limits_the_aspect_cannot_fit_raise():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    ax.set_adjustable('datalim')
    ax.set_aspect(2)
    ax.set_xlim(5, -5)
    ax.set_xscale('symlog')
    ax.set_ylim(-1, 1)
    ax.set_yscale('logit')
    assert np.isnan(ax.get_data_ratio())
    for call in (ax.apply_aspect, ax.get_position, ax.get_tightbbox,
                 fig.canvas.draw):
        with pytest.raises(ValueError, match='Axis limits cannot be NaN or Inf'):
            call()
    assert ax.get_xlim() == (5.0, -5.0)
    assert ax.get_ylim() == (-1.0, 1.0)


def test_polar_logit_axes_refuse_their_minor_ticks():
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.plot([1, 2, 3])
    ax.set_ylim(0.05, 0.95)
    ax.set_yscale('logit')
    ax.set_yticks([0.5])
    message = 'Polar axis cannot be logit scaled yet'
    for call in (ax.get_yticklabels, ax.get_tightbbox, fig.canvas.draw):
        with pytest.raises(NotImplementedError, match=message):
            call()


def test_font_lookups_log_as_matplotlib(caplog):
    fig, ax = plt.subplots()
    ax.text(0.5, 0.5, 'light', fontweight='light')
    ax.text(0.5, 0.2, 'xkcd', family=['xkcd', 'DejaVu Sans'])
    with caplog.at_level(logging.WARNING):
        fig.savefig(io.BytesIO(), format='png')
    messages = [record.getMessage() for record in caplog.records]
    assert messages.count('findfont: Failed to find font weight light for '
                          'DejaVu Sans, now using 400.') == 1
    assert "findfont: Font family 'xkcd' not found." in messages
