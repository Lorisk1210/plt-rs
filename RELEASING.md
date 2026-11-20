# Releasing pltrs

A release of version `X.Y.Z` publishes, all from one tagged commit:

* **pltrs (Python)** on PyPI, the distribution `pltrs`: its sdist and one
  wheel per platform, all tagged `cp312-abi3`, so one wheel serves CPython
  3.12 and every later version. The workflow `.github/workflows/wheels.yml`
  builds, tests and uploads them.
* **pltrs (Rust)** on crates.io: the crate `pltrs` and the eight crates it
  is built from, all at the version of `Cargo.toml`, published by hand
  with `cargo publish` (below).
* A GitHub release holding every archive, among them the wheel of the
  second distribution, `matplotlib` (pltrs as matplotlib, `pip install .
  -C distribution=matplotlib`). That distribution is not published on
  PyPI, where the name is matplotlib's.

| wheel | runs on |
|-------|---------|
| `manylinux_2_17_x86_64.manylinux2014_x86_64`, `manylinux_2_17_aarch64.manylinux2014_aarch64` | Linux with glibc 2.17 or later (CentOS 7, Ubuntu 14.04, Debian 8 and newer) |
| `musllinux_1_2_x86_64`, `musllinux_1_2_aarch64` | Linux with musl 1.2 or later (Alpine 3.13 and newer) |
| `macosx_10_12_universal2` | macOS 10.12 or later on Intel, macOS 11 or later on Apple silicon (the Intel half runs first in the workflow's job `macos-intel`) |
| `win_amd64` (`win_arm64` when asked for) | 64-bit Windows 10 and later |

On glibc 2.17, a draw started from inside a callback (a tick formatter or
an event handler saving its figure) leaves about 3 to 10 KiB of the
64 KiB the stack guard keeps, against 17 and more on glibc 2.28, current
glibc and musl (task REL3, 2026-11-18; versions in between not
measured): it works, with less to spare before a runaway recursion ends
in a crash instead of a `RecursionError`.

The crates, in the order they are published (each needs the ones
before it): `pltrs-core`, `pltrs-codec`, `pltrs-font`, `pltrs-raster`,
`pltrs-platform`, `pltrs-text`, `pltrs-render`, `pltrs-ui`, `pltrs`.
`pltrs-python`, `pltrs-spec`, `xtask` and `pltrs-examples` are never
published (`publish = false`).

The workflow starts only by hand or for a tag `v<version>`: never for
pushes of branches or pull requests, as every run costs Actions minutes.

## Once, before the first release

1. **pltrs' licence** is MIT OR Apache-2.0 (`LICENSE-MIT`,
   `LICENSE-APACHE`).
   * `Cargo.toml`'s `[workspace.package]` gives it to every crate.
   * `pyproject.toml` gives it to both distributions: `license` in
     `[tool.pltrs-backend.pltrs]` and `[tool.pltrs-backend.matplotlib]`
     (the backend writes it as `License-Expression`), and the two files
     in `license-files`, beside the notices of `THIRD_PARTY.md`.
   * A crate is packaged from its own directory, so each published crate
     holds copies of the two files, and the rows of `THIRD_PARTY.md` and
     the texts of `LICENSES/` that concern it. After changing
     `THIRD_PARTY.md` or `LICENSES/`, run
     `python python/tools/crate_notices.py`, which writes them.
   * `python/tests/test_crates_io.py` and `python/tests/test_build_backend.py`
     check that all of these agree.
2. **Decide what the PyPI page and crates.io link to.** The links all
   point at the GitHub repository, which is private: visitors would find
   nothing. They are:
   * `pyproject.toml`'s `urls`;
   * `Cargo.toml`'s `repository`, on every crate's page on crates.io and
     docs.rs (no crate sets `documentation`: crates.io then links to
     docs.rs);
   * the README's links to files of the tree (`python/DESIGN.md`,
     `RELEASING.md`, ...), which are absolute GitHub URLs. The README is
     the text of the PyPI page and of the crate `pltrs`' page.

   Make the repository public, or point the links elsewhere.
