- Feature Name: Make unreachable patterns error, not warning
- Start Date: 2017-07-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

At now compiler accept such nonsense:

```rust
let x = 1;
match x {
  y => {}
  z => {}
  _ => {}
}
```

`rustc` generate warnings, not errors about the same patterns,
but this is 100% error of developer, so let's show this as error.

# Motivation
[motivation]: #motivation

Novice in `rustc` can try to use `match` with variables,
not literals, like this:

```rust
let x = calculate();
let y = 2;
let z = 3;
match x {
  y => {}
  z => {}
  _ => {}
}
```

this code compiles and run and sometimes warning about unused
variables (`y`, `z`) comes first before warning about unreachable pattern,
what makes unclear what is going on.

So to simplify for novice `match` usage, let's make unreachable pattern is error,
not warning.

Also not novice may made typo, for example miss `z if x == z` in some of "match arms",
so error clear show typo.


# How We Teach This
[how-we-teach-this]: #how-we-teach-this

I suppose that this behaviour expected from `match`.
Compiler/language is strict enough to not allow miss some branch,
so obviously expected that it not allow duplicate branches.

# Drawbacks
[drawbacks]: #drawbacks

Some buggy code will be not accepted by compiler.

