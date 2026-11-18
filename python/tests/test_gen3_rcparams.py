"""
What the differential tester's rcParams and style programs (task GEN3)
found pltrs doing otherwise than matplotlib 3.11: the validation of
rcParams and the rc files, and what the parameters change in figures.
Each expected value is recorded from matplotlib.
"""

import logging
import pathlib

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors
from pltrs.backend_bases import MouseEvent
from build_facts import DEFAULT_INT


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))


# -- assigning and reading rcParams -----------------------------------------------

@pytest.mark.parametrize("key, value, message", [
    ("axes.labelpad", None, "Key axes.labelpad: Could not convert None to float"),
    ("lines.color", "notacolor",
     "Key lines.color: 'notacolor' does not look like a color arg"),
    ("font.weight", "heavyish", "Key font.weight: heavyish is not a valid font weight."),
    ("figure.figsize", [1, 2, 3],
     "Key figure.figsize: Expected 2 values, but there are 3 values in [1, 2, 3]"),
    ("savefig.bbox", "loose", "Key savefig.bbox: bbox should be 'tight' or 'standard'"),
    ("font.style", 1, "Key font.style: Could not convert 1 to str"),
    ("animation.html", "gif", "Key animation.html: 'gif' is not a valid value for "
     "animation.html; supported values are ['html5', 'jshtml', 'none']"),
])
def test_invalid_values_fail_with_matplotlibs_messages(key, value, message):
    with pytest.raises(ValueError) as caught:
        plt.rcParams[key] = value
    assert str(caught.value) == message


def test_some_validators_fail_with_other_errors():
    with pytest.raises(RuntimeError, match=r"^Value must be >-0\.5; got -0\.6$"):
        plt.rcParams['axes.xmargin'] = -0.6


@pytest.mark.parametrize("key, value, expected", [
    ("boxplot.bootstrap", 1.5, 1),
    ("lines.linestyle", (0, (3, 1, 1, 1)), (0, (3, 1, 1, 1))),
    ("lines.dash_capstyle", "round", "round"),
    ("hist.bins", [0, 2, 4], [0.0, 2.0, 4.0]),
])
def test_values_are_kept_as_matplotlibs_validators_give_them(key, value, expected):
    plt.rcParams[key] = value
    assert plt.rcParams[key] == expected


def test_rc_params_cannot_be_deleted():
    params = pltrs.RcParams({'lines.linewidth': 2})
    for remove in (lambda: params.__delitem__('lines.linewidth'),
                   lambda: params.pop('lines.linewidth'), params.popitem):
        with pytest.raises(KeyError) as caught:
            remove()
        assert str(caught.value) == ""
    assert dict(params) == {'lines.linewidth': 2.0}


def test_the_style_core_module_comes_with_rcdefaults():
    import subprocess
    import sys
    done = subprocess.run([sys.executable, "-c", (
        "import pltrs, pltrs.pyplot as plt\n"
        "print(hasattr(plt.style, 'core'))\n"
        "pltrs.rcdefaults()\n"
        "print(hasattr(plt.style, 'core'))")], capture_output=True, text=True, check=True)
    assert done.stdout.split() == ["False", "True"]


def test_rc_files_log_matplotlibs_messages(tmp_path, monkeypatch, caplog):
    monkeypatch.chdir(tmp_path)
    pathlib.Path('custom.rc').write_text(
        "lines.linewidth : 5\nlines.color: notacolor\nbad line\nlines.linewidth: 6\n"
        "no.such.key: 1\nbackend: pdf\n")
    with caplog.at_level(logging.WARNING):
        params = pltrs.rc_params_from_file('custom.rc', use_default_template=False)
    assert [record.getMessage() for record in caplog.records] == [
        "Missing colon in file 'custom.rc', line 3 ('bad line')",
        "Duplicate key in file 'custom.rc', line 4 ('lines.linewidth: 6')",
        "Bad value in file 'custom.rc', line 2 ('lines.color: notacolor'): "
        "Key lines.color: 'notacolor' does not look like a color arg",
        "\nBad key no.such.key in file custom.rc, line 5 ('no.such.key: 1')\n"
        "You probably need to get an updated matplotlibrc file from\n"
        "https://github.com/matplotlib/matplotlib/blob/v3.11.2/lib/matplotlib/"
        "mpl-data/matplotlibrc\nor from the matplotlib source distribution",
    ]
    assert (params['lines.linewidth'], params['backend']) == (6.0, 'pdf')
    with pytest.raises(ValueError, match="^Key lines.color: 'notacolor' does not look"):
        pltrs.rc_params_from_file('custom.rc', fail_on_error=True)