3. **PyPI**: with an account that has two-factor authentication, add a
   *pending trusted publisher* (https://pypi.org/manage/account/publishing/):
   PyPI project name `pltrs`, owner `Lorisk1210`, repository `plt-rs`,
   workflow `wheels.yml`, environment `pypi`. The first upload then
   creates the project; no token is ever stored in GitHub. To rehearse,
   do the same on https://test.pypi.org and add
   `repository-url: https://test.pypi.org/legacy/` to the publish step
   for that run.
4. **crates.io**:
   * Sign in to https://crates.io with GitHub, and verify the account's
     e-mail address (crates.io refuses to publish without one).
   * Make an API token at https://crates.io/settings/tokens with the
     scopes `publish-new` and `publish-update`, limited to the crates
     `pltrs*`. On the machine you publish from (Linux or macOS), store
     it with `cargo login`, which asks for it. The token stays on that
     machine (`~/.cargo/credentials.toml`), never in the repository or
     in GitHub.
   * The names of the nine crates were free on 2026-11-17. The first
     upload of each makes the account its owner.
5. **GitHub**, in the repository's *Settings*:
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
6. **Minutes.** A full run costs roughly 10 hours of billed
   minutes: about 75 Linux minutes, 35 Windows minutes (billed twice),
   25 macOS minutes on arm64 for the universal2 wheel, and, since
   2026-11-17, an estimated 15 to 25 minutes on an Intel Mac
   (`macos-15-intel`) that tests the wheel's x86_64 half natively (all
   macOS minutes are billed ten times; the Intel job has not run yet).
   * **Linux memory:** the extension's build (fat LTO, one codegen unit)
     peaks at about 4.8 GB of memory (4.5 GiB) for x86_64 and 6.0 GB
     (5.6 GiB) for aarch64, within a standard runner's 7 GB.
   * **MSVC memory:** with MSVC the build peaked higher, at 8.5 to
     9.0 GiB committed for the fat LTO (the Windows PC, 2026-11-15),
     while the release profile's line tables
     (`debug = "line-tables-only"`) went through the LTO until the link
     stripped them. The profile the extension is built with
     (`[profile.extension]` in `Cargo.toml`) leaves them out (a wheel
     ships without them), which brings MSVC's peak to 2 to 2.3 GiB.

## Rehearsing

Nothing here publishes anything.

### The crates

From the root of the tree, on Linux or macOS:

```sh
python python/tools/crate_notices.py --check
cargo publish --dry-run -p pltrs-core -p pltrs-codec -p pltrs-font -p pltrs-raster \
    -p pltrs-platform -p pltrs-text -p pltrs-render -p pltrs-ui -p pltrs
```

* **What the dry run does:** cargo packages the nine crates in
  dependency order, builds each from its package alone (the others come
  from a local registry of the packages just made), and stops before
  uploading ("aborting upload due to dry run"). With uncommitted
  changes, add `--allow-dirty`.
* **Name the nine:** `cargo publish --workspace` would try to package
  `xtask` too, and fail.
* **Sizes:** the packages are in `target/package/*.crate`. crates.io
  takes at most 10 MB per crate; `pltrs-font`, the largest for its
  fonts, packs to about 6 MB.

### pltrs (Python) on Linux

1. **Once: the tools.** Zig, which links the Linux wheels against glibc
   2.17 and musl, and the two aarch64 Rust targets:

   ```sh
   python/tools/zig.sh install
   rustup target add aarch64-unknown-linux-gnu aarch64-unknown-linux-musl
   ```

2. **Build.** `cargo xtask python wheels` builds the sdist, the alias and
   the four Linux wheels into `target/wheels/`, and checks each archive
   as an index would (each wheel's fat LTO build took 8 to 10 minutes on
   a 32-core machine under other load, 2026-11-17).

