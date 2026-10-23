"""The creation dates of vector documents, after matplotlib's own
``tests/test_determinism.py``.

Area ``mpl_determinism``. Upstream draws a figure in three processes and
compares the bytes, and checks that ``SOURCE_DATE_EPOCH`` sets the
creation date. The byte comparison is a plt-rs integration test
(``crates/plt/tests/determinism``), which also checks that plt-rs reads
``SOURCE_DATE_EPOCH``; the probes compare the dates written for an epoch
with matplotlib's.

Ops
---
``savefig.creation_date``  {format, epoch} -> the creation date an empty
    ``Figure()`` saved as ``format`` records with ``SOURCE_DATE_EPOCH =
    epoch``: the PDF ``/CreationDate`` string, the PostScript
    ``%%CreationDate`` comment or the SVG ``dc:date`` element.
"""

import io
import os
import re

from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("mpl_determinism")

DATES = {
    "pdf": rb"/CreationDate \((D:[^)]*)\)",
    "ps": rb"%%CreationDate: ([^\n]*)\n",
    "svg": rb"<dc:date>([^<]*)</dc:date>",
}


def _creation_date(format, epoch):
    saved = os.environ.get("SOURCE_DATE_EPOCH")
    os.environ["SOURCE_DATE_EPOCH"] = str(epoch)
    try:
        fig = Figure()
        FigureCanvasAgg(fig)
        buffer = io.BytesIO()
        fig.savefig(buffer, format=format)
    finally:
        if saved is None:
            del os.environ["SOURCE_DATE_EPOCH"]
        else:
            os.environ["SOURCE_DATE_EPOCH"] = saved
    return re.search(DATES[format], buffer.getvalue()).group(1).decode("ascii")


def test_determinism_source_date_epoch():
    for epoch in (946684800, 0, 1234567890, 951782400, 4102444799, -86401):
        for format in ("pdf", "ps", "svg"):
            probes.probe("savefig.creation_date", _creation_date, format=format, epoch=epoch)