# -- what the parameters change -----------------------------------------------------

def test_axis_labels_are_placed_in_pixels_once_drawn():
    plt.rcParams['xaxis.labellocation'] = 'left'
    fig, ax = plt.subplots(figsize=(4, 3))
    ax.plot([0, 1], [0, 1])
    ax.set_xlabel('x label')
    ax.set_ylabel('y label')
    assert ax.xaxis.label.get_position() == (0, 0)
    fig.canvas.draw()
    x, y = ax.xaxis.label.get_position()
    assert (x, type(x), type(y)) == (0, int, np.float64)
    assert y == pytest.approx(3.833333333333327)
    assert ax.yaxis.label.get_position()[0] == pytest.approx(12.722222222222223)


@pytest.mark.parametrize("labelcolor, expected", [
    ('None', ['black', 'black']),
    ('linecolor', [(0.12156862745098039, 0.4666666666666667, 0.7058823529411765),
                   (0.12156862745098039, 0.4666666666666667, 0.7058823529411765, 1.0)]),
])
def test_legend_labels_take_their_colour_as_matplotlib_gives_it(labelcolor, expected):
    plt.rcParams['legend.labelcolor'] = labelcolor
    fig, ax = plt.subplots()
    ax.plot([1, 2], label='line')
    ax.bar([1, 2], [0.5, 0.3], label='bars')
    colours = [text.get_color() for text in ax.legend().get_texts()]
    assert [tuple(c) if isinstance(c, tuple) else c for c in colours] == expected
    assert type(colours[0]) is type(expected[0])


def test_the_legend_frame_alpha_follows_the_shadow_given():
    plt.rcParams['legend.shadow'] = True
    fig, ax = plt.subplots()
    ax.plot([1, 2], label='line')
    assert ax.legend().get_frame().get_facecolor()[3] == 0.8
    assert ax.legend(shadow=True).get_frame().get_facecolor()[3] == 1


def test_patch_flags_and_colours_as_the_parameters_make_them():
    plt.rcParams.update({'patch.antialiased': False, 'path.snap': False,
                         'patch.edgecolor': 'g', 'patch.force_edgecolor': True,
                         'axes.edgecolor': 'b', 'figure.edgecolor': 'r'})
    fig, ax = plt.subplots()
    rect = ax.add_patch(plt.Rectangle((0, 0), 1, 1))
    bar, = ax.bar([1], [1])
    assert (rect.get_antialiased(), rect.get_snap()) == (False, False)
    for color in (rect.get_edgecolor(), bar.get_edgecolor()):
        assert color == (0.0, 0.5, 0.0, 1) and type(color[3]) is int
    assert type(ax.spines['left'].get_edgecolor()[3]) is int
    assert type(fig.get_edgecolor()[3]) is int


def test_minor_gridlines_of_the_other_axis_keep_their_level():
    plt.rcParams.update({'axes.grid': True, 'axes.grid.axis': 'x',
                         'axes.grid.which': 'minor'})
    fig, ax = plt.subplots()
    ax.minorticks_on()
    fig.canvas.draw()
    assert [tick.gridline.get_visible() for axis in (ax.xaxis, ax.yaxis)
            for tick in axis.get_major_ticks()[:1] + axis.get_minor_ticks()[:1]] == [
        False, True, False, True]


def test_tick_labels_report_the_alignment_of_the_parameters():
    plt.rcParams.update({'xtick.alignment': 'left', 'ytick.alignment': 'top'})
    fig, ax = plt.subplots()
    assert ax.xaxis.get_major_ticks()[0].label1.get_horizontalalignment() == 'left'
    assert ax.yaxis.get_major_ticks()[0].label2.get_verticalalignment() == 'top'


def test_mouse_events_rotate_3d_axes():
    plt.rcParams['axes3d.mouserotationstyle'] = 'azel'
    fig = plt.figure(figsize=(4, 3))
    ax = fig.add_subplot(projection='3d')
    fig.canvas.draw()
    cx, cy = ax.bbox.x0 + ax.bbox.width / 2, ax.bbox.y0 + ax.bbox.height / 2
    assert fig.canvas.inaxes((cx, cy)) is ax
    for name, x, y in [('button_press_event', cx, cy),
                       ('motion_notify_event', cx + 30, cy + 20),
                       ('button_release_event', cx + 30, cy + 20)]:
        fig.canvas.callbacks.process(name, MouseEvent(name, fig.canvas, x, y, button=1))
    assert (ax.elev, ax.azim, ax.roll) == pytest.approx(
        (14.415584415584407, -83.37662337662339, 0))


