- Start Date: 2020-10-25
- RFC PR: [rust-lang/rfcs#3007](https://github.com/rust-lang/rfcs/pull/3007)
- Rust Issue: [#80162](https://github.com/rust-lang/rust/issues/80162)

# Summary

This RFC proposes to make `core::panic!` and `std::panic!` identical and consistent in Rust 2021,
and proposes a way to deal with the differences in earlier editions without breaking code.

# Problems

`core::panic!` and `std::panic!` behave mostly the same, but have their own incompatible quirks for the single-argument case.

This leads to several different problems, which would all be solved if they didn't special-case `panic!(one_argument)`.

For multiple-arguments (e.g. `panic!("error: {}", e)`), both already behave identical.

## Panic

Both do not use `format_args!("..")` for `panic!("..")` like they do for multiple arguments, but use the string literally.

*💔 **Problem 1:** `panic!("error: {}")` is probably a mistake, but compiles fine.*

*💔 **Problem 2:** `panic!("Here's a brace: {{")` outputs two braces (`{{`), not one (`{`).*

In the case of `std::panic!(x)`, `x` does not have to be a string literal, but can be of any (`Any + Send`) type.
This means that `std::panic!("{}")` and even `std::panic!(&"hi")` compile without errors or warnings, even though these are most likely mistakes.

*💔 **Problem 3:** `panic!(123)`, `panic!(&"..")`, `panic!(b"..")`, etc. are probably mistakes, but compile fine with `std`.*

In the case of `core::panic!(x)`, `x` must be a `&str`, but does not have to be a string literal, nor does it have to be `'static`.
This means that `core::panic!("{}")` and `core::panic!(string.as_str())` compile fine.

*💔 **Problem 4:** `let error = String::from("error"); panic!(&error);` works fine in `no_std` code, but no longer compiles when switching `no_std` off.*

*💔 **Problem 5:** `panic!(CustomError::Error);` works with std, but no longer compiles when switching `no_std` on.*

## Assert

`assert!(expr, args..)` and `assert_debug(expr, args..)` expand to `panic!(args..)` and therefore will have all the same problems.
In addition, these can result in confusing mistakes:

```rust
assert!(v.is_empty(), false); // runs panic!(false) if v is not empty  😕
```

*💔 **Problem 6:** `assert!(expr, expr)` should probably have been a `assert_eq!`, but compiles fine and gives no useful panic message.*

Because `core::panic!` and `std::panic!` are different, `assert!` and related macros expand to `panic!(..)`, not to `$crate::panic!(..)`,
making these macros not work with `#![no_implicit_prelude]`, as reported in [#78333](https://github.com/rust-lang/rust/issues/78333).
This also means that the panic of an assert can be accidentally 'hijacked' by a locally defined `panic!` macro.

*💔 **Problem 7:** `assert!` and related macros need to choose between `core::panic!` and `std::panic!`, and can't use `$crate::panic!` for proper hygiene.*

## Implicit formatting arguments

[RFC 2795] adds implicit formatting args, as follows:

```rust
let a = 4;
println!("a is {a}");
```

It modifies `format_args!()` to automatically capture variables that are named in a formatting placeholder.

With the current implementations of `panic!()` (both core's and std's), this would not work if there are no additional explicit arguments:

```rust
let a = 4;

println!("{}", a); // prints `4`
panic!("{}", a); // panics with `4`

println!("{a}"); // prints `4`
panic!("{a}"); // panics with `{a}`  😕

println!("{a} {}", 4); // prints `4 4`
panic!("{a} {}", 4); // panics with `4 4`
```

*💔 **Problem 8:** `panic!("error: {error}")` will silently not work as expected, after [RFC 2795] is implemented.*

## Bloat

`core::panic!("hello {")` produces the same `fmt::Arguments` as `format_args!("hello {{")`, not `format_args!("{}", "hello {")` to avoid pulling in string's `Display` code,
which can be quite big.

However, `core::panic!(non_static_str)` does need to expand to `format_args!("{}", non_static_str)`, because `fmt::Arguments` requires a `'static` lifetime
for the non-formatted pieces. Because the `panic!` `macro_rules` macro can't distinguish between non-`'static` and `'static` values,
this optimization is only applied to what macro_rules consider a `$_:literal`, which does not include `concat!(..)` or `CONST_STR`.

*💔 **Problem 9:** `const CONST_STR: &'static str = "hi"; core::panic!(CONST_STR)` works,
but will silently result in a lot more generated code than `core::panic!("hi")`.
(And also needs [special handling](https://github.com/rust-lang/rust/pull/78069) to make `const_panic` work.)*

# Solution if we could go back in time

None of these these problems would have existed if
1\) `panic!()` did not handle the single-argument case differently, and
2\) `std::panic!` was no different than `core::panic!`:

```rust
// core
macro_rules! panic {
    () => (
        $crate::panic!("explicit panic")
    );
    ($($t:tt)*) => (
        $crate::panicking::panic_fmt($crate::format_args!($($t)+))
    );
}

// std
use core::panic;
```

The examples from problems 1, 2, 3, 4, 5, 6 and 9 would simply not compile, and problems 7 and 8 would not occur.

However, that would break too much existing code.

# Proposed solution

Considering we should not break existing code, I propose we gate the breaking changes on the 2021 edition.

In addition, we add a lint that *warns* about the problems in Rust 2015/2018, while not giving errors or changing the behaviour.

Specifically:

- Only for Rust 2021, we apply the breaking changes as in the previous section.
  So, `core::panic!` and `std::panic!` are the same, and *always* put their arguments through `format_args!()`.

  Any optimization that needs special casing should be done *after* `format_args!()`.
  (E.g. using [`fmt::Arguments::as_str()`](https://github.com/rust-lang/rust/pull/74056),
  as is [already done](https://github.com/rust-lang/rust/pull/78119) for `core::panic!("literal")`.)

  This means `std::panic!(x)` can no longer be used to panic with arbitrary (`Any + Send`) payloads.

- We [add `std::panic::panic_any(x)`](https://github.com/rust-lang/rust/pull/74622),
  that still allows programs with std to panic with arbitrary (`Any + Send`) payloads.

- We [add a lint](https://github.com/rust-lang/rust/pull/78088) for Rust 2015/2018 that warns about problem 1, 2, and 8,
  similar to [what Clippy already has](https://rust-lang.github.io/rust-clippy/master/index.html#panic_params).

  Note that this lint isn't just to warn about incompatibilities with Rust 2021, but also to warn about usages of `panic!()` that are likely mistakes.

  This lint suggests add an argument to `panic!("hello: {}")`, or to insert `"{}", ` to use it literally: `panic!("{}", "hello: {}")`.
  ([Screenshot here.](https://user-images.githubusercontent.com/783247/96643867-79eb1080-1328-11eb-8d4e-a5586837c70a.png))
  The second suggestion can be a pessimization for code size, but I believe that [can be solved separately](https://github.com/rust-lang/rust/issues/78356).

- After `panic_any` is stable, we add a lint for Rust 2015/2018 (or extend the one above) to warn about problem 3, 4, 5 and 9.
  It warns about `panic!(x)` for anything other than a string literal, and suggests to use
 `panic_any(x)` instead of `std::panic!(x)`, and
 `panic!("{}", x)` instead of `core::panic!(x)`.

  It will also detect problem 6 (e.g. `assert!(true, false)`) because that expands to such a panic invocation,
  but will suggest `assert_eq!()` for this case instead.

- We [modify the panic glue between core and std](https://github.com/rust-lang/rust/pull/78119)
  to use `Arguments::as_str()` to make sure both `std::panic!("literal")` and `core::panic!("literal")`
  result in a `&'static str` payload. This removes one of the differences between the two macros in Rust 2015/2018.

  This is already merged.

- Now that `std::panic!("literal")` and `core::panic!("literal")` behave identically,
  [we modify `todo!()`, `unimplemented!()`, `assert_eq!()`, etc.](https://github.com/rust-lang/rust/pull/78343)
  to use `$crate::panic!()` instead of `panic!()`. This solves problem 7 for all macros except `assert!()`.

- We modify `assert!()` to use `$crate::panic!()` instead of `panic!()` for the single argument case in Rust 2015/2018,
  and for all cases in Rust 2021.

  This solves problem 7 for the common case of `assert!(expr)` in Rust 2015/2018, and for all cases of `assert!` in Rust 2021.

Together, these actions address all problems, without breaking any existing code.

# Drawbacks

- This results in subtle differences between Rust editions.

- This requires `assert!` and `panic!` to behave differently depending on the Rust edition of the crate it is used in.
  `panic!` is just a `macro_rules` macro right now, which does not natively support that.

# Alternatives

- Instead of the last step, we could also simply break `assert!(expr, non_string_literal)` in all editions.
  This usage is probably way less common than `panic!(non_string_literal)`.

[RFC 2795]: https://rust-lang.github.io/rfcs/2795-format-args-implicit-identifiers.html
