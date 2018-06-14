- Feature Name: clippy_uno
- Start Date: 2018-06-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Release Clippy 1.0, in preparation for it being shipped via rustup and eventually available via Rust Stable.

# Motivation
[motivation]: #motivation

See also: [The Future of Clippy][future]

Clippy, the linter for Rust, has been a nightly-only plugin to Rust for many years.
In that time, it's grown big, but it's nightly-only nature makes it pretty hard to use.

The eventual plan is to integrate it in Rustup á la Rustfmt/RLS so that you can simply fetch prebuilt binaries
for your system and `cargo clippy` Just Works ™️. In preparation for this, we'd like to nail down various things
about its lints and their categorization.

[future]: https://manishearth.github.io/blog/2018/06/05/the-future-of-clippy-the-rust-linter/

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Usage and lint philosophy

We expect clippy to be used via `cargo clippy`.

Clippy aims to follow the general Rust style. It may be somewhat opiniated in some situations.

In general clippy is intended to be used with a liberal sprinkling of `#[allow()]` and `#[warn()]`; _it is okay_ to
disagree with Clippy's choices. This is a weaker philosophy than that behind rustc's lints, where usually flipping
one is an indication of a very specialized situation.

## Lint attributes

Currently to allow/deny clippy lints you have to `#[cfg_attr(clippy, allow(lintname))]` which is somewhat tedious.

The compiler should support something like `#[allow(clippy::lintname)]` which won't attempt to warn about nonexistant lints
at all when not running clippy.


## Stability guarantees

Clippy will have the same idea of of lint stability as rustc; essentially we do not guarantee stability under #[deny(lintname)].
This is not a problem since deny only affects the current crate (dependencies have their lints capped)
so at most you’ll be forced to slap on an `#[allow()]` for your _own_ crate following a Rust upgrade.

This means that we will never remove lints. We may recategorize lints, and we may "deprecate" them. Deprecation "removes" them by
removing their functionality and marking them as deprecated, which may cause further warnings but cannot cause a compiler
error.

It also means that we won't make fundamentally large changes to lints. You can expect that turning on a lint will keep it behaving
mostly similarly over time, unless it is removed. The kinds of changes we will make are:

 - Adding entirely new lints
 - Fixing false positives (A lint may no longer lint in a buggy case)
 - Fixing false negatives (A case where the lint _should_ be linting but doesn’t is fixed)
 - Bugfixes (When the lint panics or does something otherwise totally broken)

When fixing false negatives this will usually be fixing things that can be
understood as comfortably within the scope of the lint as documented/named.
For example, a lint on having the type `Box<Vec<_>>` may be changed to also catch `Box<Vec<T>>`
where `T` is generic, but will not be changed to also catch `Box<String>` (which can be linted
on for the same reasons).

An exception to this is the "nursery" lints &mdash; Clippy has a lint category for unpolished lints called the "nursery" which
are allow-by-default. These may experience radical changes, however they will never be entirely "removed" either.

Pre-1.0 we may also flush out all of the deprecated lints.

## Lint audit and categories

A couple months ago we did a lint audit to recategorize all the Clippy lints. The [Reference-Level explanation below][cat] contains a list
of all of these lints as currently categorized.

The categories we came up with are:


 - Correctness (Deny): Probable bugs, e.g. calling `.clone()` on `&&T`,
   which clones the (`Copy`) reference and not the actual type
 - Style (Warn): Style issues; where the fix usually doesn't semantically change the code but instead changes naming/formatting.
   For example, having a method named `into_foo()` that doesn't take `self` by-move
 - Complexity (Warn): For detecting unnecessary code complexities and helping
   simplify them. For example, a lint that asks you to replace `.filter(..).next()` with `.find(..)`
 - Perf (Warn): Detecting potential performance footguns, like using `Box<Vec<T>>` or calling `.or(foo())` instead of `or_else(foo)`.
 - Pedantic (Allow): Controversial or exceedingly pedantic lints
 - Nursery (Allow): For lints which are buggy or need more work
 - Cargo (Allow): Lints about your Cargo setup
 - Restriction (Allow): Lints for things which are not usually a problem, but may be something specific situations may dictate disallowing.
 - Internal (Allow): Nothing to see here, move along
 - Deprecated (Allow): Empty lints that exist to ensure that `#[allow(lintname)]` still compiles after the lint was deprecated.