def test_lookup_tables_are_sampled_once():
    plt.rcParams['image.lut'] = 4
    fig, ax = plt.subplots()
    mesh = ax.pcolormesh(Z, cmap='RdBu_r')
    fig.canvas.draw()
    assert len(np.unique(mesh.get_facecolors(), axis=0)) > 4


def test_concise_dates_tick_at_interval_multiples():
    plt.rcParams.update({'date.converter': 'concise', 'date.interval_multiples': False})
    fig, ax = plt.subplots()
    start = np.datetime64('2021-03-04T05:06:07')
    ax.plot([start, start + np.timedelta64(20, 'D')], [0, 1])
    fig.canvas.draw()
    assert [t.get_text() for t in ax.get_xticklabels()] == [
        '05', '09', '13', '17', '21', '25']


def test_bootstrapped_notches_draw_from_numpys_random_state():
    plt.rcParams['boxplot.bootstrap'] = 100
    np.random.seed(5)
    stats = pltrs.cbook.boxplot_stats([np.arange(10.0)], bootstrap=100)
    assert (stats[0]['cilo'], stats[0]['cihi']) == pytest.approx((1.475, 6.5))


@pytest.mark.parametrize("args, error, message", [
    ((Z,), ValueError, "For X (6) and Y (5) with flat shading, A should have shape "
     "(4, 5, 3) or (4, 5, 4) or (4, 5) or (20,), not (5, 6)"),
    ((np.arange(7), np.arange(6), Z), TypeError,
     "Dimensions of C (5, 6) are incompatible with X (7) and/or Y (6); see help(pcolor)"),
])
def test_gouraud_polygons_fail_as_matplotlibs(args, error, message):
    plt.rcParams['pcolor.shading'] = 'gouraud'
    fig, ax = plt.subplots()
    with pytest.raises(error) as caught:
        ax.pcolor(*args)
    assert str(caught.value) == message


def test_svg_images_go_beside_the_file(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    plt.rcParams.update({'svg.image_inline': False, 'svg.hashsalt': 'salt'})
    fig, ax = plt.subplots()
    ax.imshow(Z)
    fig.savefig('figure.svg')
    assert sorted(p.name for p in tmp_path.iterdir()) == [
        'figure.svg', 'figure.svg.image0.png']
    svg = (tmp_path / 'figure.svg').read_text()
    assert 'xlink:href="figure.svg.image0.png"' in svg
    assert 'id="Im_imagef70b21507e"' in svg


def test_meshes_made_of_given_coordinates_keep_them_masked():
    fig, ax = plt.subplots()
    assert type(ax.pcolormesh(Z).get_coordinates()) is np.ndarray
    given = ax.pcolormesh(np.arange(7), np.arange(6), Z).get_coordinates()
    assert type(given) is np.ma.MaskedArray and given.dtype == DEFAULT_INT
    colorbar = fig.colorbar(ax.imshow(Z))
    assert type(colorbar.solids.get_coordinates()) is np.ma.MaskedArray


def test_limit_callbacks_run_as_each_limit_changes():
    calls = []
    fig, ax = plt.subplots()
    ax.plot([0, 1, 2], [0, 1, 0], picker=5)
    ax.callbacks.connect('xlim_changed', lambda a: calls.append(('x', a.get_xlim())))
    ax.callbacks.connect('ylim_changed', lambda a: calls.append('y'))
    ax.set_xlim(0, 5)
    assert [c if c == 'y' else c[0] for c in calls] == ['x', 'y', 'x']
    assert type(ax.lines[0].get_picker()) is int


def test_3d_panes_keep_the_colour_of_their_parameter():
    plt.rcParams['axes3d.xaxis.panecolor'] = 'r'
    ax = plt.figure().add_subplot(projection='3d')
    color = ax.xaxis.pane.get_facecolor()
    assert color == (1.0, 0.0, 0.0, 1) and type(color[3]) is int


def test_a_colour_alpha_of_matplotlibs_type():
    assert type(colors.to_rgba('b')[3]) is int
