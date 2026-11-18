"""
Attributes, types and behaviours the differential tester (``cargo xtask
diff``) found pltrs lacking or answering otherwise than matplotlib 3.11:
the attributes matplotlib's objects keep on their instances, the types
of the values they hold, and the errors and warnings they raise. Every
expected value was recorded from matplotlib.
"""

import io
import pickle
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import cbook, patches, scale, ticker, transforms
from pltrs.offsetbox import AnchoredOffsetbox, AnchoredText, TextArea
from build_facts import DEFAULT_INT


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_instance_attributes_of_artists():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2, 3])
    assert line.ind_offset == 0
    assert line.clipbox is line.get_clip_box()
    leg = ax.legend(["a"])
    assert (leg.labelspacing, leg.handletextpad, leg.borderaxespad,
            leg.columnspacing, leg.shadow) == (0.5, 0.8, 0.5, 2.0, False)
    arrow = patches.FancyArrowPatch((0.1, 0.1), (0.8, 0.6))
    assert (arrow.shrinkA, arrow.shrinkB) == (2, 2)
    arrow.shrinkA = 5
    assert arrow.shrinkA == 5
    link = patches.ConnectionPatch((0.1, 0.1), (0.9, 0.9), 'data')
    assert (link.shrinkA, link.shrinkB) == (0.0, 0.0)
    assert ax.stairs([1, 2], orientation='horizontal').orientation == 'horizontal'
    assert fig.subfigures(1, 2)[0].suppressComposite is None


def test_axes_and_axis_attributes():
    fig, ax = plt.subplots()
    assert ax.ignore_existing_data_limits
    ax.axhline(0.5)
    assert not ax.ignore_existing_data_limits
    assert (ax.xaxis.label_position, ax.xaxis.offset_text_position,
            ax.yaxis.offset_text_position) == ('bottom', 'bottom', 'left')
    assert ax.xaxis.isDefault_label
    ax.xaxis.set_label_text('')
    assert not ax.xaxis.isDefault_label
    assert ax.spines is ax.spines
    assert ax.get_subplotspec() is ax.get_subplotspec()
    assert fig.subplotpars is fig.subplotpars
    assert type(fig.subplotpars).__name__ == 'SubplotParams'
    with pytest.raises(TypeError, match=r"_make_getset_interval\.<locals>\.setter"):
        ax.xaxis.set_view_interval(np.array([0.0, 1.0]))


def test_containers_go_stale_when_labelled():
    fig, ax = plt.subplots()
    bars = ax.bar(['a', 'b'], [1, 2])
    assert bars.stale is True
    labelled = ax.bar([1], [1], label='x')
    assert 'stale' not in vars(labelled)


def test_contour_set_attributes():
    fig, ax = plt.subplots()
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    cs = ax.contour(z, levels=3)
    assert (cs.colors, cs.extent, cs.hatches, cs.labelCValues, cs.logscale,
            cs.monochrome, cs.nchunk, cs.origin) == (
                None, None, (None,), [], False, False, 0, None)
    assert cs.get_paths() is cs.get_paths()
    cs.clabel()
    assert len(cs.labelCValues) == len(cs.labelTexts)


def test_figure_images_keep_their_settings():
    fig = plt.figure()
    im = fig.figimage(np.ones((3, 4)))
    assert (im.ox, im.oy, im.magnification, im.origin) == (0, 0, 1.0, 'upper')
    assert (im.get_filternorm(), im.get_filterrad(), im.get_interpolation(),
            im.get_interpolation_stage(), im.get_resample()) == (
                True, 4.0, 'auto', 'auto', False)
    im.set_interpolation('bicubic')
    assert im.get_interpolation() == 'bicubic'


def test_anchored_boxes_have_their_frame_before_they_are_added():
    fig, ax = plt.subplots()
    at = AnchoredText("Figure 1a", prop=dict(size=15), loc='upper left')
    frame = at.patch
    frame.set_boxstyle("round,pad=0.,rounding_size=0.2")
    ax.add_artist(at)
    assert at.patch is frame
    assert type(frame.get_boxstyle()).__name__ == 'Round'
    assert frame.get_mutation_scale() == 15.0
    box = AnchoredOffsetbox(loc='lower left', child=TextArea("x"), frameon=False)
    assert not box.patch.get_visible()