3. **Check** each Python to try (`check_dist.py --help`; each run makes
   its venvs under `target/check-dist`, or the directory given with
   `--work`):

   ```sh
   python python/tools/check_dist.py --install --python python3.12 \
       target/wheels/pltrs-*manylinux*x86_64.whl target/wheels/matplotlib-*.whl
   python python/tools/check_dist.py --install --lowest --python python3.12 \
       target/wheels/pltrs-*manylinux*x86_64.whl
   python python/tools/check_dist.py --install target/wheels/pltrs-*.tar.gz
   ```

   * **Time:** each check took about 10 minutes, most of it pltrs' smoke
     tests (about 3,560), which run serially.
   * **Several at once:** checks of several Pythons may run at the same
     time from one tree, each with its own `--work`.
   * **Memory:** the sdist's check builds it (pip runs cargo with fat
     LTO), which takes about 4.5 GiB.

### pltrs (Python) on a Mac

1. **Build.** `cargo xtask python wheels` builds the `arm64`, `x86_64`
   and `universal2` wheels, the sdist and the alias, and audits all
   five (16 minutes on an M2 Max). The x86_64 wheel is cross-compiled:
   building runs no x86_64 code.

2. **Check the `arm64` and `universal2` wheels natively**, with an
   arm64 Python of each version to try (about 3 minutes each):

   ```sh
   python python/tools/check_dist.py --install --python <arm64 python3.12|3.13|3.14> \
       target/wheels/pltrs-*macosx_11_0_arm64.whl target/wheels/matplotlib-*.whl
   python python/tools/check_dist.py --install --python <arm64 python3.12|3.13|3.14> \
       target/wheels/pltrs-*universal2.whl target/wheels/matplotlib-*.whl
   python python/tools/check_dist.py --install --lowest --python <arm64 python3.12> \
       target/wheels/pltrs-*macosx_11_0_arm64.whl
   python python/tools/check_dist.py --install --python <arm64 python3.14> \
       target/wheels/pltrs-*.tar.gz
   ```

   **Memory:** the sdist's check builds it (pip runs cargo with fat LTO),
   which needs more than 4 GB of memory. A run limited to 4 GB per
   process was killed; one limited to 12 GB passed.

3. **Check the x86_64 code statically only.** Do not run an x86_64
   Python under Rosetta on the project's Mac: on 2026-11-11 a native
   stack overflow inside a Python run under Rosetta left its processes
   unkillable, and every other x86_64 program on that Mac hung at launch
   for hours. In the unpacked `universal2` wheel (and the `x86_64` one):

   ```sh
   lipo -archs pltrs/_native.abi3.so            # x86_64 arm64
   otool -l pltrs/_native.abi3.so | grep -A4 -E 'LC_VERSION_MIN_MACOSX|LC_BUILD_VERSION'
   otool -L pltrs/_native.abi3.so               # system libraries only
   nm -gU pltrs/_native.abi3.so | grep PyInit   # the module's entry point
   ```

   The x86_64 code runs in the workflow's job `macos-intel`, natively on
   GitHub's Intel runner `macos-15-intel`, or on an Intel Mac.

### pltrs (Python) on Windows

`cargo xtask python wheels` builds the `win_amd64` wheel, the sdist and
the alias with MSVC (Visual Studio's C++ build tools;
`aarch64-pc-windows-msvc` given cross-builds `win_arm64`, to try on an
arm64 Windows).

* **The toolchain:** the toolchain of `rust-toolchain.toml` is the one
  of rustup's default host. Where that is `x86_64-pc-windows-gnu`, set
  `RUSTUP_TOOLCHAIN=1.99-x86_64-pc-windows-msvc`, so that the wheel is
  linked as the workflow links it, and the `-gnu` toolchain to try the
  sdist with gnu, as a user of that host builds it.
* **The interpreters:** every one is `python.exe`, so name each by its
  path:

  ```sh
  python python/tools/check_dist.py --install --python C:/.../3.12/python.exe \
      target/wheels/pltrs-*win_amd64.whl target/wheels/matplotlib-*.whl
  ```

