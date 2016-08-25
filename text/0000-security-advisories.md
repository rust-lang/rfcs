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

When compared to other ecosystems such as Python's, Rust's packaging tooling
encourages many single-purpose crates instead of larger monoliths. This
situation, together with the strongly encouraged practice of pinning MINOR
versions of dependencies, slows down the propagation of critical security
fixes.

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

### `cargo vuln`

A command called `vuln` will be added to Cargo. Here is an excerpt of its help
page:

    $ cargo vuln --help

    Usage:
        cargo vuln [options] -- [<crate>]
        --vers VERSION      Versions to mark as vulnerable. Can be specified multiple times.
        [...]

`vuln` has a similar CLI compared to `yank`.

- It takes exactly the same positional arguments, defaulting to the crate in
  the current working directory.

- Like `yank` it takes a `--vers` option, with two differences:

  - if a version is not specified, `vuln` will default to marking all existing
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
   # Edit this as you wish
   package = "mypackage"
   versions = ["1.2.0", "1.2.3", "1.2.4", "1.2.5"]

   # It is recommended to request a CVE at https://iwantacve.org/ and
   # reference the assigned number here.
   # cve = "CVE-YYYY-XXXX"
   cve = false

   # Enter a short-form description of the vulnerability here. Preferrably a
   # single paragraph.
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
   
   If not, Cargo should print an error message and wait for the user to either
   hit enter or `^C`. In the former case, open the editor with the same file
   again.

3. When the vulnerability report is found to be valid, Cargo should print a
   summary, ask the user for confirmation and upload it to the package index.

The recommended workflow is to first file the vulnerability report with `cargo
vuln`, and then release the versions that contain the security fix.

### Using vulnerable packages

- `cargo build` and `cargo install` will emit a warning for each vulnerable
  package used, regardless of whether this package is already compiled,
  downloaded or otherwise cached, or whether it is a direct dependency or not:

      Downloading foo vx.y.z
      Downloading bar vx.y.z
      Warning: bar vx.y.z (dependency of foo vx.y.z) is vulnerable. See https://crates.io/... for details.

- Similarly `cargo test` will refuse to compile or use vulnerable packages.

- `cargo publish` will refuse to upload a crate if the latest version of a
  direct dependency satisfying the constraints in `Cargo.toml` is vulnerable.

The author of a crate that directly depends on a vulnerable crate may disable
this behavior with a switch in their `Cargo.toml`. If `iron==0.4.x` is
vulnerable, the dependent author may use the `allow_vulnerable` key to disable
all the above-described warnings and errors:

    [dependencies]
    iron = { version = "0.4", allow_vulnerable = true }

This doesn't affect other crates that depend on ``iron==0.4``. Cargo will still
print warnings etc. if another package in the dependency graph depends on the
vulnerable ``iron==0.4``.

To prevent preemptive usage of `allow_vulnerable` or other misuse, `cargo
build` will issue an error if `iron` is not vulnerable but `allow_vulnerable`
is `true`.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Alternatives
[alternatives]: #alternatives

What other designs have been considered? What is the impact of not doing this?

# Unresolved questions
[unresolved]: #unresolved-questions

What parts of the design are still TBD?