def test_values_of_matplotlibs_types():
    fig, ax = plt.subplots()
    b = ax.barbs([0, 1], [0, 1], [10, -20], [15, 5])
    assert b.get_offsets().dtype == DEFAULT_INT
    assert ax.pcolor(np.ones((2, 3))).get_coordinates().dtype == DEFAULT_INT
    assert ax.axvspan(0.2, 0.4).get_facecolor()[3] == 1
    assert type(ax.axvspan(0.2, 0.4).get_facecolor()[3]) is int
    assert type(pltrs.colormaps['Dark2'].colors) is tuple
    stats = cbook.boxplot_stats(np.array([1, 2, 3, 10]))[0]
    assert type(stats['mean']) is np.float64 and type(stats['whislo']) is DEFAULT_INT
    ax.plot([1, 2])
    ax.margins(0.2, 0.2)
    ax.autoscale(enable=None, axis="x", tight=True)
    assert ax.margins() == (0, 0) and type(ax.margins()[0]) is int


def test_tickers_answer_as_matplotlibs():
    assert ticker.FixedLocator(np.arange(20), nbins=4).tick_values(0, 1).dtype == DEFAULT_INT
    assert ticker.SymmetricalLogLocator(linthresh=1, base=10).tick_values(0, 1) == [0, 1]
    assert ticker.scale_range(0, 100) == (100.0, 0)
    assert type(ticker.scale_range(0, 100)[1]) is int
    loc = ticker.LogLocator()
    fig, ax = plt.subplots()
    loc.set_axis(ax.xaxis)
    assert loc.nonsingular(-np.inf, np.inf) == (1, 10)
    assert type(loc.nonsingular(1, 1)[0]) is np.float64
    with pytest.raises(ValueError, match=r"^Data cannot be log-scaled"):
        loc.tick_values(0, 1)
    with pytest.warns(UserWarning, match="cannot be logit-scaled"):
        ticker.LogitLocator().tick_values(5, 5)
    formatter = ticker.LogitFormatter()
    formatter.set_axis(ax.xaxis)
    formatter.format_ticks([0.1, 0.5])
    assert isinstance(formatter._locs, np.ndarray)


def test_polar_axes_refuse_other_scales():
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.plot(np.linspace(0, 2 * np.pi, 7), [3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])
    with pytest.raises(NotImplementedError, match="xscale cannot be set"):
        ax.set_xscale('log')
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        ax.set_yscale('logit')
    with pytest.raises(NotImplementedError, match="cannot be logit scaled"):
        ax.get_yticks()
    with pytest.raises(NotImplementedError, match="cannot be logit scaled"):
        fig.canvas.draw()


def test_lines_of_mismatched_data_raise_matplotlibs_error():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2], [1, 2])
    line.set_xdata([1, 2, 3])
    with pytest.raises(ValueError, match="shape mismatch: objects cannot be broadcast"):
        fig.canvas.draw()


def test_pyplot_commands_and_package_names():
    with pytest.warns(PendingDeprecationWarning):
        commands = plt.get_plot_commands()
    assert {'polar', 'rc', 'xticks', 'yscale'} <= set(commands)
    assert not {'bxp', 'pcolorfast', 'supxlabel', 'violin'} & set(commands)
    assert pltrs.cycler(color=['r']).keys == {'color'}
    fig, ax = plt.subplots()
    region = fig.canvas.copy_from_bbox(ax.bbox)
    assert type(region).__module__.endswith('backends._backend_agg')


def test_gridline_tick_parameters():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    ax.tick_params(grid_drawstyle='steps', grid_gapcolor='r', grid_snap=True,
                   grid_rasterized=True, grid_clip_on=False, grid_url='u',
                   grid_gid='g', grid_agg_filter=None, grid_animated=True,
                   grid_clip_box=None, grid_clip_path=None)
    ax.grid(True, linestyle='--')
    fig.canvas.draw()
    g = ax.xaxis.get_major_ticks()[1].gridline
    assert (g.get_drawstyle(), g.get_gapcolor(), g.get_snap(), g.get_rasterized(),
            g.get_clip_on(), g.get_url(), g.get_gid(), g.get_animated()) == (
                'steps', 'r', True, True, False, 'u', 'g', True)
    ax.tick_params(grid_rasterized=False)
    buf = io.BytesIO()
    fig.savefig(buf, format='svg')
    assert b'id="g"' in buf.getvalue()


