- Feature Name: `unwrap_or_panic`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

`Result` and `Option` both currently have a method `expect`.
This proposal renames those methods to `unwrap_or_panic`,
deprecating the old  name `expect`.
Also `Result::expect_err` becomes `Result::unwrap_err_or_panic`.

# Motivation
[motivation]: #motivation

The method name `expect` is bad:
it doesn’t obviously convey to newcomers what it does or that it panics,
and it encourages incorrect use of the string argument.

This change makes the standard library more consistent,
makes the feature of unwrap-with-a-custom-panic-message more discoverable,
and makes code that uses the method read more naturally.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

All documentation that uses `expect` will change it to `unwrap_or_panic`.
Associated explanations of the semantics of `expect` can be simplified,
since the name will now make it much more obvious.

Much the same happens with `Result::expect_err`,
becoming `Result::unwrap_err_or_panic`.

It may also be worth demonstrating the equivalent `unwrap_or_else` expressions,
which I think flow more naturally now that the names match:

```rust
option.unwrap_or_panic("option was none")
option.unwrap_or_else(|| panic!("option was none"))

result.unwrap_or_panic("result was err")
result.unwrap_or_else(|error| panic!("result was err: {:?}", error))

result.unwrap_err_or_panic("result was ok")
result.unwrap_err_or_else(|error| panic!("result was ok: {:?}", error))
```

(Well, *almost* equivalent. `unwrap_or_panic` and `unwrap_err_or_panic`
use `#[cold]` and `#[inline(never)]` on the panic function.)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation phases are:

1. Create new unstable methods `{Option, Result}::unwrap_or_panic`,
   identical in contents to the corresponding `expect` methods,
   and `Result::unwrap_err_or_panic`,
   identical in contents to `Result::expect_err`.

2. When stabilising `{Option, Result}::unwrap_or_panic`
   and `Result::unwrap_err_or_panic`,
   mark `{Option, Result}::expect` and `Result::expect_err` as deprecated.

No further explanation is required.

# Drawbacks
[drawbacks]: #drawbacks

1. It’s change. Existing users are used to `expect`.

2. The name `unwrap_or_panic` is a good deal longer than `expect`.

3. I can imagine that the existence of `unwrap_or_panic` could *potentially*
   mislead people briefly into thinking `unwrap` doesn’t panic.

4. `expect` is very widely used; when it is deprecated, users maintaining a
   minimum supported Rust version without stable `unwrap_or_panic` will want to
   keep using it, and this deprecation warning will be annoying to the point
   that there’s serious danger that the unfortunately-blunt instrument
   `#[allow(deprecated)]` will be abused (e.g. applied crate-wide).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

As noted and discussed in <https://github.com/rust-lang/rust/issues/35083>,
the name `expect` isn’t great on a couple of axes:

1. It doesn’t overtly signal by its name what it does.
   Experienced Rust developers will recognise it as a panicky method,
   and the type signature (`fn(Option<T>) -> T`) loosely implies it may panic,
   but the inexperienced will very probably be tripped up by it.

2. It guides you in the direction of writing the message back to front,
   expressing what you *expected* to be the case,
   whereas it is supposed to be an error message when the expectation *failed*,
   meaning what went *wrong*, which is often expressed as the exact opposite.

Consider these three varied spellings in capacity calculation:

- `cap.checked_mul(elem_size).expect("capacity overflow")`
  is easily misread as “expect capacity overflow”, which is quite wrong;
  it means “expect [a value, or panic complaining of] capacity overflow”.

- `cap.checked_mul(elem_size).unwrap_or_panic("capacity overflow")`
  is unambiguous, reading as
  “unwrap [the value,] or [else] panic [with message] ‘capacity overflow’”,
  leaning on the more popular and better understood term “unwrap”.

- `cap.checked_mul(elem_size).or_panic("capacity overflow")`
  is unambiguous, reading as
  “[use the value,] or [else] panic [with message] ‘capacity overflow’”.

When you see them side by side, I think `expect` is obviously inferior,
having an idiosyncratic meaning that makes it suboptimal for comprehension.

The first question is then whether it’s worth the bother of changing.
I think the answer is yes, because we’ve made such change easy,
and this makes the standard library more consistent and helps newcomers,
without meaningfully inconveniencing oldtimers.

## `unwrap_or_panic` versus `or_panic`

As noted, `unwrap_or_panic` is mildly unwieldy due to its length.
It’s tempting, therefore, to rename it to `or_panic`:
it’s shorter, and the word “panic” obviates the word “unwrap”.

However, I don’t think `or_panic` is the best name,
because it doesn’t match two established conventions:

- `or` and `or_else` aren’t unwrapping
  (`Option<T> -> Option<T>`, `Result<T, E> -> Result<T, F>`),
  so adding a method with the `or_` prefix that *does* unwrap is inconsistent.
  (I don’t think this would actually *mislead* anyone,
  but it’s the principle of consistency at stake here.
  Potentially a foolish consistency, as the Python folks might say.)

- Except for `expect` and `expect_err` (`Result`),
  *all* of the unwrapping methods start with `unwrap`:
  `unwrap`, `unwrap_unchecked`, `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`,
  `unwrap_err` and `unwrap_err_unchecked`.
  `expect` is an outlier, and it’d be a pity to replace one outlier with another.

Consequently, I’m inclined toward `unwrap_or_panic`.
But I think `or_panic` would still be better than `expect`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. The name: `unwrap_or_panic` or `or_panic`.

2. As noted in the drawbacks section, deprecating `expect` will encourage some
   to disable the *deprecated* lint altogether. We could therefore decide to
   delay formally deprecating `expect` until some time after `unwrap_or_panic`
   is stabilised, or until a better solution is found (e.g. strawman syntax
   `#![allow(deprecated(std::option::Option::expect))]`).

# Future possibilities
[future-possibilities]: #future-possibilities

1. The concept of a more fine-grained deprecation warning,
   to make deprecating `expect` more palatable.
   (Have we any experience deprecating anything anywhere near this popular?)

2. Back when [`Result` was given its `expect` method (to match `Option`) in
   2015](https://github.com/rust-lang/rfcs/pull/1119), the suggestion was made
   of changing its signature from taking `&str` to taking `impl fmt::Display`.
   That is a backwards-compatible action, but introducing a new method would
   make for a convenient time to do it, if we wanted to do it. [A follow-up
   proposal was made in 2017](https://github.com/rust-lang/rfcs/pull/1968), but
   was closed. This is probably not worth resurrecting now (most of the reasons
   for closing still hold), but I mention it as a point of historical interest
   at the least. And being able to use `format_args!(…)` *is* kinda nice.

3. Can this be made a fixable idiom?
