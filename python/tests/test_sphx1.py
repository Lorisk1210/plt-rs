"""
matplotlib's Sphinx extensions under pltrs (``matplotlib.sphinxext``): the
project in ``sphinx_project`` uses every option of the plot directive and
every configuration value of it and of ``mathmpl``, ``figure-mpl`` and the
roles, and is built under matplotlib and under pltrs installed as
matplotlib, in each of its three configurations (``conf.py``): the same
warnings, the same files, the same HTML and the same pictures
(``sphinx_builds``).
"""

import pathlib
import sys

import pytest

import sphinx_builds

PROJECT = pathlib.Path(__file__).with_name("sphinx_project")


def _require_sphinx():
    """Skips unless both libraries' interpreters have Sphinx."""
    if not sphinx_builds.has_sphinx(sys.executable):
        pytest.skip("Sphinx is not installed (python/requirements-suite.txt)")
    if not sphinx_builds.REFERENCE_PYTHON.exists():
        pytest.skip(f"no reference venv at {sphinx_builds.REFERENCE_PYTHON}")
    if not sphinx_builds.has_sphinx(sphinx_builds.REFERENCE_PYTHON):
        pytest.skip("Sphinx is not installed in the reference venv")


#: What each configuration writes under matplotlib, which shows that the
#: project reaches the options it is about: (file, text) pairs.
WRITTEN = {
    "default": [
        ("plots.html", "The code of a bar chart"),  # code-caption
        ("plots.html", 'alt="Points along a parabola"'),
        ("plots.html", "wide-plot"),  # class
        ("plots.html", "The caption of the spiral, given in the content."),
        ("plots.html", "The petals’ caption, from the option."),
        ("plots.html", "hires.png</span></code></a>"),  # plot_html_show_formats
        ("_images/functions_rings.png", ""),
        ("_images/plots-8_01_00.png", ""),  # two figures, split at plt.show()
        ("_downloads/<hash>/bars.py", ""),  # filename-prefix
        ("_images/mathmpl/math-3db0bf8f34.png", ""),
        ("roles.html", "(default: "),
        ("figures.html", 'srcset="_images/picture.png, _images/picture.2x.png 2.00x"'),
        ("nested/index.html", "../_images/nested-picture.2x.png 2.00x"),
    ],
    "srcset": [
        ("plots.html", 'srcset="_images/plots-1.png, _images/plots-1.2x.png 2.00x"'),
        ("_images/mathmpl/math-3db0bf8f34-1_5x.png", ""),
        ("context.html", "close-figs"),  # plot_include_source
    ],
    "custom": [
        ("plots.html", "(custom template) \nA scatter plot with a caption."),
    ],
}


@pytest.mark.parametrize("tag", WRITTEN)
def test_the_project_builds_as_under_matplotlib(tmp_path, tag):
    _require_sphinx()
    args = [] if tag == "default" else ["-t", tag]
    matplotlib = sphinx_builds.build(PROJECT, tmp_path / "matplotlib", pltrs=False, args=args)
    assert matplotlib.returncode == 0, "\n".join(matplotlib.warnings)
    written = sphinx_builds.files(matplotlib.html)
    for name, text in WRITTEN[tag]:
        assert name in written
        assert text in written[name].read_text(encoding="utf-8", errors="replace")
    pltrs = sphinx_builds.build(PROJECT, tmp_path / "pltrs", pltrs=True, args=args)
    assert sphinx_builds.differences(matplotlib, pltrs) == []
