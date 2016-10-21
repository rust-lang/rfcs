- Feature Name: security_advisories
- Start Date: 2016-08-24
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Crates.io should offer an API to release security advisories for crates. Cargo
should offer a new command for the same purpose and warn about vulnerable crate
versions during compilation.

# Motivation
[motivation]: #motivation

Keeping on top of security vulnerabilities for all dependencies of a typical
Rust application is currently extremely hard, if not impossible. Particularly
for Rust this task is relatively hard, since Rust's broader community prefers
many single-purpose crates over larger "collections" of tools (like Boost in
C++).

One might think that a regular `cargo update` may help in such situations.
However, the application developer still does not know which updates were
security updates, and which were not, and therefore doesn't know if a new
release of their application is needed or not. `cargo update` doesn't even
suffice to automatically recieve all security updates. Here's an example.
Assume that:

- There are two crates, `A` and `B`.
- The latest version of `B` is `1.0.0`, which is a complete rewrite and comes
  with an entirely new API. There are no bugfixes, but the API of `0.x` became
  gradually more quirky, to a point that it had to be completely overhauled.
- `A` depends on `B = ^0.9.0`, and did so before `B = 1.0.0` was released. The
  author of `A` doesn't care about updating their code because why change a
  running system?
- The author of `B` doesn't officially support `0.9` anymore. No bugfixes or
  security updates.

If there is now a security issue in all existing versions of `B` (including the
`0.9` and `1.0` series), `B` will get a security patch. However, that patch
will only be available for `1.0` as a backport is costly. The author of `A` is
never notified that they were running a vulnerable version of `B` all the time,
unless they take extra measures to keep themselves informed. Especially in
open-source development (not sponsored by a multinational company) that is
unlikely to be the case.

Cargo and other tooling that builds on top of the proposed new API for
Crates.io could alert crate users of their vulnerabilities, which in turn spurs
them to update their dependencies accordingly. Even if that does not happen,
the additional metadata at least makes it clear which crates are potentially
dangerous to use and which ones not. This not only helps Rust programmers, but
potentially also distributors (such as packagers of Linux distros) and
end-users.

# Detailed design
[design]: #detailed-design

## Crates.io

Similar to yanking, Crates.io should provide an API that allows a user of Cargo
to attach an arbitrary amount of so-called security advisories to crates they
own.

Each advisory gets assigned an ID that is unique within the set of advisories
for the affected crate.  Every advisory should have a unique URL, for example
`https://crates.io/crates/<crate>/advisory/<id>`, where `<id>` is the
advisory's ID. On that URL a human-readable representation of the advisory
should be stored.

Other pages on Crates.io should link to those advisories prominently where
appropriate.

## Cargo

### `cargo advisory`

A command called `advisory` will be added to Cargo. Here is an excerpt of its help
page:

```
$ cargo advisory --help
Generate and upload security advisories for the given or the current crate.

Usage:
    cargo advisory [options] -- [<crate>]
    --filename PATH      The filename to use. Defaults to `./Advisory.toml`.
                         If `-` is given, generated advisories are printed to
                         stdout and advisories to upload are read from stdin.

    --vers VERSION       Versions to release this advisory for. Can be
                         specified multiple times. Only valid in conjunction
                         with --generate.

    --upload/--generate  Whether to upload or generate a advisory. The default
                         is to generate. These options are mutually exclusive.
    [...]
```

Like `yank` it takes a `--vers` option, with two differences:

- if a version is not specified, `advisory` will default to all existing
  versions.

- Version ranges such as `<1.2.6, >1.0.0` can be specified. This is comparable
  to the syntax used for specifying dependencies in the `Cargo.toml`, with the
  exception that `x.y.z` is not equivalent to `^x.y.z`, but means the exact
  version.

Here's the workflow:

1. The user invokes `cargo advisory` without the `--upload` option. Cargo will
   generate a file under `filename`. Cargo should abort if the file already
   exists. The content looks like this:

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

