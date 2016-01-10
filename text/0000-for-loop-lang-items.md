- Feature Name: for_loop_lang_items
- Start Date: 2016-01-10
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Rewrite the `for` loop desugaring to use language items instead of hardcoded paths.

# Motivation
[motivation]: #motivation

As noted in issue [#30803], pull request [#20790] changed the `for` loop desugaring to use the
`IntoIterator` trait, removing the `iterator` language item. As a result, `for` loops are the only
syntax that involves desugaring based on hardcoded paths to items instead of language items. In
particular, the desugaring references `core::iter::IntoIterator`, `core::iter::Iterator`, and
`core::option::Option`.

[#20790]: https://github.com/rust-lang/rust/pull/20790
[#30803]: https://github.com/rust-lang/rust/issues/30803

This makes them inconsistent, and results in poor error messages for code using `#![no_core]`:

```rust
#![feature(lang_items, no_core)]
#![no_core]

#[lang = "sized"]
trait Sized {}

fn main() {
    for _ in () {}
}
```

Compilation output:

```
foo.rs:8:5: 8:9 error: failed to resolve. Maybe a missing `extern crate iter`? [E0433]
foo.rs:8     for _ in () {}
                          ^~~~
foo.rs:8:5: 8:9 help: run `rustc --explain E0433` to see a detailed explanation
foo.rs:8:5: 8:19 error: unresolved name `iter::IntoIterator::into_iter` [E0425]
foo.rs:8     for _ in () {}
                          ^~~~~~~~~~~~~~
foo.rs:8:5: 8:19 help: run `rustc --explain E0425` to see a detailed explanation
foo.rs:8:5: 8:9 error: failed to resolve. Maybe a missing `extern crate iter`? [E0433]
foo.rs:8     for _ in () {}
                          ^~~~
foo.rs:8:5: 8:9 help: run `rustc --explain E0433` to see a detailed explanation
foo.rs:8:5: 8:19 error: unresolved name `iter::Iterator::next` [E0425]
foo.rs:8     for _ in () {}
                          ^~~~~~~~~~~~~~
foo.rs:8:5: 8:19 help: run `rustc --explain E0425` to see a detailed explanation
foo.rs:8:5: 8:19 error: unresolved enum variant, struct or const `Some` [E0419]
foo.rs:8     for _ in () {}
                          ^~~~~~~~~~~~~~
foo.rs:8:5: 8:19 help: run `rustc --explain E0419` to see a detailed explanation
foo.rs:8:5: 8:19 error: unresolved enum variant, struct or const `None` [E0419]
foo.rs:8     for _ in () {}
                          ^~~~~~~~~~~~~~
foo.rs:8:5: 8:19 help: run `rustc --explain E0419` to see a detailed explanation
```

# Detailed design
[design]: #detailed-design

1. Add an `into_iterator` language item and provide it with `core::iter::IntoIterator`.
2. Restore the `iterator` language item and provide it with `core::iter::Iterator`.
3. Add an `option` language item and provide it with `core::option::Option`.
4. Rewrite the `for` loop desugaring to use the new language items.

# Drawbacks
[drawbacks]: #drawbacks

Possible backward compatibility concerns with existing `#![no_core]` code.

# Alternatives
[alternatives]: #alternatives

Retain the use of hardcoded paths.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
