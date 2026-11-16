"""
The date ``SOURCE_DATE_EPOCH`` gives saved documents, read as matplotlib's
PDF, PostScript and SVG writers read it: ``datetime.fromtimestamp(int(value),
timezone.utc)``, raising what that raises on this system
(`build_facts.source_date_error`: a value that is no integer everywhere,
a time before 1970 on Windows).
"""

import io
import re
import sys

import pytest

import pltrs.pyplot as plt

import build_facts

#: The formats whose writers read the variable.
FORMATS = ["pdf", "ps", "eps", "svg"]

#: Values to read: no integers, before 1970, after it, in Python's
#: integer syntax only, far ahead.
VALUES = ["abc", "1.5", "-86401", "86401", "1_000", "8589934592", str(2 ** 40)]

#: How each format writes the date of 86401 seconds after the epoch.
WRITTEN = {
    "pdf": b"/CreationDate (D:19700102000001Z)",
    "ps": b"%%CreationDate: Fri Jan 02 00:00:01 1970",
    "eps": b"%%CreationDate: Fri Jan 02 00:00:01 1970",
    "svg": b"<dc:date>1970-01-02T00:00:01+00:00</dc:date>",
}

#: The same of ``1_000``, which only Python's integer syntax reads.
WRITTEN_1000 = {
    "pdf": b"/CreationDate (D:19700101001640Z)",
    "ps": b"%%CreationDate: Thu Jan 01 00:16:40 1970",
    "eps": b"%%CreationDate: Thu Jan 01 00:16:40 1970",
    "svg": b"<dc:date>1970-01-01T00:16:40+00:00</dc:date>",
}


def _saved(fmt, **kwargs):
    buffer = io.BytesIO()
    fig = plt.figure()
    try:
        fig.savefig(buffer, format=fmt, **kwargs)
    finally:
        plt.close(fig)
    return buffer.getvalue()


def test_the_facts_are_the_platforms():
    assert build_facts.source_date_error("abc")[0] is ValueError
    assert build_facts.source_date_error("86401") is None
    before_1970 = build_facts.source_date_error("-86401")
    assert (before_1970 is not None and before_1970[0] is OSError) == (sys.platform == "win32")


@pytest.mark.parametrize("value", VALUES)
@pytest.mark.parametrize("fmt", FORMATS)
def test_documents_take_the_date_as_the_platforms_datetime_does(fmt, value, monkeypatch):
    monkeypatch.setenv("SOURCE_DATE_EPOCH", value)
    error = build_facts.source_date_error(value)
    if error is None:
        _saved(fmt)
    else:
        with pytest.raises(error[0], match=re.escape(error[1])):
            _saved(fmt)


@pytest.mark.parametrize("fmt", FORMATS)
def test_documents_write_the_date_python_read(fmt, monkeypatch):
    monkeypatch.setenv("SOURCE_DATE_EPOCH", "86401")
    assert WRITTEN[fmt] in _saved(fmt)
    monkeypatch.setenv("SOURCE_DATE_EPOCH", "1_000")
    assert WRITTEN_1000[fmt] in _saved(fmt)


def test_a_given_date_wins_but_in_postscript(monkeypatch):
    import datetime
    monkeypatch.setenv("SOURCE_DATE_EPOCH", "86401")
    given = datetime.datetime(2001, 2, 3, tzinfo=datetime.timezone.utc)
    assert b"/CreationDate (D:20010203000000Z)" in _saved("pdf", metadata={"CreationDate": given})
    assert WRITTEN["ps"] in _saved("ps", metadata={"CreationDate": given})
    assert b"<dc:date>2001-02-03" in _saved("svg", metadata={"Date": given})
    # The variable is read whatever the PDF metadata gives.
    monkeypatch.setenv("SOURCE_DATE_EPOCH", "abc")
    with pytest.raises(ValueError):
        _saved("pdf", metadata={"CreationDate": given})
