"""
The configuration of the Sphinx project `test_sphx1.py` builds under
matplotlib and under pltrs: matplotlib's four Sphinx extensions, each
configuration value of the plot directive and of ``mathmpl``.

Values that are one per build take their other side in builds tagged
(``-t TAG``): ``srcset`` writes the plots' srcsets (through ``figure-mpl``)
and the math's, and includes the source by default; ``custom`` gives the
plots a template of its own, no links to their formats, rcParams applied
in a context too, and runs the scripts in their own directory.
"""

from pathlib import Path

import matplotlib
from matplotlib import pyplot as plt

HERE = Path(__file__).parent

extensions = [
    "matplotlib.sphinxext.plot_directive",
    "matplotlib.sphinxext.mathmpl",
    "matplotlib.sphinxext.figmpl_directive",
    "matplotlib.sphinxext.roles",
]
project = "sphx1"
copyright = "2026, the pltrs authors"
master_doc = "index"
exclude_patterns = ["_build", "scripts", "workdir"]
html_theme = "classic"

# The pictures of the `figure-mpl` page, drawn by the library under test.
matplotlib.use("agg")
pictures = HERE / "_generated"
pictures.mkdir(exist_ok=True)
with matplotlib.rc_context({"figure.figsize": (2, 1.5)}):
    figure, axes = plt.subplots()
    axes.plot([0, 1, 2], [0, 1, 0], marker="o")
    axes.set_title("figure-mpl")
    figure.savefig(pictures / "picture.png", dpi=50)
    figure.savefig(pictures / "picture.2x.png", dpi=100)
    plt.close(figure)

plot_basedir = "scripts"
plot_formats = [("png", 60), ("hires.png", 120), "pdf", ("svg", 72)]
plot_rcparams = {"figure.figsize": (4, 3), "lines.linewidth": 2.5,
                 "axes.grid": True}
plot_pre_code = (
    "import numpy as np\n"
    "from matplotlib import pyplot as plt\n"
    "WAVE = np.sin(np.linspace(0, 2 * np.pi, 50))\n"
)
plot_working_directory = str(HERE / "workdir")
plot_html_show_formats = True
plot_include_source = False
plot_html_show_source_link = True
mathmpl_fontsize = 11.0

if tags.has("srcset"):  # noqa: F821 (Sphinx gives conf.py its tags)
    plot_srcset = ["2x"]
    plot_include_source = True
    mathmpl_srcset = ["1.5x"]

if tags.has("custom"):  # noqa: F821
    plot_template = """
{{ source_code }}

.. only:: html

   {% for img in images %}
   .. figure:: {{ build_dir }}/{{ img.basename }}.{{ default_fmt }}
      {% for option in options -%}
      {{ option }}
      {% endfor %}

      (custom template) {{ caption }}
   {% endfor %}
"""
    plot_html_show_formats = False
    plot_apply_rcparams = True
    plot_working_directory = None
