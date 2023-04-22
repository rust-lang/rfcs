- Feature Name: `ignore_result`
- Start Date: 2023-04-22
- RFC PR: [rust-lang/rfcs#3423](https://github.com/rust-lang/rfcs/pull/3423)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC would add a function (maybe called `ignore_result`) that applies to `Option<T>`, `Result<T, E>` and other possible future types that have an `unwrap` method, that ignores their result when it is unneeded.

# Motivation
[motivation]: #motivation

Sometimes the output of a function is irrelevant. For example, a user could add an item to a `BTreeMap<T>` using [`insert`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html#method.insert), which returns an `Option<V>`. Maybe the user doesn't care if this `Option<V>` is `Some(_)` or `None`, and that's the case where this function could come in handy.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `ignore_result` method is a Quality-Of-Life method that applies to `Result<T, E>` and `Option<T>` . Similar to `.unwrap()` but that, instead of panicking in the failing case, it just continues. It returns `()` in both cases.

It's useful when you have a function with side effects, like [`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html)'s [`insert`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html#method.insert), during debug stages (where error reporting doesn't matter much).

The way to use this method could be something like:

```rust
use std::collections::BTreeMap;

fn main() {
  let mut foo = BTreeMap::new();

  // [...]
  // A lot of operations inserting a popping items from `foo`.

  foo.insert("maybe an already inserted key", "bar").ignore_result(); // We don't care about the result of this function, we only care about the key being there.
}
```

It's easier to read, as the alternative would be using a [`match`](https://doc.rust-lang.org/std/keyword.match.html) statement with both empty statements. Something like this:

```rust
match foo.insert("maybe an already inserted key", "bar") {
	Some(_) => {}
	None => {}
}

// Or, more compact but less used (it's more complex).

match foo.insert("maybe an already inserted key", "bar") {
	Some(_) | None => {}
} 
```

It's easier to understand that we don't care about the result by using a method explicitly named `ignore_result` than having an empty `match` statement, which may be seen as a bug or unimplemented logic.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

> Note: Currently, only `Option<T>` and `Result<T, E>` have `unwrap` methods, but this would apply to any type that has an `unwrap` method.

This would be a sister method to `unwrap` for both `Option` and `Result`, that returns `()`.
`ignore_result` would be an inline'd method, that would be just a match statement with both `Some` / `Ok` and `None` / `Err` returning `()`.

It wouldn't have any corner cases due to the simplicity of the function.

# Drawbacks
[drawbacks]: #drawbacks

* Maybe it encourages worse error reporting? This could also be said for `unwrap` and this method is intended for debug stages.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

It's clearer, using a `match` statement not only is unnecessarily verbose in these cases, but also may seem like there's unimplemented logic, when it's intended.

A possible alternative would be using `let _ = ...`, this silences alerts about not using `Result`(s) or `Options`(s), but it is unclear to newer Rustaceans, confusing in general and doesn't benefit from IDE auto-completion as much as a method would do.

Not implementing this method wouldn't be a big deal, but it would mean less convenience for Rust users.

Note that there are some posts [in the Forums](https://users.rust-lang.org/t/what-is-the-best-way-to-ignore-a-result/55187) and [StackOverflow](https://stackoverflow.com/questions/51141672/how-do-i-ignore-an-error-returned-from-a-rust-function-and-proceed-regardless) asking how to ignore a Result. This method would solve those issues.

# Prior art
[prior-art]: #prior-art

Nothing as far as I know. Only `unwrap`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should the name be `ignore_result`, or should the name be changed?

# Future possibilities
[future-possibilities]: #future-possibilities

*Nothing, this is such a small change that couldn't really change anything else in the Standard Library.*
