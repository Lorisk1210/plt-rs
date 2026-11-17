"""
The default look follows the name pltrs is first imported under in a
process: as ``matplotlib`` (the alias, the ``matplotlib`` distribution),
matplotlib's look, as ``PLTRS_LOOK=matplotlib`` gives it; as ``pltrs``,
pltrs' modern look. The variables ``PLTRS_LOOK``, ``PLTRS_FIGURE_LOOK``
and ``PLTRS_CHROME_LOOK`` choose over the name, and a later import under
the other name changes nothing. Each case runs in a process of its own,
with none of the three variables set unless the case sets it.
"""

import json
import os
import pathlib
import subprocess
import sys

import pytest

import pltrs

#: The alias of the tree, and the pltrs under test (the tree's or an
#: installed wheel's).
ALIAS = pathlib.Path(__file__).resolve().parents[1] / "alias"
PACKAGE_DIR = pathlib.Path(pltrs.__file__).resolve().parents[1]

#: The variables that choose a look.
LOOK_VARIABLES = ("PLTRS_LOOK", "PLTRS_FIGURE_LOOK", "PLTRS_CHROME_LOOK")

#: Parameters the two looks set apart, as a program reads them: the
#: values of matplotlib's look (matplotlib 3.11.2's defaults), and of the
#: modern one.
MATPLOTLIB_LOOK = {
    "figure.constrained_layout.use": False,
    "lines.linewidth": 1.5,
    "axes.prop_cycle": "#1f77b4",
    "font.family": ["sans-serif"],
    "axes.spines.top": True,
}
MODERN_LOOK = {
    "figure.constrained_layout.use": True,
    "lines.linewidth": 1.75,
    "axes.spines.top": False,
}

#: A program printing the parameters of `MATPLOTLIB_LOOK` after *imports*
#: (the first colour of the cycle, in hexadecimal, for ``axes.prop_cycle``).
LOOK = """
import json
{imports}
import pltrs
import pltrs.colors
params = dict(pltrs.rcParams)
params["axes.prop_cycle"] = pltrs.colors.to_hex(params["axes.prop_cycle"].by_key()["color"][0])
print(json.dumps({{key: params[key] for key in {keys!r}}}))
"""

#: A program printing every parameter and every default, as text (without
#: addresses: the default backend is a sentinel object).
ALL_PARAMS = """
import json, re
{imports}
import pltrs
address = re.compile(r" at 0x[0-9a-fA-F]+")
print(json.dumps({{
    "rcParams": {{key: address.sub("", repr(value)) for key, value in pltrs.rcParams.items()}},
    "rcParamsDefault": {{key: address.sub("", repr(value))
                        for key, value in pltrs.rcParamsDefault.items()}},
}}))
"""

#: A program printing the pixels of a figure, as hexadecimal.
PIXELS = """
{imports}
import pltrs.pyplot as plt
fig, ax = plt.subplots()
ax.plot([1, 3, 2], label="a")
ax.bar([1, 2, 3], [0.5, 1, 1.5], label="b")
ax.set_title("A title")
ax.legend()
import io
buffer = io.BytesIO()
fig.savefig(buffer, format="rgba")
print(buffer.getvalue().hex())
"""


def _run(code, tmp_path, *, alias=True, **variables):
    """What *code* prints, run with pltrs on the path (its alias too if
    *alias*), none of the look variables set but *variables*."""
    path = [str(ALIAS), str(PACKAGE_DIR)] if alias else [str(PACKAGE_DIR)]
    env = {key: value for key, value in os.environ.items() if key not in LOOK_VARIABLES}
    env.update(variables, PYTHONPATH=os.pathsep.join(path), MPLBACKEND="agg",
               MPLCONFIGDIR=str(tmp_path), PLTRSCONFIGDIR=str(tmp_path))
    return subprocess.run([sys.executable, "-c", code], env=env, cwd=tmp_path, check=True,
                          capture_output=True, text=True).stdout


def _look(imports, tmp_path, **variables):
    keys = list(MATPLOTLIB_LOOK)
    return json.loads(_run(LOOK.format(imports=imports, keys=keys), tmp_path, **variables))


def _is_matplotlibs(look):
    return look == MATPLOTLIB_LOOK