* **What has run on Windows:** on 2026-11-19 and 20, on a PC with
  Windows 11 (German locale), MSVC 14.43 and an AVX2 processor (Intel
  i9-10900K), on the tree of `df82b08e` with the clock fix and the tests
  that became `3a3fef18`: the wheel, the alias and the native code are
  `3a3fef18`'s, whose sdist differs from the one built there in tests
  and in this document only.
  * The Rust workspace: clippy (MSVC and gnu) and fmt clean; `cargo test
    --workspace` passed 6,341 and failed none; the spec passed 48,559
    probes at `x86_64-v3` and 48,558 at `v2`, 48,446 without TeX, with no
    failure (the 4 probes of `savefig` with a distiller skipped: its
    program is not installed); figures 3,402 of 3,402; pixels and vectors.
  * `cargo xtask python test` passed 4,036 tests and failed 1, the colour
    codes `test_flaky_outcomes` expects, which `-n 8` in `PYTEST_ADDOPTS`
    changes.
  * The `win_amd64` wheel (19.1 MiB, 543 files), the sdist (15.8 MiB) and
    the alias (1.6 MiB) were built with MSVC and passed the audit and
    `twine check --strict`. `_native.pyd` imports only Windows', the
    UCRT's and Python's libraries (`python3.dll`, `VCRUNTIME140`).
  * `check_dist.py --install` with the wheel and the alias passed 4,011
    tests on Python 3.12 and 4,010 on 3.13 and 3.14 (one test fewer,
    before the clock fix), then the alias stage and the drop-in program,
    which drew its 5 figures. `--lowest` passed 3,958; the sdist, built
    from source by pip with MSVC (about 11 minutes a build), 4,010.
  * Before Python 3.13, `time.monotonic` and `time.time` step by 15.6 ms
    on Windows. Two tests that measured with them failed on 3.12 there
    (`benchmarks/compare`'s child, the kernel loop's wait); both measure
    on the performance counter now.
  * `benchmarks/compare/compare.py` ran headless on an example and on a
    copy of it under a path with a space and umlauts (`--runs 3 --look
    both --video`): the same picture under both libraries, the clips
    written as animated WebP (no ffmpeg on the PC). Its peak-memory column
    is empty on Windows.
  * matplotlib's suite: of the 10,159 tests that pass under matplotlib
    there, 10,157 pass under pltrs. The other 2 expect matplotlib's
    configuration directory (`test_configdir_uses_localappdata_on_windows`,
    `test_configdir_uses_userprofile_on_windows_if_exists`; pltrs' is
    `%LOCALAPPDATA%\pltrs`, see `DIFFERENCES.md`). Without Developer Mode
    the suite's site of links is a copy, in which `_outside_package`
    recurses: the tests ran against the package itself, with copies of the
    sdist's tests, and the GUI tests in the site.
  * matplotlib's 138 window tests (Qt, Tk, the interactive backends) ran
    under both libraries in the service session, which has no screen, and
    passed; nothing has run on a visible desktop.
  * The notebooks: 1,093 of 1,093 in Jupyter under both libraries, their
    suites 1,091 of 1,091.
  * The libraries' suites, 74 libraries: of the 23,979 tests that pass
    under matplotlib there, 23,977 pass under pltrs; the other 2 are
    pytest-mpl's comparisons, whose outcome changes from run to run under
    matplotlib itself and is the same under both alone. healpy has no
    Windows wheel and is left out.
  * Run on the earlier tree of `a855aaca` only (2026-11-18): a first
    user's steps in fresh venvs (the wheel alone, then with the alias,
    pandas, seaborn and a Sphinx build, with configuration and working
    directories whose names hold a space and umlauts, and the
    uninstall), and the sdist's build without cargo on `PATH`, which
    stops with a message that names rustup.
  * Not run on Windows: windows on a visible desktop, and `win_arm64`,
    which has never been built: the workflow's run is its first.

### On GitHub

*Actions > wheels > Run workflow*. Everything runs but publishing: the
archives are the run's artifacts (`pltrs-*` and `alias`), to download
and try. Run `python python/tools/check_workflows.py`, which checks the
workflow itself, after any change to it.

## Releasing

The commands, in order, for version `0.1.0`; for another version, replace
it everywhere. They run from a clone of the repository on Linux or
macOS, with `origin` the GitHub repository and the crates.io token
stored (`cargo login`). **Each command marked PUBLISHES cannot be undone:
a version uploaded to PyPI or crates.io can never be uploaded again,
even after it is deleted or yanked.**