def test_three_d_names():
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    ax.plot([0, 1, 2], [1, 2, 0], [3, 1, 2])
    assert ax.xy_dataLim.bounds == (0.0, 0.0, 2.0, 2.0)
    assert ax.zz_dataLim.intervalx.tolist() == [1.0, 3.0]
    assert ax.xy_viewLim is ax.xy_viewLim
    assert ax.fmt_zdata is None
    assert (ax.zaxis.isDefault_label, ax.zaxis.label_position) == (True, 'bottom')
    assert type(ax.zaxis.pane).__name__ == 'Polygon'
    X, Y = np.meshgrid(np.arange(3), np.arange(3))
    assert type(ax.contour(X, Y, X * Y)).__name__ == 'QuadContourSet3D'
    assert type(ax.contour(X, Y, X * Y, extend3d=True)).__name__ == 'QuadContourSet'


def test_more_values_of_matplotlibs_types():
    fig, ax = plt.subplots()
    q = ax.quiver([0, 1], [0, 1], [1, -1], [1, 2])
    assert q.Umask is np.False_
    assert sorted(q.polykw) == ['facecolors', 'linewidths']
    ax.plot([1, 2, 3])
    assert isinstance(ax.xaxis.get_major_ticks()[0].get_view_interval(), np.ndarray)
    link = patches.ConnectionPatch((0.1, 0.1), (0.9, 0.9), 'data')
    assert link.get_mutation_aspect() == 1 and type(link.get_mutation_aspect()) is int
    assert link.get_mutation_scale() == 10.0
    circle = patches.Circle((0.5, 0.5), 0.2)
    ax.add_patch(circle)
    assert type(patches.Shadow(circle, 0.02, -0.02).get_zorder()) is np.float64
    assert AnchoredText('x', loc='upper left').get_clip_on() is False
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    assert ax.contourf(z, hatches=['//', None]).get_hatch() is None
    assert ax.fill([0, 1, 1], [0, 0, 1])[0].get_label() is None
    fig = plt.figure()
    polar = fig.add_subplot(projection='polar')
    polar.plot([0, 1], [1, 2])
    assert type(polar.get_yticks()) is list


def test_labels_notify_their_callbacks_once():
    fig, ax = plt.subplots()
    calls = []
    line, = ax.plot([1, 2])
    line.add_callback(lambda artist: calls.append(artist))
    line.set_label('a')
    line.set_label('a')
    assert len(calls) == 1 and line.stale


def test_fancy_arrows_are_polygons():
    arrow = patches.FancyArrow(0.1, 0.1, 0.5, 0.3, width=0.05)
    assert arrow.get_closed() and arrow.get_xy().shape == (8, 2)
    arrow.set_closed(False)
    assert not arrow.get_closed()
    assert arrow.xy.shape == (7, 2) and arrow.get_path().codes is None
    with pytest.raises(ValueError, match=r"not enough values to unpack \(expected 2, got 1\)"):
        arrow.set_xy((1, 2))
    arrow.set_xy([[0, 0], [1, 0], [1, 1]])
    assert arrow.get_xy().tolist() == [[0, 0], [1, 0], [1, 1]]
    arrow.set_data(dx=0.2)
    assert arrow.get_xy().shape == (7, 2)
    arrow.set_closed(True)
    assert arrow.get_path().codes.tolist() == [1, 2, 2, 2, 2, 2, 2, 79]
    with pytest.raises(ValueError, match="too many values to unpack"):
        patches.Polygon([[0, 0], [1, 0]]).set_xy(np.zeros((2, 2, 2)))


def test_three_d_markers_of_mismatched_styles_fail_to_draw():
    x = np.linspace(0, 2 * np.pi, 7)

    def scatter():
        fig = plt.figure()
        return fig, fig.add_subplot(projection='3d').scatter(x, np.sin(x), x)

    fig, markers = scatter()
    markers.set_array(np.arange(4.0))
    with pytest.raises(ValueError, match=r"\(4,4\)  and requested shape \(7,4\)"):
        fig.canvas.draw()
    fig, markers = scatter()
    markers.set_sizes([10, 20])
    with pytest.raises(TypeError, match="only integer scalar arrays"):
        fig.canvas.draw()
    fig, markers = scatter()
    markers.set_linewidth([1, 2])
    with pytest.raises(IndexError, match="index 2 is out of bounds for axis 0 with size 2"):
        fig.canvas.draw()
    markers.set_linewidth([1])
    fig.canvas.draw()


