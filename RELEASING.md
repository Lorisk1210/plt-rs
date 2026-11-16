# Releasing pltrs (Python)

A release publishes the distribution `pltrs` to PyPI: its sdist and one
wheel per platform, all tagged `cp312-abi3`, so one wheel serves CPython
3.12 and every later version:

| wheel | runs on |
|-------|---------|
| `manylinux_2_17_x86_64.manylinux2014_x86_64`, `manylinux_2_17_aarch64.manylinux2014_aarch64` | Linux with glibc 2.17 or later (CentOS 7, Ubuntu 14.04, Debian 8 and newer) |
| `musllinux_1_2_x86_64`, `musllinux_1_2_aarch64` | Linux with musl 1.2 or later (Alpine 3.13 and newer) |
| `macosx_10_12_universal2` | macOS 10.12 or later on Intel, macOS 11 or later on Apple silicon |
| `win_amd64` (`win_arm64` when asked for) | 64-bit Windows 10 and later |

The second distribution of the tree, `matplotlib` (pltrs as matplotlib,
`pip install . -C distribution=matplotlib`), is not published: on PyPI
the name is matplotlib's. Its wheel goes with the GitHub release.

Everything runs in the workflow `.github/workflows/wheels.yml`, which
starts only by hand or for a tag `v<version>`: never for pushes of
branches or pull requests, as every run costs Actions minutes.

## Once, before the first release

1. **pltrs' licence** is chosen: MIT OR Apache-2.0 (`LICENSE-MIT`,
   `LICENSE-APACHE`). Cargo.toml's `[workspace.package]` gives it to
   every crate (each links the two files, which `cargo package` takes
   in), and `pyproject.toml` to both distributions: `license` in
   `[tool.pltrs-backend.pltrs]` and `[tool.pltrs-backend.matplotlib]`
   (the backend writes it as `License-Expression`), and the two files in
   `license-files` beside the notices of `THIRD_PARTY.md`.
   `python/tests/test_build_backend.py` checks that the three agree.
2. **Decide what the PyPI page links to.** `pyproject.toml`'s `urls` (and
   Cargo.toml's `repository`) point at the GitHub repository, which is
   private: visitors of the PyPI page would find nothing. Make it public,
   or point the links elsewhere. The README is the page's text; its
   links to files of the tree (`python/DESIGN.md`, `RELEASING.md`, ...)
   are absolute GitHub URLs, which work only once the repository is
   public.
3. **PyPI**: with an account that has two-factor authentication, add a
   *pending trusted publisher* (https://pypi.org/manage/account/publishing/):
   PyPI project name `pltrs`, owner `Lorisk1210`, repository `plt-rs`,
   workflow `wheels.yml`, environment `pypi`. The first upload then
   creates the project; no token is ever stored in GitHub. To rehearse,
   do the same on https://test.pypi.org and add
   `repository-url: https://test.pypi.org/legacy/` to the publish step
   for that run.
4. **GitHub**, in the repository's *Settings*:
   * *Environments*: create `pypi`, with yourself as a *required
     reviewer* and *deployment branches and tags* limited to the tag
     pattern `v*`. The publish job waits for your approval there.
   * *Actions > Runners*: the aarch64 Linux wheels are built on an
     x86_64 runner (Zig cross-links them) and tested on
     `ubuntu-24.04-arm`. If the repository's plan does not offer it,
     set the repository variable `LINUX_ARM64_RUNNER` (*Secrets and
     variables > Actions > Variables*) to a runner label that is
     arm64 Linux (a larger or a self-hosted runner). The same holds for
     `windows-11-arm` and `WINDOWS_ARM64_RUNNER`, when `win_arm64` is
     built (the run's input `win_arm64`, or the variable `WIN_ARM64`
     set to `true` for tag runs).
5. **Minutes.** A full run costs roughly 6 hours of billed minutes:
   about 75 Linux minutes, 35 Windows minutes (billed twice) and 25
   macOS minutes (billed ten times). The extension's build (fat LTO,
   one codegen unit) peaks at about 4.8 GB of memory (4.5 GiB) for
   x86_64 and 6.0 GB (5.6 GiB) for aarch64 on Linux, within a standard
   runner's 7 GB. With MSVC it peaked higher, at 8.5 to 9.0 GiB committed
   for the fat LTO (desktop-r1t0nt5, 2026-11-15), while the release
   profile's line tables (`debug = "line-tables-only"`) went through the
   LTO until the link stripped them. The backend leaves them out of the
   extension's build (a wheel ships without them), which brings MSVC's
   peak to 2 to 2.3 GiB.

## Rehearsing

