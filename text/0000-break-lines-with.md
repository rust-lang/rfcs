- Feature Name: break-lines-with
- Start Date: 2017-08-09
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This RFC allows the users to unify linebreaks to be used in a string through method `break_lines_with()`.

# Motivation
[motivation]: #motivation

Programs sometimes need to know about their working environment to do their job properly. Linebreak convention is one of the platform-related issues: It differs for Windows (CR LF) and \*NIX (LF). Such discrepancy can lead to problems easily, especially when a program needs to communicate with aged third-party libraries.

As a system programming language, it would be good for Rust to know the system it works on.

# Detailed design
[design]: #detailed-design

This RFC mainly introduces method `break_lines_with()` to primitive `str`. The method replaces all recognizable linebreaks with new linebreak specified, and it will not consume any linebreaks. Here is a simplified implementation of it:

Add a field `LINEBREAK` in `std::env::consts`:

```rust
/// Conventional linebreak of current platform.
#[cfg(windows)]
pub const LINEBREAK: &str = "\r\n";
#[cfg(not(windows))]
pub const LINEBREAK: &str = "\n";
```

An enum `Linebreak` representing different linebreaks:

```rust
enum Linebreak {
    // *NIX style:'\n' only.
    Lf,
    // Windows style: '\r\n'.
    CrLf,
    // Depend on target platform.
    Platform,
}
```

Implement `str` and `Deref<Target = str>` with:

```rust
fn break_lines_with(&self, lb: Linebreak) -> String {
    let lb = match lb {
        Linebreak::Lf => "\n",
        Linebreak::CrLf => "\r\n",
        Linebreak::Platform => std::env::consts::LINEBREAK,
    };
    let lines: Vec<&str> = self.lines().collect();
    let mut rv = lines.join(lb);
    if let Some(ch) = self.chars().last() {
        if ch == '\n' {
            rv += lb;
        }
    }
    rv
}
```

Further optimization could be done, while the behavior of `break_lines_with()` should be consistent with these code.

# Drawbacks
[drawbacks]: #drawbacks

Introduced more items into the standard library.

# Alternatives
[alternatives]: #alternatives

There are two alternatives for the suggested implementation. They are from different aspects and thus are independent to each other.

## Linebreak as Parameter

`break_lines_with()` can simply receive a `&str` linebreak as parameter. It allows the users to make their own choices of linebreaks. The user code will be like:

```rust
break_lines_with(text, "\r\n");
break_lines_with(text, "\u{2028}"); // Unicode line separator.
break_lines_with(text, std::env::consts::LINEBREAK); // Depend on target platform.
```

## Iterator

`break_lines_with()` can return a intermediate iterator rather than the resultant string. Lazy execution would help in situations where there is no need to process the entire string, or to specify types and allocate memory for results immediately.

# Unresolved questions
[unresolved]: #unresolved-questions

Should we support replacement of [unicode linebreaks](https://en.wikipedia.org/wiki/Newline#Unicode)? If the first alternative was taken, there will be no such problem.
