- Feature Name: Add a count parameter to the expect attribute
- Start Date: 2023-03-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a new paramter to [the `#[expect]`
attribute](https://github.com/rust-lang/rust/issues/54503) named `count` that asserts a
precise count for the number of lints suppressed by the attribute. If the number
of lints to suppress is given, then it will raise a warning unless that number
is exactly matched.

# Motivation
[motivation]: #motivation

A major use-case of the `#[expect]` attribute is temporarily adding an exception
to a lint, to be resolved later. By using `#[expect]` instead of `#[allow]`, you
will immediately know when it is safe to remove the exception to the lint.

However, adding a lint exception to any level of scope can often, in my
experience, lead to additional exceptions that weren't originally intended
accidentally slipping through, resulting in a process that, instead of gradually
phasing out lint exceptions, causes more and more offending code to be added
without anyone realizing. Especially with an exception on a wide scope, the
author of the code may not even know the exception exists.

By setting an explicit number of allowances, anything which produces another
instance of the lint will cause the lint to still trigger, making it clear to
the programmer that they introduced another instance of the lint and allowing
them to decide whether the original exception should extend to it as well, or
whether they should fix the instance immediately.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

(to be appended to the guide-level explanation on the [`#[expect]` lint
attribute](https://github.com/rust-lang/rfcs/blob/master/text/2383-lint-reasons.md#expect-lint-attribute)

Additionally, you may specify an expected amount of lint instances to suppress,
in which case, it will suppress the lints only if there is exactly the expected
number, and in all other cases it will log an additional warning (in addition to
any lints inside the scope).

This is useful when gradually applying a new lint to a codebase, to allow an
exception for existing instances of the lint, without allowing any new
exceptions to accidentally slip through.
```rust
#[expect(unused_mut, count = 1)]
fn foo() -> usize {
    let mut a = Vec::new();
    a.len()
}
```
will remain quiet and not produce any lints. However, if you add another
instance of `unused_mut`, then it will fire and warn you.
```rust
#[expect(unused_mut, count = 1)]
fn foo() -> usize {
    let mut a = Vec::new();
    let mut b = Vec::new();
    a.len() + b.len()
}
```
will emit the `unused_mut` lints for both `a` and `b` at a `warn` level, and
will also emit:
```
warning: expected lint `unused_mut` did not appear the expected number of times
 --> src/lib.rs:1:1
  |
1 | #[expect(unused_mut, count = 1)]
  |   ---------------------------^-
  |   |
  |   help: replace `count` from `1` to `2`
  |
  = note: #[warn(expectation_missing)] on by default
```

As previously, removing all instances of the lint will produce the same error
message, so:
```rust
#[expect(unused_mut, count = 1)]
fn foo() -> usize {
    let a = Vec::new();
    a.len()
}
```
will emit:
```
warning: expected lint `unused_mut` did not appear
 --> src/lib.rs:1:1
  |
1 | #[expect(unused_mut, count = 1)]
  |   ---------------------------^-
  |   |
  |   help: remove this `#[expect(...)]`
  |
  = note: #[warn(expectation_missing)] on by default
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This would reuse the existing `#[expect]` lint level, but, when a `count` is
provided, it will act like a `#[warn]` instead of an `#[allow]` when the number
of instances of the lint covered don't match. Additionally, when the count
mismatches, it will emit the same lint as if a normal `#[expect]` attribute had
not matched any lints (open to change).

The `count` parameter is expected to be strictly positive, and it will fail to
compile if a `count` which is either zero or negative is given.

# Drawbacks
[drawbacks]: #drawbacks

The only drawback to adding this feature is that doing so would increase the
scope of the rust compiler to support it, and potentially extra work of figuring
out how future features interact with this one.

Nothing would be forced upon codebases, so there is no threat of increasing the
complexity of learning the language to newcomers.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Add to `#[allow]` instead of `#[expect]`

This could be made an option on the `#[allow]` attribute and entirely supplant the
need for `#[expect]`. However, I think it makes more sense to include this on
the `#[expect]` if such an attribute exists, and removing the `#[expect]`
attribtue for just allowing a specific count would remove the case where you
don't care how many violations exist.

## Other ways of handling the count

The `count` as proposed must match exactly the number of instances of the lint
triggered inside. We could allow constructs like `count <= #`, either in
addition to or instead of the currently-proposed exact match.

In my view, this is worse for the case of gradually removing lints from an
existing project. If an `expect` has 10 instances of the lint, I might slap an
`#[expect(..., count <= 10)]` on it. Then, say I do some cleanup and reduce it
down to 5 instances. Nothing prompts me to change the count, so I don't. Later,
having forgotten about this, I do something else that adds back in up to 5
instances of the lint, and the "slack" from the previous 5 being removed cause
me to not get any warnings about the new instances.

## Other ways of handling the inner lint instances

If the `count` doesn't match, then we could continue to suppress the lints
inside, but output one mega-lint which tells the user about all of the instances
inside. However, I think that would be worse from a UX perspective.

We could also track the level that was specified for this lint outside of the
`#[expect]` annotation and reapply that level. However, this seems like extra
work for marginal benefit, and it would lead to imo worse UX if the outside
scope was an `#[allow]`, as it would render the lint locations effectively
invisible.

We could also require the `count` attribute be paired with a `level` attribute
to indicate the level to pass through lints on a mismatched count, but in my
opinion that adds too much complexity and makes it needlessly difficult to use.

## Do Nothing

We could do nothing, which would miss out on the benefits I outlined in the
motivation section.

# Prior art
[prior-art]: #prior-art

## Syntax in Rust

Other attributes take such named parameters, such as the `stable` and `unstable`
attributes, and the `reason` parameter proposed for all lint levels in the same
RFC as the `#[expect]` lint level.

## Behavior

I am unaware of any languages which have a similar functionality to what I am
describing here.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Can we allow `#[expect(..., count = 0)]` with some useful behavior? What about
  negative `count`s?
