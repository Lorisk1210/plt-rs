"""Writes the generated programs of the differential tester.

    python generate.py OUT_DIR

Runs under matplotlib (the reference venv): the programs are made by
looking at matplotlib's API, so that they call what matplotlib has. Each
program is a list of statements, each one call, run in ``steps`` mode
(``trace.py``): every statement's value, exception and warnings are
compared on their own. The generators are the modules of ``generators/``:

* ``artists``: every getter of every kind of artist before and after a
  draw, then every setter with sample values, each followed by its getter;
* ``plotting``: every plotting method of ``Axes`` with argument variants;
* ``pyplot``: pyplot's functions;
* ``modules``: formatters, locators, scales, norms, colormaps, colours,
  transforms, paths, text, legends, colorbars, layout, 3-D axes and the
  toolkits;
* ``randomized``: sequences of plotting calls, setters and getters drawn
  with a seeded SplitMix64 (``spec_support.rng``'s);
* ``three_d``: the 3-D toolkit's plotting methods, view, interaction,
  ``art3d``, ``proj3d`` and ``axis3d``;
* ``offsetboxes``: offset boxes of the program's over the package's, boxes
  changed once drawn, and what Axes of each patch shape clip to;
* ``api``: every public callable of matplotlib (``api_index``) with each
  of its keyword parameters, at representative and edge values, on the
  subject it belongs to;
* ``unreached``: the core modules' names the ``api`` programs leave
  unreached, each called where it has what it needs;
* ``hostile``: every callable of the ``api`` programs again, each
  parameter given what users get wrong (other types, None, empty data,
  NaN, inf, negative and zero sizes, every string choice and a wrong
  one), unknown keywords, missing and extra arguments, mismatched data
  and the aliases of the artists' properties;
* ``orders``: calls in unusual orders: on artists removed, on closed
  figures and cleared Axes, artists moved across Axes and figures, and
  the same call twice;
* ``rest``: the callables and parameters no other generator reaches
  (DVI fonts, FreeType's buffers, the testing helpers, an animation
  saved), each called where it has what it needs;
* ``rcparams``: every rcParam but the GUI windows' set to other valid
  values, on a scene it changes, drawn, saved and read back; every style
  sheet alone and in pairs; ``rc_context``, the rc files and functions,
  and an invalid value for each validator;
* ``pickling``: figures of every kind of artist, Axes and toolkit
  pickled, unpickled and pickled again, their getters called on both;
  values pickled and copied (``copy.copy``, ``copy.deepcopy``).

Programs are written to ``OUT_DIR/<generator>/<name>.py``; the directory
is emptied first. What the generators' own runs of statements write goes
to ``OUT_DIR/.scratch``.
"""

from __future__ import annotations

import os
import pathlib
import shutil
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import matplotlib  # noqa: E402

matplotlib.use("agg")

from generators import (  # noqa: E402
    api, artists, hostile, modules, offsetboxes, orders, pickling, plotting, pyplot, randomized, rcparams, rest,
    three_d, unreached)

GENERATORS = {
    "artists": artists.programs,
    "plotting": plotting.programs,
    "pyplot": pyplot.programs,
    "modules": modules.programs,
    "randomized": randomized.programs,
    "three_d": three_d.programs,
    "offsetboxes": offsetboxes.programs,
    "api": api.programs,
    "unreached": unreached.programs,
    "hostile": hostile.programs,
    "orders": orders.programs,
    "rest": rest.programs,
    "rcparams": rcparams.programs,
    "pickling": pickling.programs,
}


def main() -> int:
    """Write every generator's programs."""
    out = pathlib.Path(sys.argv[1]).resolve()
    if out.exists():
        shutil.rmtree(out)
    # The generators run some statements to see what they make; what those
    # write goes here.
    scratch = out / ".scratch"
    scratch.mkdir(parents=True)
    os.chdir(scratch)
    for group, generate in GENERATORS.items():
        directory = out / group
        directory.mkdir(parents=True)
        for name, statements in generate():
            (directory / f"{name}.py").write_text("\n".join(statements) + "\n",
                                                  encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
