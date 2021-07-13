- Feature Name: `add-filename-info-to-io-error`
- Start Date: 2021-02-11
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- [Prior discussion](https://internals.rust-lang.org/t/add-filename-information-to-io-error/)
- [Rust issue requesting for this change](https://github.com/rust-lang/rust/issues/44938)

# Summary

[summary]: #summary

This RFC proposes to add filename information to `std::io:Error`. By doing so, `std::io` error messages will include which file the user is trying to access and making it easier for the user to notice where their code is failing.

# Motivation

[motivation]: #motivation

Consider a simple program which reads a file which doesn't exist:

```rust
use std::fs::File;

fn main() -> std::io::Result<()> {
    let file = File::open("hello.txt")?;
    println!("file: {:?}", file);
    Ok(())
}
```

This program emits the following error message:

```txt
Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

This error message doesn't tell the user which file doesn't exist and doesn't give the user a full breakdown of the error and what went wrong. As I've experienced and many others have, this error doesn't help a beginner Rust programmer to understand what went wrong and how to fix it. If the compiler emitted a full error and more information such as the name of the file which doesn't exist, this would be much more helpful.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Suppose a user creates the same program as before:

```rust
use std::fs::File;

fn main() -> std::io::Result<()> {
    let file = File::open("hello.txt")?;
    println!("file: {:?}", file);
    Ok(())
}
```

With this change, a much more descriptive and helpful error message will be emitted:

```txt
Error: Os { code: 2, kind: NotFound, message: "No such file or directory", file: "/Users/henryboisdequin/hello.txt" }
```

This error explains where the error is emitted and the cause of it. As you can see, this error is much more helpful. It points to where the problem is and the cause of it. Let's look at another case:

```rust
use std::fs::File;

fn main() -> std::io::Result<()> {
    let _ = File::create("/etc/protocols")?;

    Ok(())
}
```

This program returns the following error:

```txt
Error: Os { code: 13, kind: PermissionDenied, message: "Permission denied" }
```

This program is not helpful at all, not displaying file information or how to fix this error. With this change, here is the projected error message:

```
Error: Os { code: 13, kind: PermissionDenied, message: "Permission denied", file: "/etc/protocols" }
```

In this case, this change would add the file to the `Os` error message. Many crates and Rust applications write their own error messages to improve the experience while working with `std::io`. This is why we should incorporate filename information to `std::io:Error` to reduce the amount of boilerplate crates and Rust applications need to write to ensure a better `std::io` development process.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

1\.

- Add field `file: Option<PathBuf>` to struct `std::io::Error`
  - Whenever a file doesn't exist or another error occurs when writing/reading/accessing a file, add the filename to the `file` field

2\.

- Test impact of performance by adding benchmarks and looking at `std`'s and `std::io`'s general difference in performance before and after this implementation

# Drawbacks

[drawbacks]: #drawbacks

- `Option<PathBuf>` would make `std::io:Error` heavier than before [(originally pointed out by @withoutboats)](https://internals.rust-lang.org/t/add-filename-information-to-io-error/5120/5)
  - In some cases, this would also make `std::io:Error` inefficient as operations which expect a large number of failures would have to allocate to the error itself and the `PathBuf` in the `file` field [(originally pointed out by @alexcrichton)](https://internals.rust-lang.org/t/add-filename-information-to-io-error/5120/7)

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

1. Of course, an alternative to this RFC would to leave OS errors as they are. We should implement this change because it makes is much more helpful to the user with a more useful diagnostic. For example, if a certain crate that a user is using is trying to access an arbitrary file which doesn't exist, currently an error message such as this would be emptied:

```
Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

This doesn't help the user whatsoever. If they were also trying to access files from their own code, they wouldn't know where the error is coming from as their is no filename or information on how to resolve this problem.

2. (@nagisa's idea): To have callers to attach the filename and the operation done to the error by adding context to it (similarly to how the community handles `std::io` errors with error handling crates such as `anyhow`).

# Prior art

[prior-art]: #prior-art

- [Python's `OSError`](https://docs.python.org/3/library/exceptions.html#OSError) exception contains filename information (`filename` and `filename2`) which is later used when emitting an `OSError`

- This change was [previously implemented](https://github.com/rust-lang/rust/pull/14629) in 2014, but was removed during the [`std::io` reform](https://github.com/rust-lang/rfcs/blob/master/text/0517-io-os-reform.md) due to various different reasons

- [Node's `EACCES` Error](https://man7.org/linux/man-pages/man3/errno.3.html) also contains filename information which include the path of the file

- [fs-err](https://crates.io/crates/fs-err) is a replacement for `std::fs` which keeps adds the filename to the diagnostic