def test_norms_refuse_limits_they_cannot_take():
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    fig, ax = plt.subplots()
    filled = ax.contourf(z)
    filled.set_norm(pltrs.colors.LogNorm())
    assert filled.get_clim() == (0.5, 2.0)
    with pytest.raises(ValueError, match="^Invalid vmin or vmax$"):
        filled.set_clim((0, 1))
    fig, ax = plt.subplots()
    points = ax.scatter([1, 2], [1, 2], c=[1, 2])
    points.set_norm(pltrs.colors.LogNorm())
    points.set_clim((0, 1))
    with pytest.raises(ValueError, match="^Invalid vmin or vmax$"):
        fig.canvas.draw()


def test_errors_of_matplotlibs_checks():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^Automated estimation of the number of bins"):
        ax.hist([1, 2, 3], bins='auto', weights=[1, 1, 1])
    with pytest.raises(TypeError, match=r"^Axes\.boxplot\(\) got an unexpected keyword"):
        ax.boxplot([1, 2, 3], labels=['v'])
    assert type(ax).boxplot.__qualname__ == 'Axes.boxplot'
    assert type(ax).set_xlim.__qualname__ == '_AxesBase.set_xlim'
    assert pickle.loads(pickle.dumps(type(ax).plot)) is type(ax).plot
    ax3d = plt.figure().add_subplot(projection='3d')
    with pytest.raises(ValueError, match=r"'aspect' must be 1D with shape \(3,\)"):
        ax3d.set_box_aspect(1)


def test_scales_of_masked_values_draw():
    class Clipped(transforms.Transform):
        input_dims = output_dims = 1

        def transform_non_affine(self, values):
            return np.ma.masked_where(values > 1, values)

        def inverted(self):
            return self

    class ClippedScale(scale.ScaleBase):
        name = 'clipped'

        def get_transform(self):
            return Clipped()

        def set_default_locators_and_formatters(self, axis):
            axis.set_major_locator(ticker.FixedLocator([0, 0.5, 1]))
            axis.set_major_formatter(ticker.ScalarFormatter())

    fig, ax = plt.subplots()
    ax.plot([0, 0.5, 2])
    ax.set_yscale(ClippedScale(ax.yaxis))
    fig.savefig(io.BytesIO(), format='png')


def test_tick_locations_of_one_level():
    fig, ax = plt.subplots()
    ax.plot([0, 1])
    ax.tick_params(which='major', bottom=False, labelbottom=False)
    assert len(ax.get_xticks()) == 8
    fig = plt.figure()
    polar = fig.add_subplot(projection='polar')
    polar.plot([0, 1], [0.2, 0.8])
    polar.set_yscale('logit')
    polar.set_yticks([0.5])
    assert polar.get_yticks() == [0.5]
    assert len(polar.get_ygridlines()) == 1
    with pytest.raises(NotImplementedError, match="cannot be logit scaled"):
        polar.yaxis.get_minorticklocs()


def test_attributes_of_quivers_and_3d_axes():
    fig, ax = plt.subplots()
    arrows = ax.quiver([0, 1], [0, 1], [1, -1], [1, 2])
    assert arrows.transform is ax.transData
    fig = plt.figure()
    ax3d = fig.add_subplot(projection='3d')
    fig.canvas.draw()
    assert np.allclose(ax3d.invM @ ax3d.M, np.eye(4))


def test_arrows_of_annotations_have_a_path():
    fig, ax = plt.subplots()
    with pltrs.rc_context({'lines.linewidth': 2}):
        note = ax.annotate('x', (0.5, 0.5), xytext=(0.2, 0.2),
                           arrowprops=dict(arrowstyle='->'))
    fig.canvas.draw()
    path = note.arrow_patch.get_path()
    assert len(path.vertices) > 2
    assert np.allclose(path.vertices.max(axis=0),
                       note.arrow_patch.get_window_extent().p1, atol=2)