* On this host: `cargo xtask python wheels` builds the sdist, the
  alias and the four Linux wheels into `target/wheels/` (Zig links
  them against glibc 2.17 and musl: `python/tools/zig.sh install`
  first, and `rustup target add aarch64-unknown-linux-gnu
  aarch64-unknown-linux-musl`), and checks them. Then, for each Python
  to try:

  ```sh
  python python/tools/check_dist.py --install --python python3.12 \
      target/wheels/pltrs-*manylinux*x86_64.whl target/wheels/matplotlib-*.whl
  python python/tools/check_dist.py --install --lowest --python python3.12 \
      target/wheels/pltrs-*manylinux*x86_64.whl
  python python/tools/check_dist.py --install target/wheels/pltrs-*.tar.gz
  ```

  On a Mac, `cargo xtask python wheels` builds `arm64`, `x86_64` and
  `universal2` wheels; check them natively and with an x86_64 Python
  under Rosetta.

  On Windows, `cargo xtask python wheels` builds the `win_amd64` wheel,
  the sdist and the alias with MSVC (Visual Studio's C++ build tools;
  `aarch64-pc-windows-msvc` given cross-builds `win_arm64`, to try on an
  arm64 Windows). The toolchain of `rust-toolchain.toml` is the one of
  rustup's default host: where that is `x86_64-pc-windows-gnu`, set
  `RUSTUP_TOOLCHAIN=1.99-x86_64-pc-windows-msvc`, so that the wheel is
  linked as the workflow links it (and the `-gnu` toolchain to try the
  sdist with gnu, as a user of that host builds it). Every interpreter
  is `python.exe`, so name each by its path:

  ```sh
  python python/tools/check_dist.py --install --python C:/.../3.12/python.exe \
      target/wheels/pltrs-*win_amd64.whl target/wheels/matplotlib-*.whl
  ```
* On GitHub: *Actions > wheels > Run workflow*. Everything runs but
  publishing: the archives are the run's artifacts (`pltrs-*` and
  `alias`), to download and try.
* `python python/tools/check_workflows.py` checks the workflow itself
  after any change to it.

## Releasing

1. If native code changed since the order of the extension's functions
   was last written, write it again and commit it: the x86_64 Linux
   wheel is linked in that order (`crates/pltrs-python/symbol-order.txt`),
   and code the file does not name lies among code no program runs,
   which costs every process memory. Twice, as the tool's docstring
   says: `cargo xtask python --release`, then
   `.venv-pltrs/bin/python crates/pltrs-python/tools/symbol_order.py`.
   `python/tests/test_startup_work.py` then holds what a small program
   maps against its ceilings.
2. Set the version in `Cargo.toml`'s `[workspace.package]`: the one
   source of the crates', the wheels' and the alias' versions (the
   alias is `<matplotlib version>+pltrs.<version>`). Give `CHANGELOG.md`
   the version's section (its heading names the version, which
   `python/tests/test_readme_facts.py` holds against `Cargo.toml`, as it
   holds the README's and the changelog's numbers against the tree).
   Commit and push.
3. Tag the commit and push the tag:

   ```sh
   git tag -a v0.1.0 -m "pltrs 0.1.0"
   git push origin v0.1.0
   ```

4. The workflow runs: it first checks that the tag is `v` followed by
   Cargo.toml's version, then builds every archive, installs and tests each
   on its platform (pltrs alone against matplotlib alone, pltrs' smoke
   tests and a matplotlib program, on Python 3.12 and 3.14, on the oldest
   glibc and musl the tags allow, and with the lowest versions of the
   requirements). Nothing is published unless all of that passes.
5. Approve the `pypi` deployment when the publish job asks. It checks
   the archives once more and uploads them to PyPI through trusted
   publishing.
6. A draft GitHub release then holds every archive, the alias wheel
   included. Write its notes (the version's section of `CHANGELOG.md`)
   and publish it.

If any job fails, nothing reaches PyPI. Fix the cause on `main`, delete
the tag (`git push --delete origin v0.1.0; git tag -d v0.1.0`), tag
again and push. A version that reached PyPI can never be uploaded again,
even after deleting it: release the fix as the next version.

## Checking the release

* https://pypi.org/project/pltrs/ lists the sdist and the six (or
  seven) wheels above, and shows the README.
* In fresh venvs on Linux, macOS and Windows:

  ```sh
  python -m pip install pltrs
  python -c "import pltrs.pyplot as plt; plt.plot([1, 3, 2]); plt.savefig('check.png')"
  ```

* From the sdist, with Rust installed: `python -m pip install
  --no-binary pltrs pltrs`.
* pltrs as matplotlib, from the release's alias wheel: `python -m pip
  install pltrs matplotlib-<version>+pltrs.<version>-py3-none-any.whl`,
  then run a matplotlib program unchanged.

A broken release is *yanked* on PyPI (the project's *Manage* page):
pip then no longer picks it, unless asked for that exact version.
