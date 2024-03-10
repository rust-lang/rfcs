- Feature Name: `respect-lockfiles`
- Start Date: 2024-03-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Make `cargo install` respect lockfiles by default.

# Motivation
[motivation]: #motivation

Currently, `cargo install` does not respect `Cargo.lock` files without
`--locked`, using the latest semver-compatible dependencies.

This causes dependees to break when a dependency releases a new
semver-compatible version with a compilation error or bug.

By respecting the lockfile when running `cargo install`, these breakages are
avoided.

Additionally, users often find it surprising that `cargo build` may succeed
when `cargo install` fails, and this avoids that surprise by giving them the
same behavior with respect to lockfiles.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Since it is the default assumption that `cargo build` and `cargo install` have
the same behavior with respect to lockfiles, no documentation is needed beyond
existing documentation for `cargo build` and lockfiles.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When running `cargo install`, lockfiles are respected.

# Drawbacks
[drawbacks]: #drawbacks

Currently, when running `cargo install`, the latest semver-compatible versions
of dependencies will be selected. This can be beneficial if these versions have
bugfixes.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The current behavior of `cargo install` is often surprising, since the default
assumption is that it behaves the same way as `cargo build`. Additionally, not
respecting lockfiles often causes issues, and those issues are usually
incorrectly reported to the dependee, and not the broken dependency.

Some indications that the current behavior is problematic:

- [Searching Google](https://www.google.com/search?q=cargo+install+broken+%22--locked%22)
turns up a large number of issues.

- [Searching GitHub for `cargo install --locked`](https://github.com/search?q=%22cargo+install+--locked%22&type=code)
  turns up a large number of results, suggesting that `cargo install --locked`
  is often used, negating much of the possible benefit of respecting lockfiles.

- [Searching markdown files on GitHub for `cargo install --locked`](https://github.com/search?q=path%3A*.md+%22cargo+install+--locked%22&type=code)
  turns up a large number of results, suggesting that crate authors document
  `cargo install --locked` as the way to install their crates, negating much of the possible benefit of respecting lockfiles.

- [The GitHub issue discussing this](https://github.com/rust-lang/cargo/issues/7169)
  has a great number of likes, and there are around 100 issues and PRs which link to the issue,
  which are primary breakages caused by `cargo install` not respecting lockfiles.

The benefit of not respecting lockfiles is that new versions of dependencies
may introduce bugfixes which `cargo install` will then pick up. However, new
versions of dependencies may also *introduce* bugs, which are more likely to
cause issues in dependees, since the old versions of the dependency have been
tested in the dependee, and the new versions have not.

Additionally, this makes tracking down issues in a built binary very difficult.
If a user has installed a binary with `cargo install`, and then reports issues
with that binary, even if the version of the binary is known, it is
more-or-less impossible to know which versions of which dependencies were used,
since it depends on *when* `cargo install` was run, and the update history of
all dependencies. This makes `cargo install`ed binaries essential black boxes
with unknown dependency versions.

One possible alternative is to add a `--unlocked` flag, and require that one of
`--locked` and `--unlocked` be passed to `cargo install`, avoiding the element
of surprise. This however would be extremely disruptive, as all instances of
`cargo install` invocations not using `--locked` would need to be changed.

The impact of not doing this is continued issues and breakages stemming from
the surprising behavior of `cargo install`.

# Prior art
[prior-art]: #prior-art

I checked, and Node JS respects lockfiles when installing binaries. It would be
helpful if those familiar with others could confirm whether or not other
popular programming language package managers respect lockfiles when installing
binaries.
