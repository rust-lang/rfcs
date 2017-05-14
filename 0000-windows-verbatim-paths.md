- Feature Name: windows_verbatim_paths
- Start Date: 2015-11-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Transparently use the long pathname API in Windows, allowing Rust programs to
deal with paths longer than 260 characters without having to implement their
own pathname handling.

# Motivation
[motivation]: #motivation

Windows has multiple ways to handle paths:

1. ANSI paths, as in `CreateFileA`, restricted to one code page with 256
   characters and paths of 260 characters: `r"C:\"`. Rust doesn't use these.
2. Traditional Unicode paths, as in `CreateFileW`, accepts a superset of
   UTF-16, as it allows for unmatched surrogates, maximum of 260 UTF-16 code
   units: `r"C:\"`. Rust currently uses these.
3. Extended Unicode paths, also accepted by `CreateFileW`, accepts the same
   superset of UTF-16 but allows for roughly 32768 code units.

Rust currently uses the second path style, unless the application explicitly
chooses to implement the path handling needed for 3. Since paths of the other
forms can be converted into the third form, Rust could implement this in the
standard library. Rust applications would consistently and robustly support
long path names.

# Detailed design
[design]: #detailed-design

The path translation will happen directly above the WinAPI layer, when the
`OsString`s are converted to arrays of 16-bit integers (UTF-16 code units).

- `C:\foo\bar` to `\\?\C:\foo\bar`.
- `\foo\bar` to the current working directory (CWD) joined with `\foo\bar`.
- `foo\bar` to CWD joined with `foo\bar`.
- `C:foo\bar`. This depends if on the current working directory. If it is on
  drive `C:`, then this results in the CWD joined with `foo\bar`, otherwise
  it'll result in `\\?\C:\foo\bar`.
- `\\server\share\` to `\\?\UNC\server\share\`.

During that conversion, a few modifications are applied:
1. Slashes must be replaced by backslashes.
2. `.` components are stripped out.
3. `..` components and their direct predecessors are removed. This
   transformation happens from left to right and doesn't remove any predecessor
   if the `..` component is the leftmost component.

Care must be taken when converting paths for use in symbolic linking functions,
as the file system does support relative symbolic links. Otherwise, symbolic
links on Windows behave in a way that does not clash with the behavior of `..`
above: If `C:\foo` is a symlink to `C:\bar\baz`, then `C:\foo\..` refers to
`C:\` and not to `C\bar`.

A working implementation can be found at
https://github.com/rust-lang/rust/pull/27916.

# Drawbacks
[drawbacks]: #drawbacks

One could argue that we want to expose the current operating system as closely
as possible, thus avoiding such path transformations in the standard library.
In this case however, this mostly results in exposing the limitations created
by the backward-compatiblity promises by Windows.

# Alternatives
[alternatives]: #alternatives

The impact of not doing this is that a lot of Rust application won't support
it. In fact, as of today not even the compiler or Cargo do.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
