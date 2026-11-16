"""
The date of a document as matplotlib's PDF, PostScript and SVG writers
take it from ``SOURCE_DATE_EPOCH``
(https://reproducible-builds.org/specs/source-date-epoch/), each where it
reads it: ``datetime.fromtimestamp(int(value), timezone.utc)``. A value
that is no integer raises ``ValueError``; one the platform's clock
functions refuse raises what they raise there (Windows' refuse a time
before 1970 with ``OSError``). pltrs (Rust) is then given the date read
here, so that it writes the instant Python read.
"""

import os
from datetime import datetime, timezone


def source_date():
    """The UTC date ``SOURCE_DATE_EPOCH`` names, or None when it is unset
    or empty."""
    epoch = os.getenv("SOURCE_DATE_EPOCH")
    return datetime.fromtimestamp(int(epoch), timezone.utc) if epoch else None
