- Feature Name: `process_set_process_group`
- Start Date: 2022-02-02
- RFC PR: [rust-lang/rfcs#3228](https://github.com/rust-lang/rfcs/pull/3228)
- Rust Issue: [rust-lang/rust#93857](https://github.com/rust-lang/rust/issues/93857)

# Summary
[summary]: #summary

Add a `process_group` method to `std::os::unix::process::CommandExt` that
allows setting the process group id (i.e. calling `setpgid`) in the child, thus
enabling users to set process groups while leveraging the `posix_spawn` fast
path.

# Motivation
[motivation]: #motivation

The Unix process spawn code has two paths: a fast path that uses `posix_spawn`,
and a slow path that uses `fork` and `exec`.

The performance between the two APIs has been shown to be very noticeable:

https://github.com/rust-lang/rust/commit/8fe61546696b626ecf68ef838d5d82e393719e80

Currently, users can set the process group on the commands they spawn via:

```
let pre_exec = || nix::unistd::setpgid( ... );
unsafe {
    cmd.pre_exec(pre_exec)
};
```

This approach forces the slow path because of the usage of `pre_exec`.

However, `posix_spawn` supports setting the process group
(`posix_spawnattr_setpgroup`). This RFC proposes exposing that functionality,
which allows users to set the process group id without forcing the slow path.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`std::os::unix::process::CommandExt::process_group` allows you to set the
process group ID of the child process. This translates to a `setpgid` call
in the child.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The changes needed are:

- Expose a `CommandExt` a `process_group` method on `CommandExt` that takes a
  PID as argument.
- Add a call to `posix_spawnattr_setpgroup` on the fast path.
- Add a call to `setpgid` on the slow path.

# Drawbacks
[drawbacks]: #drawbacks

This marginally expands the API surface on `CommandExt`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Using `pre_exec` this is a viable alternative for programs where `fork` is
  either sufficiently fast or infrequent.
- Not using `std::process`, and rolling your own instead, is an alternative.
  This would however break interoperability with e.g. Tokio's
  `tokio::process::Command`, which currently can be created using a
  `Command` from the std lib.

# Prior art
[prior-art]: #prior-art

The primary prior art here is all the other calls that already exist on
`CommandExt` that translate to parameterizing `posix_spawn`, such as
configuring groups, signal mask, current working directory, open pipes.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- None known at this point.

# Future possibilities
[future-possibilities]: #future-possibilities

There are a few other `posix_spawn` options that are not supported, such as
`setsid` (which is a GNU extension). Those might warrant inclusion as well.
