"""
What pltrs (Python) does on Windows as matplotlib does there (task WIN6),
measured on the Windows PC.

A new configuration directory gets the package's data entries copied into
place by every process that finds them missing (`pltrs._copy_into_place`).
Windows now and then refuses such a rename as if the target existed (error
183) while nothing is there yet, when several processes rename into the
directory at once: the first use of a new configuration directory by
``multiprocessing``'s workers crashed in about one round of eight processes
in fifteen. `test_first_use.py` runs the race itself.
"""

import os
import sys

import pytest

import pltrs


def test_a_rename_into_place_refused_with_nothing_there_is_tried_again_on_windows(
        tmp_path, monkeypatch):
    entry = tmp_path / "entry.txt"
    entry.write_text("rc", encoding="utf-8")
    target = tmp_path / "mpl-data" / "matplotlibrc"
    target.parent.mkdir()
    refused = []
    rename = os.rename

    def refusing_twice(source, destination):
        if len(refused) < 2:
            refused.append(destination)
            raise FileExistsError(17, "Cannot create a file when that file already exists",
                                  str(source), None, 183)
        rename(source, destination)

    monkeypatch.setattr(os, "rename", refusing_twice)
    if sys.platform == "win32":
        pltrs._copy_into_place(entry, target)
        assert target.read_text(encoding="utf-8") == "rc"
    else:
        with pytest.raises(FileExistsError):
            pltrs._copy_into_place(entry, target)
    assert len(refused) == 2 or sys.platform != "win32"
    assert not list(target.parent.glob(".*partial"))


def test_a_rename_refused_for_a_copy_another_process_placed_keeps_that_copy(
        tmp_path, monkeypatch):
    entry = tmp_path / "entry.txt"
    entry.write_text("ours", encoding="utf-8")
    target = tmp_path / "mpl-data" / "matplotlibrc"
    target.parent.mkdir()

    def placed_meanwhile(source, destination):
        target.write_text("theirs", encoding="utf-8")
        raise FileExistsError(17, "File exists", str(source), None, 183)

    monkeypatch.setattr(os, "rename", placed_meanwhile)
    pltrs._copy_into_place(entry, target)
    assert target.read_text(encoding="utf-8") == "theirs"
    assert not list(target.parent.glob(".*partial"))