Lints can only belong to one lint group at a time, and the lint group defines the lint level. There is a bunch of overlap between
the style and complexity groups -- a lot of style issues are also complexity issues and vice versa. We separate these groups
so that people can opt in to the complexity lints without having to opt in to Clippy's style.

## Compiler uplift

The compiler has historically had a "no new lints" policy, partly with the desire that lints would
incubate outside of Clippy. This feels like a good time to look into uplifting these lints.

This RFC does not _yet_ propose lints to be uplifted, but the intention is that the RFC
discussion will bring up lints that the community feels _should_ be uplifted and we can list them here.

Such an uplift may change the lint level; correctness lints are Deny
by default in Clippy but would probably switch to Warn if uplifted since the compiler is more
conservative here (Using clippy is in itself an opt-in to a "please annoy me more" mode).


We'd also like to establish a rough policy for future lints here:  Some correctness lints should probably belong in the compiler,
whereas style/complexity/etc lints should probably belong in Clippy. Lints may be incubated in Clippy, of course.

I don't think the compler will want _all_ correctness lints here, however if the lint is about a common enough situation
where it being _not_ a bug is an exceedingly rare case (i.e. very low false positive frequency) it should probably belong in the
compiler.

 [cat]: #lint-categorization

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation


## Lint categorization

This categorization can be browsed [online].

 [online]: http://rust-lang-nursery.github.io/rust-clippy/current/

Please leave comments on thoughts about these lints -- if their categorization is correct, if they should exist at all, and if we should be uplifting them to the compiler.


