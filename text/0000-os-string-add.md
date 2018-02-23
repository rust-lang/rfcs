- Feature Name: `os_string_add`
- Start Date: 2017-06-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Implement `Add` for `OsString` analogous to `Add` for `String`.

# Motivation
[motivation]: #motivation

Make concatenating `OsString`s (e.g. in paths and/or command-line arguments) a tiny bit more convenient, which in turn makes writing small command-line utilities easier.

# Detailed design
[design]: #detailed-design

Basically this:

```rust
impl<'a> Add<&'a OsStr> for OsString {
    type Output = OsString;
    fn add(mut self, other: &'a OsStr) -> Self::Output {
        self.push(other);
        self
    }
}
```

It consumes the LHS and is a thin wrapper over `.push` written in a more functional style.  The main advantage of this over `push` is that `push` necessitates having a mutable (in/out) variable and is hard to nest.  A typical usage of `push` would require at least 2 extra lines: one for the variable definition, and one for the `push`, neither of which are necessary for `+`.

Example of its use:

```rust
fn process_file<P: AsRef<Path>>(path: P) { /* ... */ }

// from command-line parser (e.g. clap)
let prefix: &OsStr = /* ... */;
let suffix: &OsStr = /* ... */;

process_file(prefix.to_owned() + "-header-".as_ref() + suffix);
process_file(prefix.to_owned() + "-body-".as_ref() + suffix);
process_file(prefix.to_owned() + "-footer-".as_ref() + suffix);
```

Compare with how it would be written today:

```rust
fn process_file<P: AsRef<Path>>(path: P) { /* ... */ }

// from command-line parser (e.g. clap)
let prefix: &OsStr = /* ... */;
let suffix: &OsStr = /* ... */;

let mut path = prefix.to_owned();
path.push("-header-");
path.push(suffix);
process_file(path);
let mut path = prefix.to_owned();
path.push("-body-");
path.push(suffix);
process_file(path);
let mut path = prefix.to_owned();
path.push("-footer-");
path.push(suffix);
process_file(path);
```

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

- Examples of its use can be added to the documentation of `Add` for `OsString` analogous to `Add` for `String`.
- The syntactic trade-offs with `push` can be elaborated upon.

# Drawbacks
[drawbacks]: #drawbacks

- Slightly more bloat to the std API surface
- There seems to be some controversy over the existing `String + &str` API

# Alternatives
[alternatives]: #alternatives

- There are more general implementations such as:
  `impl<T: AsRef<OsStr>> Add<T> for OsString`.
- Since `.push` already exists, there is no other sensible way to define `Add` that wouldn't break user expectations.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
