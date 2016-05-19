- Feature Name:
- Start Date: 2016-05-20
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a compiler flag that emits crate dependencies on a best-effort basis.

# Motivation
[motivation]: #motivation

When working with projects that consist of dozens or hundreds of compilation
units, it is useful to have some kind of automatic dependency management in the
build system. Otherwise such information has to be maintained in at least two
locations: Once in the compilation unit itself via an `extern crate X` statement
and once in the build system.

To this end, this RFC proposes a new compiler flag which emits the names of the
crates a compilation unit depends on to a text file.

# Detailed design
[design]: #detailed-design

Add a new compiler flag `-Z emit-crate-deps`. If the flag is passed, the
compiler emits a file `<crate name>.crate_deps` which contains a list of all
crates the compilation unit depends on.

If the flag `-Z parse-only` is passed, the file is emitted immediately after
parsing. Otherwise it's emitted after macro expansion.

The crates are collected by walking the AST and inspecting all `extern crate`
statements. If the statement is of the form `extern crate X as Y`, `X` but not
`Y` is added to the list.

The file contains the names of the crates separated by `\n`.

# Drawbacks
[drawbacks]: #drawbacks

If `-Z parse-only` is passed, the created file is, in general, not precise. This
is because macro expansion can itself create new `extern crate` statements.
However, macro expansion does, in general, depend on the dependencies having
already been compiled.

The `-Z parse-only` variant is, however, essential for the first round of
dependency resolution in the build system.

I assume that the generation of `extern crate` statements is a rarely used
features. If it is used, the problem can be mitigated by tracking only the
generated statements separately in the build system.

# Alternatives
[alternatives]: #alternatives

None

# Unresolved questions
[unresolved]: #unresolved-questions

None
