"""
Download what ``cargo xtask ecosystem suite`` runs the libraries' test
suites from, checking every download's SHA-256.

``cargo xtask ecosystem setup`` runs it as

* ``fetch.py sdist NAME VERSION DEST``: looks the release up in PyPI's
  JSON API, downloads its sdist, checks the SHA-256 PyPI gives for it and
  unpacks it into the directory ``DEST``, with the archive filter that
  refuses links and paths leading out of the directory. The archive is
  unpacked beside, then its one top directory (``NAME-VERSION``, or the
  normalised ``scikit_learn-1.9.1``) is renamed into place, so that
  ``DEST`` exists only complete;
* ``fetch.py archive URL SHA256 DEST``: downloads the source archive of a
  release's tag (the tests of the libraries whose sdists ship none), checks
  its SHA-256 and unpacks it into ``DEST`` as an sdist is;
* ``fetch.py file URL SHA256 DEST``: downloads one file (a test's data
  that the sdist leaves out) to ``DEST``, if its SHA-256 is *SHA256*.

It uses the standard library alone.
"""

import hashlib
import io
import json
import pathlib
import shutil
import sys
import tarfile
import urllib.request

#: PyPI's JSON description of a release.
RELEASE_URL = "https://pypi.org/pypi/{name}/{version}/json"


def download(url, sha256):
    """The bytes at *url*, if their SHA-256 is *sha256*."""
    with urllib.request.urlopen(url) as answer:
        data = answer.read()
    digest = hashlib.sha256(data).hexdigest()
    if digest != sha256:
        raise SystemExit(f"{url}: SHA-256 {digest}, expected {sha256}")
    return data


def sdist_of(name, version):
    """The URL and SHA-256 of the sdist of *name* *version* on PyPI."""
    with urllib.request.urlopen(RELEASE_URL.format(name=name, version=version)) as answer:
        release = json.load(answer)
    for file in release["urls"]:
        if file["packagetype"] == "sdist":
            return file["url"], file["digests"]["sha256"]
    raise SystemExit(f"{name} {version} has no sdist on PyPI")


def sdist(name, version, dest):
    """Downloads, checks and unpacks the sdist into *dest*."""
    archive(*sdist_of(name, version), dest)


def archive(url, sha256, dest):
    """Downloads, checks and unpacks the archive at *url* into *dest*."""
    data = download(url, sha256)
    partial = dest.with_name(f".{dest.name}.partial")
    shutil.rmtree(partial, ignore_errors=True)
    partial.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
        tar.extractall(partial, filter="data")
    match list(partial.iterdir()):
        case [top] if top.is_dir():
            top.rename(dest)
        case entries:
            raise SystemExit(f"{url}: {len(entries)} entries at the top, expected one directory")
    shutil.rmtree(partial)
    print(f"unpacked {url} into {dest}")


def file(url, sha256, dest):
    """Downloads one checked file to *dest*."""
    data = download(url, sha256)
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(data)
    print(f"downloaded {url} to {dest}")


if __name__ == "__main__":
    match sys.argv[1:]:
        case ["sdist", name, version, dest]:
            sdist(name, version, pathlib.Path(dest))
        case ["archive", url, sha256, dest]:
            archive(url, sha256, pathlib.Path(dest))
        case ["file", url, sha256, dest]:
            file(url, sha256, pathlib.Path(dest))
        case _:
            raise SystemExit(
                "usage: fetch.py sdist NAME VERSION DEST | archive URL SHA256 DEST"
                " | file URL SHA256 DEST"
            )
