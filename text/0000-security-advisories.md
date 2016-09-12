- Feature Name: security_advisories
- Start Date: 2016-08-24
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Crates.io should offer an API to mark crate versions as vulnerable, accompanied
by a structured vulnerability report. Cargo should offer a new command for the
same purpose and warn about vulnerable crate versions during compilation.

# Motivation
[motivation]: #motivation

When compared to other ecosystems such as Python's, Rust's broader community
prefers many single-purpose crates over larger monoliths. This situation,
together with the strongly encouraged practice of pinning MINOR versions of
dependencies, slows down the propagation of critical security fixes.

Assume a crate `W`, which depends on `X`, which depends on `Y`, which depends
on `Z`.  If `Z` releases a new MINOR version including a security fix, it
requires the attention of `Y`'s  and `X`'s maintainers to propagate that
security fix to `W`. What makes this situation worse is that the author of `W`
is never notified that they were running a vulnerable version of `Z` all the
time.

An added API to Crates.io as described above would allow for the creation of
third-party tooling that notifies the author of `Z` about security releases.

The warning emitted by Cargo would further help downstream distributors (Linux
packagers for example) and end users of Rust applications to identify potential
risks in their usage.

# Detailed design
[design]: #detailed-design

## Crates.io

Similar to yanking, Crates.io should provide an API that allows Cargo to attach
a "vulnerable" flag and, if that flag is set, a vulnerability report as
detailed below.

Each version at Crates.io already has a corresponding webpage with an URL of
the format `https://crates.io/crates/<crate>/<version>`. Such webpages should
include a human-readable version of the vulnerability report, if any. The same
applies to the URL `https://crates.io/crates/<crate>/`, where the latest
version is displayed.

## Cargo

### `cargo advisory`

A command called `advisory` will be added to Cargo. Here is an excerpt of its help
page:

```
$ cargo advisory --help

Usage:
    cargo advisory [options] -- [<crate>]
    --vers VERSION      Versions to mark as vulnerable. Can be specified multiple times.
    [...]
```

`advisory` has a similar CLI compared to `yank`.

- It takes exactly the same positional arguments, defaulting to the crate in
  the current working directory.

- Like `yank` it takes a `--vers` option, with two differences:

  - if a version is not specified, `advisory` will default to marking all existing
    versions on Crates.io as vulnerable.

  - Version ranges such as `<1.2.6, >1.0.0` can be specified. This is
    comparable to the syntax used for specifying dependencies in the
    `Cargo.toml`, with the exception that `x.y.z` is not equivalent to
    `^x.y.z`, but means the exact version.

A correct invocation makes Cargo do the following:

1. Cargo will open `$EDITOR` with a file ending with `.toml` that looks like
   this:

   ```
   [vulnerability]
   package = "mypackage"
   versions = ["1.2.0", "1.2.3", "1.2.4", "1.2.5"]

   # It is strongly recommended to request a CVE, or alternatively a DWF, and
   # reference the assigned number here.
   # - CVE: https://iwantacve.org/
   # - DWF: https://distributedweaknessfiling.org/
   dwf = false
   # dwf = "CVE-YYYY-XXXX"
   # dwf = ["CVE-YYYY-XXXX", "CVE-ZZZZ-WWWW"]

   # URL to a long-form description of this issue, e.g. a blogpost announcing
   # the release or a changelog entry (optional)
   url = false

   # Enter a short-form description of the vulnerability here. Preferrably a
   # single paragraph (required)
   description = """
   
   """
   ```

2. After `$EDITOR` exits, Cargo validates the content. This includes verifying
   that:

   - the file is valid TOML
   - none of the table keys have ben removed
   - the `description` contains not only whitespace. More text than a paragraph
     should be allowed, but not wished.
   - `package` exists on Crates.io and the versions specified in `versions` exist
   - `dwf` is not an empty array. It should be ``false`` if there are none.
   
   If not, Cargo should print an error message and wait for the user to either
   hit enter or `^C`. In the former case, open the editor with the same file
   again.

3. When the vulnerability report is found to be valid, Cargo should print a
   summary, ask the user for confirmation and upload it to the package index.

The recommended workflow is to first file the vulnerability report with `cargo
advisory`, and then release the versions that contain the security fix.

### Using vulnerable packages

