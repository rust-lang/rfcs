- Feature Name: cargogo
- Start Date: 2108-04-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes shortening the name of the `cargo` command for ergonomic benefit from 5 letters down to 2 letters long.

# Motivation
[motivation]: #motivation

Currently, the `cargo` command is very long to type: 5 whole letter! Compare this to other more ergonomic commands like `ls` or `rg`, and you can see that there are clearly gains to be made. For this reason, this RFC proposes shortening the command down to 2 letters from 5 letters.

Careful consideration is given to avoid breaking scripts and other tooling.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC proposes abbreviating `cargo` (5 letters) as `go` (2 letters). For example,

```bash
cargo build
# becomes
go build

cargo check
# becomes
go check

cargo install
# becomes
go install

cargo test
# becomes
go test
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A `go` binary is packaged and distributed via `rustup`, just as `cargo` is today. The binary is identical in functionality to `cargo`, but the name is less than half as short; in fact, `go` is a 60% improvement over `cargo`!

Howerver, since shell scripts and other existing infrastructure for many projects already use the name `cargo` and don't really benefit from the ergonomics boost, we also package and distribute `cargo` as it exists today. So in summary, both the new `go` and the existing `cargo` are distributed via `rustup` in a standard installation of Rust.

# Drawbacks
[drawbacks]: #drawbacks

None that I can think of.

# Rationale and Alternatives
[alternatives]: #alternatives

1. Obviously, we can stick with the status quo and all just suffer from RSI together... Maybe we can start an RSI working group?

2. We can have `rustup` set an `alias` in the shell. However, this would only work for shells that support aliases.

3. We can make `go` a symlink to `cargo` or vice versa. However, not all filesystems support symlinks.

# Unresolved questions
[unresolved]: #unresolved-questions

- Is two letters short enough? Perhaps a convenient one-letter command would be better? If we go down this path, my vote is for `w`.