1. **The symbol order, if native code changed** since the order of the
   extension's functions was last written. The manylinux x86_64 wheel
   (and an extension built from the sdist on x86_64 Linux with glibc) is
   linked in that order (`crates/pltrs-python/symbol-order.txt`), and
   code the file does not name lies among code no program runs, which
   costs every process memory. The other wheels are not ordered:
   `build.rs` gives the order to glibc links on x86_64 alone, as its
   names are those of the host's build (the musllinux x86_64 wheel holds
   8,252 KiB of the extension's code after the import, against 3,388
   for the manylinux one, 2026-11-18). Run it twice, as the tool's docstring
   says, then commit the file; `python/tests/test_startup_work.py` then
   holds what a small program maps against its ceilings:

   ```sh
   cargo xtask python --release
   .venv-pltrs/bin/python crates/pltrs-python/tools/symbol_order.py
   cargo xtask python --release
   .venv-pltrs/bin/python crates/pltrs-python/tools/symbol_order.py
   git add crates/pltrs-python/symbol-order.txt
   git commit -m "pltrs-python: the symbol order of the release"
   ```

2. **The version.** `Cargo.toml`'s `[workspace.package] version` is the
   one source of the crates', the wheels' and the alias' versions (the
   alias is `<matplotlib version>+pltrs.<version>`).
   * **A new version** also changes the nine crates' requirements on
     each other in `[workspace.dependencies]`
     (`version = "=0.1.0"`), which must name it exactly.
   * **The changelog:** `CHANGELOG.md`'s section heading names the
     version (`python/tests/test_readme_facts.py` holds it against
     `Cargo.toml`). Replace its `(unreleased)` by the date.
   * **Check, then commit and push `main`:**

   ```sh
   git switch main && git pull --ff-only
   python python/tools/crate_notices.py --check
   cargo publish --dry-run -p pltrs-core -p pltrs-codec -p pltrs-font -p pltrs-raster \
       -p pltrs-platform -p pltrs-text -p pltrs-render -p pltrs-ui -p pltrs
   git commit -am "pltrs 0.1.0"
   git push origin main
   ```

3. **A rehearsal run of the workflow: the user decides.** It costs the
   minutes of a full run and publishes nothing:

   ```sh
   gh workflow run wheels.yml --ref main          # add -f win_arm64=true for win_arm64
   gh run watch --exit-status $(gh run list --workflow wheels.yml --event workflow_dispatch --limit 1 --json databaseId --jq '.[0].databaseId')
   ```

   It verifies what no machine of the project has run:
   * the x86_64 half of the macOS wheel, installed and tested natively
     on an Intel Mac (`macos-15-intel`): until then it has only been
     read statically;
   * the aarch64 Linux wheels on arm64 hardware (`ubuntu-24.04-arm`, or
     the runner `LINUX_ARM64_RUNNER` names): here they ran under qemu
     only;
   * the Windows wheel built and tested on GitHub's runner with MSVC,
     and, with `win_arm64=true`, the `win_arm64` wheel on an arm64
     Windows (`windows-11-arm`), which has never been built or run;
   * the workflow itself, which has never run: its runners, actions,
     artifacts and permissions.

   Without it, the tag's run does all of this before anything is
   published, and a failure costs a deleted tag and a new one (below).

4. **The tag**, which starts the workflow. This publishes nothing yet,
   but the run costs the minutes of a full run:

   ```sh
   git tag -a v0.1.0 -m "pltrs 0.1.0"
   git push origin v0.1.0
   gh run watch --exit-status $(gh run list --workflow wheels.yml --event push --limit 1 --json databaseId --jq '.[0].databaseId')
   ```

   * **What the run does first:** it checks that the tag is `v`
     followed by `Cargo.toml`'s version.
   * **Then the archives:** it builds every archive and installs and
     tests each on its platform: pltrs alone against matplotlib alone,
     pltrs' smoke tests and a matplotlib program, on Python 3.12 and
     3.14, on the oldest glibc and musl the tags allow, and with the
     lowest versions of the requirements.
   * **Nothing is published unless all of that passes.**