- `cargo build` and `cargo install` will emit a warning for each vulnerable
  package used, regardless of whether this package is already compiled,
  downloaded or otherwise cached, or whether it is a direct dependency or not:

  ```
  Downloading foo vx.y.z
  Downloading bar vx.y.z
  Warning: bar vx.y.z (dependency of foo vx.y.z) is vulnerable. See https://crates.io/... for details.
  ```

- `cargo test` make those warnings hard errors.

- `cargo publish` will refuse to upload a crate if any version of a direct
  dependency satisfying the constraints in `Cargo.toml` is vulnerable. 
  Indirect dependencies should not trigger this behavior.

  For example, if I have a dependency such as ``bar = "^1.2.3"``, this means
  ``publish`` should refuse to upload my crate even if ``bar=1.2.3`` is not
  vulnerable, as another version satisfying that constraint may be.

The author of a crate that directly depends on a vulnerable crate may disable
these warnings/errors with a switch in their `Cargo.toml`. If `iron==0.4.x` is
vulnerable, the dependent author may use the `allow_vulnerable` key to disable
all the above-described warnings and errors:

```
[dependencies]
iron = { version = "0.4", allow_vulnerable = true }
```

This doesn't affect other crates that depend on ``iron==0.4``. Cargo will still
print warnings if another package in the dependency graph depends on the
vulnerable ``iron==0.4``.

To prevent preemptive usage of `allow_vulnerable` or other misuse, `cargo
build` will issue an error if `iron` is not vulnerable but `allow_vulnerable`
is `true`.

# Drawbacks
[drawbacks]: #drawbacks

There is a risk that users will abuse this system to mark their versions as
deprecated or to call out other kinds of critical bugs such as data loss. This
would make the ``vulnerable``-flag semantically worthless.

# Alternatives
[alternatives]: #alternatives

## Extending yanking for security advisories

It has been proposed to [extend the semantics of yanking such that it could be
used for security advisories](https://github.com/rust-lang/cargo/issues/2608).
While this alternative meets the popular aesthetic preference of having generic
commands with a large variety of usecases (over single-purpose commands), using
yanking this way has a few drawbacks:

- Cargo dosen't allow yanked packages to be used in new packages. In the
  author's opinion, people who know what they're doing should be allowed to
  depend on vulnerable packages, as they might use the crate in a way that
  poses no security threat.

  Some vulnerabilities can be mitigated in ways other than upgrading a crate,
  like making local configuration changes. Some vulnerabilities may affect
  optional functionality not everyone is using, or functionality that can be
  compiled out by e.g. disabling certain cargo feature settings for that crate.
  Some may be relatively innocuous and/or hard-to-exploit and therefore not
  warrant an immediate upgrade. Sometimes no action (other than setting
  ``allow_vulnerable = true``) is required at all because the dependent crate
  never used the vulnerable functionality to begin with.

  At the same time it doesn't make sense to depend on packages that don't
  compile, and currently yanking is primarily used to mark such packages.

- Cargo doesn't give any advice about further procedure when yanking a package.
  I think in the context of security vulnerabilities this is very much needed,
  as few OSS maintainers are exposed to this problem regularly enough to know
  what they're doing.

- Lastly, the data exposed via the Crates.io would be a lot less structured.
  Automatic security notifications via third-party tooling would be impossible
  because there is no way to determine whether a package was yanked because of
  a security vulnerability or not.

Most of these problems can be fixed by asking the user to attach a "reason" to
their yanked packages, such as ``security``, ``deprecation``, ``broken`` (and
then make Cargo's behavior dependent on that). However, at that that point
``yank`` is no longer generic (as in a function having type parameters), but
simply a lot of single-purpose commands stuffed into one (as in function
overloading in Java).  And the name "yank" wouldn't make sense for crate
versions that may still be available (depending on the "reason").

# Unresolved questions
[unresolved]: #unresolved-questions

## DWF vs CVE

- It may be counterintuitive that one can specify CVEs in the DWF parameter.
  Should it be called ``cve`` instead even though it can also be used for DWFs?

Comparison:

- CVEs are more popular
- Applying for a CVE number is a manual process and requires review by a human.
  DWFs can be automatically managed assigned by Crates.io

## What to do if dwf = false

- Crates.io could apply for blocks of DWF IDs and automatically assign them if
  the user didn't specify one in the vulnerability report (``dwf = false``).


## CVSS

- Scoring vulnerabilities. Should a new field for the usage of CVSS be created?