def _is_modern(look):
    return all(look[key] == value for key, value in MODERN_LOOK.items())


def test_imported_as_matplotlib_the_look_is_matplotlibs(tmp_path):
    assert _is_matplotlibs(_look("import matplotlib", tmp_path))
    assert _is_matplotlibs(_look("import matplotlib.pyplot", tmp_path))
    assert _is_matplotlibs(_look("import mpl_toolkits.mplot3d", tmp_path))


def test_imported_as_matplotlib_every_parameter_is_as_the_variable_gives_it(tmp_path):
    by_name = json.loads(_run(ALL_PARAMS.format(imports="import matplotlib"), tmp_path))
    by_variable = json.loads(_run(ALL_PARAMS.format(imports=""), tmp_path, alias=False,
                                  PLTRS_LOOK="matplotlib"))
    assert by_name == by_variable


def test_imported_as_pltrs_the_look_is_modern(tmp_path):
    assert _is_modern(_look("", tmp_path))
    assert _is_modern(_look("", tmp_path, alias=False))


def test_a_later_import_under_the_other_name_changes_nothing(tmp_path):
    assert _is_modern(_look("import pltrs\nimport matplotlib.pyplot", tmp_path))
    assert _is_matplotlibs(_look("import matplotlib\nimport pltrs.pyplot", tmp_path))


def test_the_variables_choose_over_the_name(tmp_path):
    assert _is_modern(_look("import matplotlib", tmp_path, PLTRS_LOOK="modern"))
    assert _is_modern(_look("import matplotlib", tmp_path, PLTRS_FIGURE_LOOK="modern"))
    assert _is_matplotlibs(_look("", tmp_path, PLTRS_LOOK="matplotlib"))
    assert _is_matplotlibs(_look("", tmp_path, PLTRS_FIGURE_LOOK="matplotlib"))
    # The chrome's variable leaves the figures to the name.
    assert _is_matplotlibs(_look("import matplotlib", tmp_path, PLTRS_CHROME_LOOK="modern"))
    assert _is_modern(_look("", tmp_path, PLTRS_CHROME_LOOK="matplotlib"))


def test_a_figure_under_the_alias_is_the_one_the_variable_draws(tmp_path):
    by_name = _run(PIXELS.format(imports="import matplotlib"), tmp_path)
    by_variable = _run(PIXELS.format(imports=""), tmp_path, alias=False,
                       PLTRS_LOOK="matplotlib")
    modern = _run(PIXELS.format(imports=""), tmp_path, alias=False)
    assert by_name == by_variable
    assert by_name != modern


def test_pandas_draws_in_matplotlibs_look_through_the_alias(tmp_path):
    pytest.importorskip("pandas")
    code = """
import json
import pandas as pd
ax = pd.DataFrame({"a": [1, 3, 2], "b": [2, 1, 3]}).plot()
fig = ax.figure
import matplotlib
import matplotlib.colors
print(json.dumps([matplotlib.__name__, ax.get_lines()[0].get_linewidth(),
                  matplotlib.colors.to_hex(ax.get_lines()[0].get_color()),
                  ax.spines["top"].get_visible(),
                  fig.get_layout_engine() is None]))
"""
    assert json.loads(_run(code, tmp_path)) == ["matplotlib", 1.5, "#1f77b4", True, True]


def test_a_callback_disconnected_while_another_is_collected():
    # Found by the release rehearsal: an animation saved with Pillow
    # raised "dictionary changed size during iteration" when a collection
    # during `disconnect`'s search removed a dead method's proxy. matplotlib
    # 3.11.2 iterates the map itself and can fail alike; pltrs searches a
    # copy. The collection is forced inside the search: the cid's
    # comparison drops the only reference to a connected method's object.
    import gc

    from pltrs import cbook

    class Listener:
        def on_event(self):
            pass

    registry = cbook.CallbackRegistry()
    holder = [Listener()]
    registry.connect("event", holder[0].on_event)

    class DroppingCid(int):
        def __eq__(self, other):
            holder.clear()
            gc.collect()
            return int(self) == other

        __hash__ = int.__hash__

    kept = registry.connect("event", print)
    registry.disconnect(DroppingCid(kept))
    assert registry.callbacks.get("event", {}) == {}
