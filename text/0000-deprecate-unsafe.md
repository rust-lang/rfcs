- Feature Name: deprecate_unsafe
- Start Date: 2018-04-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Currently Rust allows using `unsafe`. This RFC proposes to deprecate
`unsafe` keyword.

# Motivation
[motivation]: #motivation

`unsafe` keyword is the first cause of segfaults in Rust, compiler bugs
being second. Removing this feature would make Rust significantly safer.
Additionally, writing `unsafe` code is tricky, and there are plenty of
[traps to aware of](https://doc.rust-lang.org/nomicon/).

This will make Rust much easier to learn, as it will remove a big part
of it which is well known for being tricky. Rust is already known to
be hard to learn, this should improve the situation.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`unsafe` keyword is deprecated in Rust epoch 2018 and removed in Rust epoch 2020.

The following code won't work. This is already implemented in Rust compiler.

```rust
unsafe fn forty_two() {
    42
}
```

This code will stay working as it's completely safe and doesn't
cause undefined behaviour.

```rust
#![no_main]

#[link_section=".text"]
#[no_mangle]
pub static main: [u32; 9] = [
    3237986353,
    3355442993,
    120950088,
    822083584,
    252621522,
    1699267333,
    745499756,
    1919899424,
    169960556,
];
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When `unsafe` keyword is used in epoch 2018, generate a depreciation warning.
In Epoch 2020, reject compilation. `unsafe` word stays reserved to avoid people
creating `unsafe!` and `#[unsafe]` macros to replace it.

# Drawbacks
[drawbacks]: #drawbacks

This makes it harder to interact with C code. You probably shouldn't need to,
but if needed it's possible to use a previous epoch of Rust. Better yet,
rewrite this code in Rust. I don't want to deal with yet another segfault
from OpenSSL or whatever you use.

Code using `unsafe` will break, but it's already broken as it's impossible
to use `unsafe` correctly. This will require some way for compiler to use
`unsafe`, as low level concepts like memory allocation will require it to
be available. Although I suppose it's possible to use stack allocation as
a replacement for heap allocation. This would require changing every
single API in Rust language however.

# Rationale and alternatives
[alternatives]: #alternatives

- Making `unsafe` not cause segfaults would require Rust to run in a virtual
  machine which panics when an undefined behaviour is triggered. Because
  Rust can interact with C which can cause undefined behavior, this is
  considered to be impractical.

- Not removing `unsafe` is possible, but won't lower amount of segfaults
  in Rust programs.

- Using a longer keyword like `yes i know this is unsafe but please let me do it`
  would help as it would discourage using `unsafe` but wouldn't solve the
  issue.

- We can default `unsafe` for the entire language. C++ did that and
  is heavily successful due to that decision.

- Make next version of Rust actually be Java. This will break most of code,
  but this will actually improve the compatibility with programs that did not
  work before, as in, it fixes more code than it breaks.

# Prior art
[prior-art]: #prior-art

- Many programming languages don't provide an `unsafe` facility.

- Even if they do, it's very very very very rarely used, unlike in Rust.

- [`cargo-osha`](https://crates.io/crates/cargo-osha)

# How we teach this?

We don't. Lack of features is truly the simplest feature to learn.

# Unresolved questions
[unresolved]: #unresolved-questions

- Should already stable libraries having `unsafe` APIs continue
  to work after this change?

- Should raw pointers be removed considering they are mostly useless
  without `unsafe`.o̸̦͓̱͖͓̖͎ơp̦̬̩s͘ i̞͉̼͓͘ͅ ̞͍͚̟̜͓f̕o͔͠r̲g͉̼ͅo͏̳̜̘̰͕̹t̻͖̘͞ 0̶̫ ̘̖̟̯̪̗͉s̶̥̦̝͍̖t͉͉͎̪͡r̵̲͇̲̰͔i̫̕n̖͉͖̹̭̫̪g̯̰͓͔̣͖ ̤͍̩̥̠̬t̨̗̟̯͖e͉̜̝͓r̷͔̣̮̙̘ḿ͇̭̳i̢͙͓̥̘̭͚̭n̢͔̻̥̗͚̳a̸t̷̟͕̠̗o̘ŕ