5. **PyPI. PUBLISHES.** When the publish job waits, approve it: on the
   run's page (*Actions > wheels > the run*), *Review deployments*, tick
   `pypi`, *Approve and deploy*. It checks the archives once more and
   uploads them through trusted publishing. Then
   https://pypi.org/project/pltrs/0.1.0/ lists the sdist and the six
   (or seven) wheels.

6. **crates.io. PUBLISHES,** each command one crate. From a clean
   checkout of the tag, one command at a time, in this order:

   ```sh
   git switch --detach v0.1.0
   git status --porcelain              # must print nothing
   cargo publish -p pltrs-core
   cargo publish -p pltrs-codec
   cargo publish -p pltrs-font
   cargo publish -p pltrs-raster
   cargo publish -p pltrs-platform
   cargo publish -p pltrs-text
   cargo publish -p pltrs-render
   cargo publish -p pltrs-ui
   cargo publish -p pltrs
   git switch main
   ```

   * **The wait between crates:** each `cargo publish` builds the crate
     from its package, uploads it, and waits until crates.io's index
     lists it (up to a minute), so the next command finds it. Run them
     one after the other. If cargo says it timed out waiting, wait a
     minute before the next command.
   * **A refusal:** crates.io limits how many new crates an account
     creates in a short time. If a command answers `429 Too Many
     Requests` ("published too many new crates"), nothing of that
     crate was published: wait as long as the message says, then run
     the same command again, and go on.
   * **Publish from Linux or macOS:** a Windows checkout may convert
     line endings (`core.autocrlf`), and the packages would then differ
     from the tag.

7. **The GitHub release. PUBLISHES.** After the publish job, a draft
   release holds every archive, the alias wheel included. Give it the
   version's section of `CHANGELOG.md` as its notes and publish it:

   ```sh
   awk '/^## .*0\.1\.0/{on=1; next} /^## /{on=0} on' CHANGELOG.md > target/release-notes.md
   gh release edit v0.1.0 --notes-file target/release-notes.md --draft=false
   ```

If a job of the workflow fails, nothing reaches PyPI:

1. Fix the cause on `main`.
2. Delete the tag: `git push --delete origin v0.1.0; git tag -d v0.1.0`.
3. Tag again and push.

Once a version is on PyPI or crates.io, it cannot be uploaded again,
even after it is deleted: release the fix as the next version. If PyPI
has the version but a crate failed to publish, fix the crate and publish
it as the next version of every crate (the nine always share one).

## Checking the release

* **The pages:**
  * https://pypi.org/project/pltrs/ lists the sdist and the six (or
    seven) wheels above, and shows the README.
  * https://crates.io/crates/pltrs and the eight others show version
    0.1.0.
  * https://docs.rs/pltrs shows the documentation once docs.rs has
    built it from the upload (it queues builds), for the systems of
    `[package.metadata.docs.rs]`: Linux, macOS on Apple silicon,
    Windows.
* **pltrs (Python)**, in fresh venvs on Linux, macOS and Windows:

  ```sh
  python -m pip install pltrs
  python -c "import pltrs.pyplot as plt; plt.plot([1, 3, 2]); plt.savefig('check.png')"
  ```

  On a Mac without a window server, or to keep windows away, set
  `MPLBACKEND=agg` (the default backend there is `macosx`).

* **From the sdist,** with Rust installed (and, on a Mac, Xcode's
  command-line tools, for the linker): `python -m pip install
  --no-binary pltrs pltrs`.
* **pltrs as matplotlib,** from the release's alias wheel: `python -m pip
  install pltrs matplotlib-<version>+pltrs.<version>-py3-none-any.whl`,
  then run a matplotlib program unchanged. It draws in matplotlib's look.
* **pltrs (Rust)**, in a new crate:

  ```sh
  cargo new check && cd check && cargo add pltrs@0.1.0
  cargo run   # with a main drawing a figure, as the README's first example
  ```

A broken release is *yanked*:

* **On PyPI**, on the project's *Manage* page: pip then no longer picks
  it, unless asked for that exact version.
* **On crates.io**, with `cargo yank --version 0.1.0 <crate>` for each
  crate: new projects no longer pick it, and those whose `Cargo.lock`
  names it keep building.