2. The user invokes `cargo advisory --upload`. Cargo verifies the passed file
   against the following rules:

   - the file exists and is valid TOML
   - When an optional key is `false`, this is semantically equivalent to it
     being omitted.
   - the `description` contains not only whitespace. More text than a paragraph
     should be allowed, but not necessarily recommended.
   - `package` exists on Crates.io
   - `versions` is non-empty and only contains versions of `package` published
     on Crates.io
   - `dwf` is not an empty array. It should be ``false`` if there are none.
   
   If not, Cargo should print one or more error messages and exit.

3. When the advisory is found to be valid, Cargo should print a summary, ask
   the user for confirmation and upload it to the package index.  The
   vulnerability ID assigned by Crates.io and optionally the corresponding URL
   should be printed to stdout.

The recommended workflow is to first file the advisory with `cargo advisory`,
and then release the versions that contain the security fix.

### Using vulnerable packages

- `cargo build` and `cargo install` will emit a warning for each vulnerable
  package used, regardless of whether this package is already compiled,
  downloaded or otherwise cached, or whether it is a direct dependency or not:

  ```
  Downloading foo vx.y.z
  Downloading bar vx.y.z
  Warning: bar vx.y.z (dependency of foo vx.y.z) is vulnerable. See https://crates.io/... for details.
  ```

- `cargo publish` will refuse to upload a crate if any version of a direct
  dependency satisfying the constraints in `Cargo.toml` is vulnerable. 
  Indirect dependencies should not trigger this behavior.

  For example, if I have a dependency such as ``bar = "^1.2.3"``, this means
  ``publish`` should refuse to upload my crate even if ``bar=1.2.3`` is not
  vulnerable, as another version satisfying that constraint may be.

The author of a crate that directly depends on a vulnerable crate may still use
vulnerable packages using switch in their `Cargo.toml`. If `iron==0.4.x` has an
advisory with the ID `deadbeef`, the dependent author may use the
`allow_vulnerable` parameter to disable the warnings for `build` and `install`
and the errors for `publish` due to this vulnerability:

```
[dependencies]
iron = { version = "0.4", allow_vulnerable = ["deadbeef"] }
```


This only affects the warnings for `deadbeef` for the current crate. Cargo will
still print warnings:

- for other vulnerabilities.  Each warning has to be explicitly disabled by
  appending its ID to that array.
- if another package in the dependency graph uses a version of `iron` that has
  the `deadbeef` vulnerability, but does not have `allow_vulnerable =
  ["deadbeef"]` set.


Cargo must reject nonexistent vulnerability IDs with a fatal error.

# Drawbacks
[drawbacks]: #drawbacks

There is a risk that users will abuse this system to mark their versions as
deprecated or to call out other kinds of critical bugs such as data loss. This
would make the entire advisory system as semantically worthless.

# Alternatives
[alternatives]: #alternatives

## Ability to mark versions as deprecated

The problem of people using unsupported versions that don't recieve security
updates can be also mitigated by adding the ability to mark versions as
unsupported. [npm](https://www.npmjs.com/) has this feature in the form of `npm
deprecate`.

However, there's a big difference between a version being unsupported and a
version actually having issues. People who use versions that are
semver-incompatible with the latest one are usually aware that they should
eventually update (and in fact there's already tooling to keep on top of that,
such as [cargo-outdated](https://github.com/kbknapp/cargo-outdated)), but, like
in the example in [Motivation](#motivation), don't yet have a good reason to do
so. A security issue would be a good reason, but marking a package as
deprecated does not imply that.

## Extending yanking for security advisories

It has been proposed to [extend the semantics of yanking such that it could be
used for security advisories](https://github.com/rust-lang/cargo/issues/2608).
While this alternative meets the popular aesthetic preference of having generic
commands with a large variety of usecases (over single-purpose commands), using
yanking this way has a few drawbacks:

- Cargo doesn't allow yanked packages to be used in new packages. In the
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
  the user didn't specify one in the advisory (``dwf = false``).


## CVSS

- Scoring vulnerabilities. Should a new field for the usage of CVSS be created?
