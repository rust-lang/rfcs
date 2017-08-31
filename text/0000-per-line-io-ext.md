- Feature Name: per-line-io-ext
- Start Date: 2017-08-09
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This RFC extends `std::io` to suit the need for uniform per-line reading/writing interface.

# Motivation
[motivation]: #motivation

Programs sometimes need to know about their working environment to do their job properly. Linebreak convention is one of the platform-related issues: It differs for Windows (CR LF) and \*NIX (LF). Such discrepancy can lead to problems easily, especially when a program needs to communicate with aged third-party libraries. So here comes the need for an interface of per-line writing.

Currently we can use the macro `writeln!()` to write lines separated automatically by `\n`. But who want to use their own char (or char sequence) as line terminators are not satisfied.

# Detailed design
[design]: #detailed-design

This RFC introduces a constant field and two traits.

Currently, in standard library, the following types are influenced:

-BufReader: Implements ReadLn
-BufRead: Derived from ReadLn
-BufWriter: Implements WriteLn
-StdIn: Implements ReadLn
-StdOut: Implements WriteLn

## Const `LINEBREAK`

```rust
// In `std::env::consts`
#[cfg(windows)]
pub const LINEBREAK: &str = "\r\n";
#[cfg(not(windows))]
pub const LINEBREAK: &str = "\n";
```

`LINEBREAK` allows users to refer to current platform linebreak through standard interface. For Windows, it is `CR LF`; for \*NIX platforms, it is `LF`.

## Trait `ReadLn`

```rust
// In `std::io`
pub trait ReadLn : Read {
    fn read_ln(&mut self, buf: &mut String) -> Result<usize>;
    fn lines(self) -> Lines<Self> where Self: Sized;
}
```

Implementing the trait indicates the reader can read per-line. Implementations themselves decide what char(s) or char sequence(s) should be recognized as raw linebreak. All types possessing per-line reading functionality should implement `ReadLn`. All `read_line()` and `lines()` should be deprecated.

Method `read_ln()` behaves similar to descried in [document of `std::io::BufRead::read_line()`](https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_line), except for that linebreaks are not copied to `buf` while included in returned value.

Method `lines()` turns the current reader into an iterator over lines, as described in [document of `std::io::BufRead::lines()`](https://doc.rust-lang.org/std/io/trait.BufRead.html#method.lines).

## Trait `WriteLn`

```rust
// In `std::io`
pub trait WriteLn : Write {
    fn linebreak(&self) -> &str;
    fn set_linebreak(&mut self, lb: &str) -> Result<()>;
    fn write_ln(&mut self, buf: &str) -> Result<()>;
    fn write_ln_fmt(&mut self, fmt: Arguments) -> Result<()>;
}
```

Implementing the trait indicates the writer can write per-line. Implementations themselves decide what char(s) or char sequence(s) should be recognized as raw linebreak. All types possessing per-line reading functionality should implement `WriteLn`.

Method `linebreak()` and `set_linebreak()` allow users to get or set current linebreak. If the linebreak to be used depends on its platform, `std::env::consts::LINEBREAK` can be passed in. `set_linebreak()` returns `Ok()` if the given linebreak is accepted and will be used by following calls to `write_ln()`.

Method `write_ln()` first concatenates its parameter `buf` with the linebreak set before (or a default one). The resultant string is then written to its destination as described in [document of `std::io::Write::write()`](https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write).

Method `write_ln_fmt()` works the same as described in [document of `std::io::Write::write_fmt()`](https://doc.rust-lang.org/std/io/trait.Write.html#method.write_fmt).

`write_ln()` is supposed to use `std::env::consts::LINEBREAK` as its default linebreak. But for different purposes, different linebreaks can be used. e.g. On writing HTTP headers, the default linebreak should always be `\r\n`, as defined in RFC2616.

# Drawbacks
[drawbacks]: #drawbacks

- Introduced more items into the standard library.
- Deprecating existing API since 1.0.0.
- Replacement for deprecated method (`read_ln()`) behaves differently.
- Name collision (`lines()`).

# Rationale and Alternatives
[alternatives]: #alternatives

**Why are `ReadLn` and `WriteLn` needed?**

Struct `OpenOption` possesses no method called `text()`. Trait `Read` and `Write` both contain methods to read or write as text or as binary. Thus, it is reasonable to say that,  text IO and binary IO are not separated by Rust standard library. These two traits just provided convenient ways to read and to write files. Rust needs another abstraction to iterpret the ability of per-line reading and writing.

**Why not call it `read_line()`?**

The name `read_ln` is given for compatibility. Method `read_line()` has been defined by two different things: `BufRead` and `StdIn`. `BufRead` is a trait and its `read_line()` borrows as mutable, while `StdIn` is a struct and borrows.

**Why does `read_ln()` have different behavior from `read_line`?**

First, `read_ln()` will not copy the linebreak chars. `lines()` omits linebreaks, `write_ln()` does not need `buf` to contain linebreaks neither. So `read_ln()` should not copy linebreaks.

Second, the value returned is nolonger the number of bytes read. Since per-line reading is entirely based on text, the number of bytes read can be ignored.

**Why are past proposals disapproved?**

_Only Const `LINEBREAK`_

Disapproved for inconvenience. It prevents users from writing concise codes as they have to refer to the constant over and over.

_Method `break_lines_with`_

Implement `str` and `Deref<Target = str>` with:

```rust
enum Linebreak {
    Lf,
    CrLf,
    Platform,
}

fn break_lines_with(&self, lb: &Linebreak) -> String {
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

Disapproved for poor practicality. Texts in a program are usually stored in fragments and are concatenated together when needed. Most of its practical use can be replaced by the following codes based on the current proposal:

```rust
io::stdout().write(text.break_lines_with(Linebreak::Lf).as_bytes());

// equals to

for line in text.lines() {
    io::stdout().write_ln(line);
}
```

# Unresolved questions
[unresolved]: #unresolved-questions

- How to resolve the name collision between new and old `lines()`?
- Should `ReadLn` and `WriteLn` allow users to specify what char(s) or char sequence(s) are recognized as raw linebreaks?
- Should `writeln!()` be deprecated?