### correctness (Deny)
* [naive_bytecount](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#naive_bytecount) (use of naive `<slice>.filter(|&x| x == y).count()` to count byte values)
* [map_entry](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#map_entry) (use of `contains_key` followed by `insert` on a `HashMap` or `BTreeMap`)
* [boxed_local](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#boxed_local) (using `Box<T>` where unnecessary)
* [large_enum_variant](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#large_enum_variant) (large size difference between variants on an enum)
* [manual_memcpy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#manual_memcpy) (manually copying items between slices)
* [unused_collect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unused_collect) (`collect()`ing an iterator without using the result; this is usually better written as a for loop)
* [expect_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#expect_fun_call) (using any `expect` method with a function call)
* [iter_nth](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_nth) (using `.iter().nth()` on a standard library type with O(1) element access)
* [or_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#or_fun_call) (using any `*or` method with a function call, which suggests `*or_else`)
* [single_char_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#single_char_pattern) (using a single-character str where a char could be used, e.g. `_.split("x")`)
* [cmp_owned](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cmp_owned) (creating owned instances for comparing with others, e.g. `x == "foo".to_string()`)
* [mutex_atomic](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mutex_atomic) (using a mutex where an atomic value could be used instead)
* [box_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#box_vec) (usage of `Box<Vec<T>>`, vector elements are already on the heap)
* [useless_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_vec) (useless `vec!`)

### style (Warn)
* [naive_bytecount](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#naive_bytecount) (use of naive `<slice>.filter(|&x| x == y).count()` to count byte values)
* [map_entry](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#map_entry) (use of `contains_key` followed by `insert` on a `HashMap` or `BTreeMap`)
* [boxed_local](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#boxed_local) (using `Box<T>` where unnecessary)
* [large_enum_variant](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#large_enum_variant) (large size difference between variants on an enum)
* [manual_memcpy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#manual_memcpy) (manually copying items between slices)
* [unused_collect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unused_collect) (`collect()`ing an iterator without using the result; this is usually better written as a for loop)
* [expect_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#expect_fun_call) (using any `expect` method with a function call)
* [iter_nth](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_nth) (using `.iter().nth()` on a standard library type with O(1) element access)
* [or_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#or_fun_call) (using any `*or` method with a function call, which suggests `*or_else`)
* [single_char_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#single_char_pattern) (using a single-character str where a char could be used, e.g. `_.split("x")`)
* [cmp_owned](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cmp_owned) (creating owned instances for comparing with others, e.g. `x == "foo".to_string()`)
* [mutex_atomic](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mutex_atomic) (using a mutex where an atomic value could be used instead)
* [box_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#box_vec) (usage of `Box<Vec<T>>`, vector elements are already on the heap)
* [useless_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_vec) (useless `vec!`)

### complexity (Warn)
* [naive_bytecount](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#naive_bytecount) (use of naive `<slice>.filter(|&x| x == y).count()` to count byte values)
* [map_entry](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#map_entry) (use of `contains_key` followed by `insert` on a `HashMap` or `BTreeMap`)
* [boxed_local](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#boxed_local) (using `Box<T>` where unnecessary)
* [large_enum_variant](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#large_enum_variant) (large size difference between variants on an enum)
* [manual_memcpy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#manual_memcpy) (manually copying items between slices)
* [unused_collect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unused_collect) (`collect()`ing an iterator without using the result; this is usually better written as a for loop)
* [expect_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#expect_fun_call) (using any `expect` method with a function call)
* [iter_nth](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_nth) (using `.iter().nth()` on a standard library type with O(1) element access)
* [or_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#or_fun_call) (using any `*or` method with a function call, which suggests `*or_else`)
* [single_char_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#single_char_pattern) (using a single-character str where a char could be used, e.g. `_.split("x")`)
* [cmp_owned](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cmp_owned) (creating owned instances for comparing with others, e.g. `x == "foo".to_string()`)
* [mutex_atomic](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mutex_atomic) (using a mutex where an atomic value could be used instead)
* [box_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#box_vec) (usage of `Box<Vec<T>>`, vector elements are already on the heap)
* [useless_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_vec) (useless `vec!`)

### perf (Warn)
* [naive_bytecount](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#naive_bytecount) (use of naive `<slice>.filter(|&x| x == y).count()` to count byte values)
* [map_entry](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#map_entry) (use of `contains_key` followed by `insert` on a `HashMap` or `BTreeMap`)
* [boxed_local](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#boxed_local) (using `Box<T>` where unnecessary)
* [large_enum_variant](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#large_enum_variant) (large size difference between variants on an enum)
* [manual_memcpy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#manual_memcpy) (manually copying items between slices)
* [unused_collect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unused_collect) (`collect()`ing an iterator without using the result; this is usually better written as a for loop)
* [expect_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#expect_fun_call) (using any `expect` method with a function call)
* [iter_nth](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_nth) (using `.iter().nth()` on a standard library type with O(1) element access)
* [or_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#or_fun_call) (using any `*or` method with a function call, which suggests `*or_else`)
* [single_char_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#single_char_pattern) (using a single-character str where a char could be used, e.g. `_.split("x")`)
* [cmp_owned](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cmp_owned) (creating owned instances for comparing with others, e.g. `x == "foo".to_string()`)
* [mutex_atomic](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mutex_atomic) (using a mutex where an atomic value could be used instead)
* [box_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#box_vec) (usage of `Box<Vec<T>>`, vector elements are already on the heap)
* [useless_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_vec) (useless `vec!`)

### pedantic (Allow)
* [naive_bytecount](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#naive_bytecount) (use of naive `<slice>.filter(|&x| x == y).count()` to count byte values)
* [map_entry](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#map_entry) (use of `contains_key` followed by `insert` on a `HashMap` or `BTreeMap`)
* [boxed_local](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#boxed_local) (using `Box<T>` where unnecessary)
* [large_enum_variant](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#large_enum_variant) (large size difference between variants on an enum)
* [manual_memcpy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#manual_memcpy) (manually copying items between slices)
* [unused_collect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unused_collect) (`collect()`ing an iterator without using the result; this is usually better written as a for loop)
* [expect_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#expect_fun_call) (using any `expect` method with a function call)
* [iter_nth](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_nth) (using `.iter().nth()` on a standard library type with O(1) element access)
* [or_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#or_fun_call) (using any `*or` method with a function call, which suggests `*or_else`)
* [single_char_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#single_char_pattern) (using a single-character str where a char could be used, e.g. `_.split("x")`)
* [cmp_owned](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cmp_owned) (creating owned instances for comparing with others, e.g. `x == "foo".to_string()`)
* [mutex_atomic](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mutex_atomic) (using a mutex where an atomic value could be used instead)
* [box_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#box_vec) (usage of `Box<Vec<T>>`, vector elements are already on the heap)
* [useless_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_vec) (useless `vec!`)

### nursery (Allow)
* [naive_bytecount](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#naive_bytecount) (use of naive `<slice>.filter(|&x| x == y).count()` to count byte values)
* [map_entry](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#map_entry) (use of `contains_key` followed by `insert` on a `HashMap` or `BTreeMap`)
* [boxed_local](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#boxed_local) (using `Box<T>` where unnecessary)
* [large_enum_variant](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#large_enum_variant) (large size difference between variants on an enum)
* [manual_memcpy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#manual_memcpy) (manually copying items between slices)
* [unused_collect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unused_collect) (`collect()`ing an iterator without using the result; this is usually better written as a for loop)
* [expect_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#expect_fun_call) (using any `expect` method with a function call)
* [iter_nth](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_nth) (using `.iter().nth()` on a standard library type with O(1) element access)
* [or_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#or_fun_call) (using any `*or` method with a function call, which suggests `*or_else`)
* [single_char_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#single_char_pattern) (using a single-character str where a char could be used, e.g. `_.split("x")`)
* [cmp_owned](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cmp_owned) (creating owned instances for comparing with others, e.g. `x == "foo".to_string()`)
* [mutex_atomic](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mutex_atomic) (using a mutex where an atomic value could be used instead)
* [box_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#box_vec) (usage of `Box<Vec<T>>`, vector elements are already on the heap)
* [useless_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_vec) (useless `vec!`)

### restriction (Allow)
* [naive_bytecount](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#naive_bytecount) (use of naive `<slice>.filter(|&x| x == y).count()` to count byte values)
* [map_entry](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#map_entry) (use of `contains_key` followed by `insert` on a `HashMap` or `BTreeMap`)
* [boxed_local](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#boxed_local) (using `Box<T>` where unnecessary)
* [large_enum_variant](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#large_enum_variant) (large size difference between variants on an enum)
* [manual_memcpy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#manual_memcpy) (manually copying items between slices)
* [unused_collect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unused_collect) (`collect()`ing an iterator without using the result; this is usually better written as a for loop)
* [expect_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#expect_fun_call) (using any `expect` method with a function call)
* [iter_nth](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_nth) (using `.iter().nth()` on a standard library type with O(1) element access)
* [or_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#or_fun_call) (using any `*or` method with a function call, which suggests `*or_else`)
* [single_char_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#single_char_pattern) (using a single-character str where a char could be used, e.g. `_.split("x")`)
* [cmp_owned](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cmp_owned) (creating owned instances for comparing with others, e.g. `x == "foo".to_string()`)
* [mutex_atomic](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mutex_atomic) (using a mutex where an atomic value could be used instead)
* [box_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#box_vec) (usage of `Box<Vec<T>>`, vector elements are already on the heap)
* [useless_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_vec) (useless `vec!`)



# Rationale and alternatives
[alternatives]: #alternatives

We don't particularly _need_ a 1.0, however it's good to have a milestone here, and a general idea of stability as we move forward in this process.

It's also good to have some community involvement in the lint design/categorization process since clippy lints
both reflect and affect the general style of the community.

# Unresolved questions
[unresolved]: #unresolved-questions

Through the process of this RFC we hope to determine if there are lints which need
to be uplifted, recategorized, or removed.

