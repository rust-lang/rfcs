- Feature Name: `clippy_uno`
- Start Date: 2018-06-14
- RFC PR: [rust-lang/rfcs#2476](https://github.com/rust-lang/rfcs/pull/2476)
- Rust Issue: [rust-lang-nursery/rust-clippy#3343](https://github.com/rust-lang-nursery/rust-clippy/issues/3343)

# Summary
[summary]: #summary

Release Clippy 1.0, in preparation for it being shipped via rustup and eventually available via Rust Stable.

# Motivation
[motivation]: #motivation

See also: [The Future of Clippy][future]

Clippy, the linter for Rust, has been a nightly-only plugin to Rust for many years.
In that time, it's grown big, but it's nightly-only nature makes it pretty hard to use.

The eventual plan is to integrate it in Rustup à la Rustfmt/RLS so that you can simply fetch prebuilt binaries
for your system and `cargo clippy` Just Works ™️. In preparation for this, we'd like to nail down various things
about its lints and their categorization.

[future]: https://manishearth.github.io/blog/2018/06/05/the-future-of-clippy-the-rust-linter/

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Usage and lint philosophy

We expect Clippy to be used via `cargo clippy`.

Clippy aims to follow the general Rust style. It may be somewhat opiniated in some situations.

In general Clippy is intended to be used with a liberal sprinkling of `#[allow()]` and `#[warn()]`; _it is okay_ to
disagree with Clippy's choices. This is a weaker philosophy than that behind rustc's lints, where usually flipping
one is an indication of a very specialized situation.

## Lint attributes

Currently to allow/deny Clippy lints you have to `#[cfg_attr(clippy, allow(lintname))]` which is somewhat tedious.

The compiler should support something like `#[allow(clippy::lintname)]` which won't attempt to warn about nonexistent lints
at all when not running Clippy.


## Stability guarantees

Clippy will have the same idea of lint stability as rustc; essentially we do not guarantee stability under `#[deny(lintname)]`.
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

The configuration file for clippy, clippy.toml, is not stabilized in this RFC. Instead, we propose to require clippy.toml users set a `clippy_toml_is_unstable_and_may_go_away` option.

The interface and existence of `cargo-clippy` is also not stabilized in this RFC. We will continue shipping it with rustup, but it may be replaced in the future with a combined `cargo lint` command.

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
incubate outside of the compiler (so usually in Clippy). This feels like a good time to look into uplifting these lints.

This RFC does not _yet_ propose lints to be uplifted, but the intention is that the RFC
discussion will bring up lints that the community feels _should_ be uplifted and we can list them here.

Such an uplift may change the lint level; correctness lints are Deny
by default in Clippy but would probably switch to Warn if uplifted since the compiler is more
conservative here (Using Clippy is in itself an opt-in to a "please annoy me more" mode).


We'd also like to establish a rough policy for future lints here:  Some correctness lints should probably belong in the compiler,
whereas style/complexity/etc lints should probably belong in Clippy. Lints may be incubated in Clippy, of course.

I don't think the compler will want _all_ correctness lints here, however if the lint is about a common enough situation
where it being _not_ a bug is an exceedingly rare case (i.e. very low false positive frequency) it should probably belong in the
compiler.

## What lints belong in clippy?

Essentially, we consider the categorization itself to be a definition of boundaries -- if it doesn't fit in the categories,
it doesn't fit in clippy (or needs an RFC for, specifically).

In itself this isn't complete, we explicitly have a "pedantic" group that's kinda ill defined.

The rules for the main categories (style/complexity/correctness/perf -- things which are warn or deny by default) are:

 - Main category lints need to be something the community has general agreement on. This does _not_ mean each lint
   addition must go through an RFC-like process. Instead, this is to be judged by the maintainers during the review of the lint pull request
   (taking into account objections raised if any). If the lint turns out to be controversial in the future we can flip it off or recategorize it.
 - Generally, _if_ a lint is triggered, this should be _useful_ to _most_ Rust programmers seeing it _most_ of the time.
  - It is okay for a lint to deal with niche code that usually won't even be triggered. Lints can target subsets of the community provided they don't randomly trigger for others.
  - It is okay if the lint has some false positives (cases where it lints for things which are actually okay), as long as they don't dominate.
  - It is also okay if the lint warns about things which people do not feel are worth fixing -- i.e. the programmer agrees that it is a problem
    but does not wish to fix this. Using clippy is itself an opt-in to more finicky linting. However, this is sometimes an indicator of such a lint potentially belonging in the pedantic group.
 - Clippy is meant to be used with a liberal sprinkling of `allow`. If there's a specific use case where a lint doesn't apply, and the solution
   is to slap `allow` on it, that's okay. A minor level of false positives like this is to be tolerated. Similarly, style lints are allowed to be
   about things a lot of people don't care about (i.e. they don't prefer the _opposite_ style, they just don't care). 
 - Clippy lints _do_ deal with the visual presentation of your code, but only for things which `rustfmt` doesn't or can't handle. So, for example,
   rustfmt will not ask you to replace `if {} else { if {} }` with `if {} else if {}`, but clippy might. There is some overlap in this area and we expect
   to work with rustfmt on precisely figuring out what goes where. Such lints are usually `style` lints or `complexity` lints.
 - Clippy lints are allowed to make some kind of semantic changes, but not all:
   - The general rule is that clippy will not attempt to change what it perceives to be the intent of the code, but will rather change
     the code to make it closer to the intent or make it achieve that intent better
   - Clippy lints _do_ deal with potential typos and mistakes. For example, clippy will detect `for x in y.next()` which is
     very likely a bug (you either mean `if let` or mean to unwrap). Such lints are usually `correctness` lints.
   - Clippy lints also _do_ deal with misunderstandings of Rust, for example code doing `foo == NaN` is a misunderstanding
     of how Rust floats work. These are also usually `correctness` lints.
   - Clippy lints _do not_ comment on the business logic of your program. This comes from the "perceived intent" rule
     above, changes to business logic are a change to perceived intent.
   - Clippy lints _do_ ask you to make semantic changes that achieve the same _effect_ with
     perhaps better performance.  Such lints are usually `perf` lints.


For the other categories (these are allow by default):

 - Lints which are "pedantic" should still roughly fall into one of the main categories, just that they are too annoying
   (or possibly controversial) to be warn by default. So a lint must follow all the above rules if pedantic, but is allowed to be
   "too finicky to fix", and may have looser consensus (i.e. some controversy).
 - Similar rules for "nursery" except their reason for being allow by default is lack of maturity (i.e. the lint is buggy or still needs some thought)
 - "restriction" lints follow all the rules for semantic changes, but do not bother with the rules
   for the lint being useful to most rust programmers. A restriction lint must still be such that you have a
   good reason to enable it &mdash; "I dislike such code" is insufficient &mdash; but will likely be a lint most programmers
   wish to keep off by default for most of their code. The goal of restriction lints is to provide tools with which you can supplement
   the language checks in very specific cases where you need it, e.g. forbidding panics from a certain area of code.
 - "cargo" lints follow the same rules as pedantic lints (we only have one of them right now, so we may be experimenting with this in the future)


 [cat]: #lint-categorization

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation


## Lint categorization

This categorization can be browsed [online].

 [online]: http://rust-lang-nursery.github.io/rust-clippy/current/

Please leave comments on thoughts about these lints -- if their categorization is correct, if they should exist at all, and if we should be uplifting them to the compiler.

For ease of review, the lints below are as they were listed in the original RFC. The proposed changes are:

 - `shadow_unrelated` be moved from `restriction` to `pedantic`
 - Various lints be uplifted to the compiler (and potentially renamed). This is tracked in https://github.com/rust-lang/rust/issues/53224
 - `explicit_iter_loop` and `explicit_into_iter_loop` be moved from `style` to `pedantic`


# correctness (Deny)

- [for_loop_over_option](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#for_loop_over_option): Checks for `for` loops over `Option` values.
- [eq_op](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#eq_op): Checks for equal operands to comparison, logical and
bitwise, difference and division binary operators (`==`, `>`, etc., `&&`,
`||`, `&`, `|`, `^`, `-` and `/`).
- [iter_next_loop](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_next_loop): Checks for loops on `x.next()`.
- [deprecated_semver](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#deprecated_semver): Checks for `#[deprecated]` annotations with a `since`
field that is not a valid semantic version.
- [drop_copy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#drop_copy): Checks for calls to `std::mem::drop` with a value
that derives the Copy trait
- [not_unsafe_ptr_arg_deref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#not_unsafe_ptr_arg_deref): Checks for public functions that dereferences raw pointer
arguments but are not marked unsafe.
- [logic_bug](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#logic_bug): Checks for boolean expressions that contain terminals that
can be eliminated.
- [clone_double_ref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#clone_double_ref): Checks for usage of `.clone()` on an `&&T`.
- [almost_swapped](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#almost_swapped): Checks for `foo = bar; bar = foo` sequences.
- [possible_missing_comma](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#possible_missing_comma): Checks for possible missing comma in an array. It lints if
an array element is a binary operator expression and it lies on two lines.
- [wrong_transmute](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#wrong_transmute): Checks for transmutes that can't ever be correct on any
architecture.
- [invalid_regex](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#invalid_regex): Checks [regex](https://crates.io/crates/regex) creation
(with `Regex::new`,`RegexBuilder::new` or `RegexSet::new`) for correct
regex syntax.
- [bad_bit_mask](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#bad_bit_mask): Checks for incompatible bit masks in comparisons.
- [drop_ref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#drop_ref): Checks for calls to `std::mem::drop` with a reference
instead of an owned value.
- [derive_hash_xor_eq](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#derive_hash_xor_eq): Checks for deriving `Hash` but implementing `PartialEq`
explicitly or vice versa.
- [useless_attribute](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_attribute): Checks for `extern crate` and `use` items annotated with
lint attributes
- [temporary_cstring_as_ptr](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#temporary_cstring_as_ptr): Checks for getting the inner pointer of a temporary
`CString`.
- [min_max](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#min_max): Checks for expressions where `std::cmp::min` and `max` are
used to clamp values, but switched so that the result is constant.
- [unit_cmp](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unit_cmp): Checks for comparisons to unit.
- [reverse_range_loop](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#reverse_range_loop): Checks for loops over ranges `x..y` where both `x` and `y`
are constant and `x` is greater or equal to `y`, unless the range is
reversed or has a negative `.step_by(_)`.
- [erasing_op](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#erasing_op): Checks for erasing operations, e.g. `x * 0`.
- [suspicious_op_assign_impl](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#suspicious_op_assign_impl): Lints for suspicious operations in impls of OpAssign, e.g.
subtracting elements in an AddAssign impl.
- [float_cmp](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#float_cmp): Checks for (in-)equality comparisons on floating-point
values (apart from zero), except in functions called `*eq*` (which probably
implement equality for a type involving floats).
- [zero_width_space](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#zero_width_space): Checks for the Unicode zero-width space in the code.
- [fn_to_numeric_cast_with_truncation](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#fn_to_numeric_cast_with_truncation): Checks for casts of a function pointer to a numeric type not enough to store address.
- [suspicious_arithmetic_impl](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#suspicious_arithmetic_impl): Lints for suspicious operations in impls of arithmetic operators, e.g.
subtracting elements in an Add impl.
- [approx_constant](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#approx_constant): Checks for floating point literals that approximate
constants which are defined in
[`std::f32::consts`](https://doc.rust-lang.org/stable/std/f32/consts/#constants) or [`std::f64::consts`](https://doc.rust-lang.org/stable/std/f64/consts/#constants), respectively, suggesting to use the predefined constant.
- [while_immutable_condition](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#while_immutable_condition): Checks whether variables used within while loop condition
can be (and are) mutated in the body.
- [never_loop](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#never_loop): Checks for loops that will always `break`, `return` or
`continue` an outer loop.
- [nonsensical_open_options](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#nonsensical_open_options): Checks for duplicate open options as well as combinations
that make no sense.
- [forget_copy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#forget_copy): Checks for calls to `std::mem::forget` with a value that
derives the Copy trait
- [if_same_then_else](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#if_same_then_else): Checks for `if/else` with the same body as the *then* part
and the *else* part.
- [cast_ptr_alignment](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cast_ptr_alignment): Checks for casts from a less-strictly-aligned pointer to a
more-strictly-aligned pointer
- [ifs_same_cond](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#ifs_same_cond): Checks for consecutive `if`s with the same condition.
- [out_of_bounds_indexing](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#out_of_bounds_indexing): Checks for out of bounds array indexing with a constant
index.
- [modulo_one](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#modulo_one): Checks for getting the remainder of a division by one.
- [inline_fn_without_body](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#inline_fn_without_body): Checks for `#[inline]` on trait methods without bodies
- [cmp_nan](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cmp_nan): Checks for comparisons to NaN.
- [ineffective_bit_mask](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#ineffective_bit_mask): Checks for bit masks in comparisons which can be removed
without changing the outcome.
- [infinite_iter](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#infinite_iter): Checks for iteration that is guaranteed to be infinite.
- [mut_from_ref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mut_from_ref): This lint checks for functions that take immutable
references and return
mutable ones.
- [unused_io_amount](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unused_io_amount): Checks for unused written/read amount.
- [invalid_ref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#invalid_ref): Checks for creation of references to zeroed or uninitialized memory.
- [serde_api_misuse](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#serde_api_misuse): Checks for mis-uses of the serde API.
- [forget_ref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#forget_ref): Checks for calls to `std::mem::forget` with a reference
instead of an owned value.
- [absurd_extreme_comparisons](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#absurd_extreme_comparisons): Checks for comparisons where one side of the relation is
either the minimum or maximum value for its type and warns if it involves a
case that is always true or always false. Only integer and boolean types are
checked.
- [for_loop_over_result](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#for_loop_over_result): Checks for `for` loops over `Result` values.
- [iterator_step_by_zero](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iterator_step_by_zero): Checks for calling `.step_by(0)` on iterators,
which never terminates.
- [enum_clike_unportable_variant](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#enum_clike_unportable_variant): Checks for C-like enumerations that are
`repr(isize/usize)` and have values that don't fit into an `i32`.


# style (Warn)

- [inconsistent_digit_grouping](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#inconsistent_digit_grouping): Warns if an integral or floating-point constant is
grouped inconsistently with underscores.
- [get_unwrap](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#get_unwrap): Checks for use of `.get().unwrap()` (or
`.get_mut().unwrap`) on a standard library type which implements `Index`
- [match_bool](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#match_bool): Checks for matches where match expression is a `bool`. It
suggests to replace the expression with an `if...else` block.
- [cmp_null](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cmp_null): This lint checks for equality comparisons with `ptr::null`
- [write_with_newline](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#write_with_newline): This lint warns when you use `write!()` with a format
string that
ends in a newline.
- [unneeded_field_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unneeded_field_pattern): Checks for structure field patterns bound to wildcards.
- [new_without_default_derive](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#new_without_default_derive): Checks for types with a `fn new() -> Self` method
and no implementation of
[`Default`](https://doc.rust-lang.org/std/default/trait.Default.html),
where the `Default` can be derived by `#[derive(Default)]`.
- [zero_ptr](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#zero_ptr): Catch casts from `0` to some pointer type
- [wrong_self_convention](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#wrong_self_convention): Checks for methods with certain name prefixes and which
doesn't match how self is taken.
- [iter_skip_next](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_skip_next): Checks for use of `.skip(x).next()` on iterators.
- [large_digit_groups](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#large_digit_groups): Warns if the digits of an integral or floating-point
constant are grouped into groups that
are too large.
- [range_minus_one](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#range_minus_one): Checks for inclusive ranges where 1 is subtracted from
the upper bound, e.g. `x..=(y-1)`.
- [regex_macro](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#regex_macro): Checks for usage of `regex!(_)` which (as of now) is
usually slower than `Regex::new(_)` unless called in a loop (which is a bad
idea anyway).
- [op_ref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#op_ref): Checks for arguments to `==` which have their address
taken to satisfy a bound
and suggests to dereference the other argument instead
- [question_mark](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#question_mark): Checks for expressions that could be replaced by the question mark operator
- [redundant_closure](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#redundant_closure): Checks for closures which just call another function where
the function can be called directly. `unsafe` functions or calls where types
get adjusted are ignored.
- [print_with_newline](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#print_with_newline): This lint warns when you use `print!()` with a format
string that
ends in a newline.
- [match_ref_pats](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#match_ref_pats): Checks for matches where all arms match a reference,
suggesting to remove the reference and deref the matched expression
instead. It also checks for `if let &foo = bar` blocks.
- [ptr_arg](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#ptr_arg): This lint checks for function arguments of type `&String`
or `&Vec` unless the references are mutable. It will also suggest you
replace `.clone()` calls with the appropriate `.to_owned()`/`to_string()`
calls.
- [chars_last_cmp](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#chars_last_cmp): Checks for usage of `.chars().last()` or
`.chars().next_back()` on a `str` to check if it ends with a given char.
- [assign_op_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#assign_op_pattern): Checks for `a = a op b` or `a = b commutative_op a`
patterns.
- [mixed_case_hex_literals](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mixed_case_hex_literals): Warns on hexadecimal literals with mixed-case letter
digits.
- [blacklisted_name](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#blacklisted_name): Checks for usage of blacklisted names for variables, such
as `foo`.
- [double_neg](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#double_neg): Detects expressions of the form `--x`.
- [unnecessary_fold](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unnecessary_fold): Checks for using `fold` when a more succinct alternative exists.
Specifically, this checks for `fold`s which could be replaced by `any`, `all`,
`sum` or `product`.
- [let_unit_value](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#let_unit_value): Checks for binding a unit value.
- [needless_range_loop](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#needless_range_loop): Checks for looping over the range of `0..len` of some
collection just to get the values by index.
- [excessive_precision](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#excessive_precision): Checks for float literals with a precision greater
than that supported by the underlying type
- [duplicate_underscore_argument](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#duplicate_underscore_argument): Checks for function arguments having the similar names
differing by an underscore.
- [println_empty_string](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#println_empty_string): This lint warns when you use `println!("")` to
print a newline.
- [panic_params](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#panic_params): Checks for missing parameters in `panic!`.
- [writeln_empty_string](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#writeln_empty_string): This lint warns when you use `writeln!(buf, "")` to
print a newline.
- [infallible_destructuring_match](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#infallible_destructuring_match): Checks for matches being used to destructure a single-variant enum
or tuple struct where a `let` will suffice.
- [block_in_if_condition_stmt](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#block_in_if_condition_stmt): Checks for `if` conditions that use blocks containing
statements, or conditions that use closures with blocks.
- [unreadable_literal](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unreadable_literal): Warns if a long integral or floating-point constant does
not contain underscores.
- [unsafe_removed_from_name](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unsafe_removed_from_name): Checks for imports that remove "unsafe" from an item's
name.
- [builtin_type_shadow](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#builtin_type_shadow): Warns if a generic shadows a built-in type.
- [option_map_or_none](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#option_map_or_none): Checks for usage of `_.map_or(None, _)`.
- [neg_multiply](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#neg_multiply): Checks for multiplication by -1 as a form of negation.
- [const_static_lifetime](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#const_static_lifetime): Checks for constants with an explicit `'static` lifetime.
- [explicit_iter_loop](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#explicit_iter_loop): Checks for loops on `x.iter()` where `&x` will do, and
suggests the latter.
- [single_match](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#single_match): Checks for matches with a single arm where an `if let`
will usually suffice.
- [for_kv_map](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#for_kv_map): Checks for iterating a map (`HashMap` or `BTreeMap`) and
ignoring either the keys or values.
- [if_let_some_result](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#if_let_some_result): * Checks for unnecessary `ok()` in if let.
- [collapsible_if](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#collapsible_if): Checks for nested `if` statements which can be collapsed
by `&&`-combining their conditions and for `else { if ... }` expressions
that
can be collapsed to `else if ...`.
- [len_without_is_empty](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#len_without_is_empty): Checks for items that implement `.len()` but not
`.is_empty()`.
- [unnecessary_mut_passed](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unnecessary_mut_passed): Detects giving a mutable reference to a function that only
requires an immutable reference.
- [useless_let_if_seq](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_let_if_seq): Checks for variable declarations immediately followed by a
conditional affectation.
- [new_ret_no_self](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#new_ret_no_self): Checks for `new` not returning `Self`.
- [write_literal](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#write_literal): This lint warns about the use of literals as `write!`/`writeln!` args.
- [block_in_if_condition_expr](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#block_in_if_condition_expr): Checks for `if` conditions that use blocks to contain an
expression.
- [toplevel_ref_arg](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#toplevel_ref_arg): Checks for function arguments and let bindings denoted as
`ref`.
- [suspicious_else_formatting](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#suspicious_else_formatting): Checks for formatting of `else if`. It lints if the `else`
and `if` are not on the same line or the `else` seems to be missing.
- [fn_to_numeric_cast](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#fn_to_numeric_cast): Checks for casts of a function pointer to a numeric type except `usize`.
- [let_and_return](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#let_and_return): Checks for `let`-bindings, which are subsequently
returned.
- [len_zero](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#len_zero): Checks for getting the length of something via `.len()`
just to compare to zero, and suggests using `.is_empty()` where applicable.
- [suspicious_assignment_formatting](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#suspicious_assignment_formatting): Checks for use of the non-existent `=*`, `=!` and `=-`
operators.
- [redundant_field_names](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#redundant_field_names): Checks for fields in struct literals where shorthands
could be used.
- [string_lit_as_bytes](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#string_lit_as_bytes): Checks for the `as_bytes` method called on string literals
that contain only ASCII characters.
- [verbose_bit_mask](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#verbose_bit_mask): Checks for bit masks that can be replaced by a call
to `trailing_zeros`
- [map_clone](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#map_clone): Checks for mapping `clone()` over an iterator.
- [new_without_default](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#new_without_default): Checks for types with a `fn new() -> Self` method and no
implementation of
[`Default`](https://doc.rust-lang.org/std/default/trait.Default.html).
- [should_implement_trait](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#should_implement_trait): Checks for methods that should live in a trait
implementation of a `std` trait (see [llogiq's blog
post](http://llogiq.github.io/2015/07/30/traits.html) for further
information) instead of an inherent implementation.
- [match_wild_err_arm](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#match_wild_err_arm): Checks for arm which matches all errors with `Err(_)`
and take drastic actions like `panic!`.
- [iter_cloned_collect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_cloned_collect): Checks for the use of `.cloned().collect()` on slice to
create a `Vec`.
- [module_inception](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#module_inception): Checks for modules that have the same name as their
parent module
- [many_single_char_names](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#many_single_char_names): Checks for too many variables whose name consists of a
single character.
- [enum_variant_names](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#enum_variant_names): Detects enumeration variants that are prefixed or suffixed
by the same characters.
- [string_extend_chars](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#string_extend_chars): Checks for the use of `.extend(s.chars())` where s is a
`&str` or `String`.
- [needless_return](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#needless_return): Checks for return statements at the end of a block.
- [print_literal](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#print_literal): This lint warns about the use of literals as `print!`/`println!` args.
- [implicit_hasher](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#implicit_hasher): Checks for public `impl` or `fn` missing generalization
over different hashers and implicitly defaulting to the default hashing
algorithm (SipHash).
- [needless_pass_by_value](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#needless_pass_by_value): Checks for functions taking arguments by value, but not
consuming them in its
body.
- [trivial_regex](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#trivial_regex): Checks for trivial [regex](https://crates.io/crates/regex)
creation (with `Regex::new`, `RegexBuilder::new` or `RegexSet::new`).
- [while_let_on_iterator](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#while_let_on_iterator): Checks for `while let` expressions on iterators.
- [redundant_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#redundant_pattern): Checks for patterns in the form `name @ _`.
- [match_overlapping_arm](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#match_overlapping_arm): Checks for overlapping match arms.
- [just_underscores_and_digits](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#just_underscores_and_digits): Checks if you have variables whose name consists of just
underscores and digits.
- [ok_expect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#ok_expect): Checks for usage of `ok().expect(..)`.
- [empty_loop](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#empty_loop): Checks for empty `loop` expressions.
- [explicit_into_iter_loop](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#explicit_into_iter_loop): Checks for loops on `y.into_iter()` where `y` will do, and
suggests the latter.
- [if_let_redundant_pattern_matching](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#if_let_redundant_pattern_matching): Lint for redundant pattern matching over `Result` or
`Option`


# complexity (Warn)

- [option_option](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#option_option): Checks for use of `Option<Option<_>>` in function signatures and type
definitions
- [precedence](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#precedence): Checks for operations where precedence may be unclear
and suggests to add parentheses. Currently it catches the following:
  - mixed usage of arithmetic and bit shifting/combining operators without
  parentheses
  - a "negative" numeric literal (which is really a unary `-` followed by a
  numeric literal)
  followed by a method call
- [useless_transmute](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_transmute): Checks for transmutes to the original type of the object
and transmutes that could be a cast.
- [partialeq_ne_impl](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#partialeq_ne_impl): Checks for manual re-implementations of `PartialEq::ne`.
- [redundant_closure_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#redundant_closure_call): Detects closures called in the same expression where they
are defined.
- [manual_swap](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#manual_swap): Checks for manual swapping.
- [option_map_unit_fn](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#option_map_unit_fn): Checks for usage of `option.map(f)` where f is a function
or closure that returns the unit type.
- [overflow_check_conditional](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#overflow_check_conditional): Detects classic underflow/overflow checks.
- [transmute_ptr_to_ref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#transmute_ptr_to_ref): Checks for transmutes from a pointer to a reference.
- [chars_next_cmp](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#chars_next_cmp): Checks for usage of `.chars().next()` on a `str` to check
if it starts with a given char.
- [transmute_bytes_to_str](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#transmute_bytes_to_str): Checks for transmutes from a `&[u8]` to a `&str`.
- [identity_conversion](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#identity_conversion): Checks for always-identical `Into`/`From` conversions.
- [double_parens](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#double_parens): Checks for unnecessary double parentheses.
- [zero_divided_by_zero](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#zero_divided_by_zero): Checks for `0.0 / 0.0`.
- [useless_asref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_asref): Checks for usage of `.as_ref()` or `.as_mut()` where the
types before and after the call are the same.
- [too_many_arguments](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#too_many_arguments): Checks for functions with too many parameters.
- [range_zip_with_len](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#range_zip_with_len): Checks for zipping a collection with the range of
`0.._.len()`.
- [temporary_assignment](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#temporary_assignment): Checks for construction of a structure or tuple just to
assign a value in it.
- [no_effect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#no_effect): Checks for statements which have no effect.
- [short_circuit_statement](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#short_circuit_statement): Checks for the use of short circuit boolean conditions as
a
statement.
- [cast_lossless](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cast_lossless): Checks for on casts between numerical types that may
be replaced by safe conversion functions.
- [unnecessary_operation](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unnecessary_operation): Checks for expression statements that can be reduced to a
sub-expression.
- [cyclomatic_complexity](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cyclomatic_complexity): Checks for methods with high cyclomatic complexity.
- [while_let_loop](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#while_let_loop): Detects `loop + match` combinations that are easier
written as a `while let` loop.
- [needless_update](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#needless_update): Checks for needlessly including a base struct on update
when all fields are changed anyway.
- [identity_op](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#identity_op): Checks for identity operations, e.g. `x + 0`.
- [search_is_some](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#search_is_some): Checks for an iterator search (such as `find()`,
`position()`, or `rposition()`) followed by a call to `is_some()`.
- [useless_format](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_format): Checks for the use of `format!("string literal with no
argument")` and `format!("{}", foo)` where `foo` is a string.
- [diverging_sub_expression](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#diverging_sub_expression): Checks for diverging calls that are not match arms or
statements.
- [transmute_ptr_to_ptr](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#transmute_ptr_to_ptr): Checks for transmutes from a pointer to a pointer, or
from a reference to a reference.
- [crosspointer_transmute](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#crosspointer_transmute): Checks for transmutes between a type `T` and `*T`.
- [needless_borrowed_reference](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#needless_borrowed_reference): Checks for useless borrowed references.
- [transmute_int_to_char](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#transmute_int_to_char): Checks for transmutes from an integer to a `char`.
- [nonminimal_bool](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#nonminimal_bool): Checks for boolean expressions that can be written more
concisely.
- [needless_bool](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#needless_bool): Checks for expressions of the form `if c { true } else {
false }`
(or vice versa) and suggest using the condition directly.
- [misrefactored_assign_op](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#misrefactored_assign_op): Checks for `a op= a op b` or `a op= b op a` patterns.
- [neg_cmp_op_on_partial_ord](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#neg_cmp_op_on_partial_ord): Checks for the usage of negated comparison operators on types which only implement
`PartialOrd` (e.g. `f64`).
- [zero_prefixed_literal](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#zero_prefixed_literal): Warns if an integral constant literal starts with `0`.
- [bool_comparison](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#bool_comparison): Checks for expressions of the form `x == true` (or vice
versa) and suggest using the variable directly.
- [extra_unused_lifetimes](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#extra_unused_lifetimes): Checks for lifetimes in generics that are never used
anywhere else.
- [int_plus_one](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#int_plus_one): Checks for usage of `x >= y + 1` or `x - 1 >= y` (and `<=`) in a block
- [duration_subsec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#duration_subsec): Checks for calculation of subsecond microseconds or milliseconds
from other `Duration` methods.
- [unnecessary_cast](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unnecessary_cast): Checks for casts to the same type.
- [unused_label](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unused_label): Checks for unused labels.
- [result_map_unit_fn](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#result_map_unit_fn): Checks for usage of `result.map(f)` where f is a function
or closure that returns the unit type.
- [clone_on_copy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#clone_on_copy): Checks for usage of `.clone()` on a `Copy` type.
- [unit_arg](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unit_arg): Checks for passing a unit value as an argument to a function without using a unit literal (`()`).
- [transmute_int_to_float](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#transmute_int_to_float): Checks for transmutes from an integer to a float.
- [double_comparisons](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#double_comparisons): Checks for double comparisons that could be simplified to a single expression.
- [eval_order_dependence](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#eval_order_dependence): Checks for a read and a write to the same variable where
whether the read occurs before or after the write depends on the evaluation
order of sub-expressions.
- [ref_in_deref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#ref_in_deref): Checks for references in expressions that use
auto dereference.
- [mut_range_bound](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mut_range_bound): Checks for loops which have a range bound that is a mutable variable
- [transmute_int_to_bool](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#transmute_int_to_bool): Checks for transmutes from an integer to a `bool`.
- [needless_lifetimes](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#needless_lifetimes): Checks for lifetime annotations which can be removed by
relying on lifetime elision.
- [explicit_counter_loop](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#explicit_counter_loop): Checks `for` loops over slices with an explicit counter
and suggests the use of `.enumerate()`.
- [explicit_write](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#explicit_write): Checks for usage of `write!()` / `writeln()!` which can be
replaced with `(e)print!()` / `(e)println!()`
- [deref_addrof](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#deref_addrof): Checks for usage of `*&` and `*&mut` in expressions.
- [filter_next](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#filter_next): Checks for usage of `_.filter(_).next()`.
- [borrowed_box](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#borrowed_box): Checks for use of `&Box<T>` anywhere in the code.
- [type_complexity](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#type_complexity): Checks for types used in structs, parameters and `let`
declarations above a certain complexity threshold.
- [match_as_ref](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#match_as_ref): Checks for match which is used to add a reference to an
`Option` value.
- [char_lit_as_u8](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#char_lit_as_u8): Checks for expressions where a character literal is cast
to `u8` and suggests using a byte literal instead.


# perf (Warn)

- [mutex_atomic](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mutex_atomic): Checks for usages of `Mutex<X>` where an atomic will do.
- [large_enum_variant](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#large_enum_variant): Checks for large size differences between variants on
`enum`s.
- [manual_memcpy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#manual_memcpy): Checks for for-loops that manually copy items between
slices that could be optimized by having a memcpy.
- [boxed_local](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#boxed_local): Checks for usage of `Box<T>` where an unboxed `T` would
work fine.
- [box_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#box_vec): Checks for use of `Box<Vec<_>>` anywhere in the code.
- [useless_vec](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#useless_vec): Checks for usage of `&vec![..]` when using `&[..]` would
be possible.
- [map_entry](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#map_entry): Checks for uses of `contains_key` + `insert` on `HashMap`
or `BTreeMap`.
- [cmp_owned](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cmp_owned): Checks for conversions to owned values just for the sake
of a comparison.
- [or_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#or_fun_call): Checks for calls to `.or(foo(..))`, `.unwrap_or(foo(..))`,
etc., and suggests to use `or_else`, `unwrap_or_else`, etc., or
`unwrap_or_default` instead.
- [unused_collect](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unused_collect): Checks for using `collect()` on an iterator without using
the result.
- [expect_fun_call](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#expect_fun_call): Checks for calls to `.expect(&format!(...))`, `.expect(foo(..))`,
etc., and suggests to use `unwrap_or_else` instead
- [naive_bytecount](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#naive_bytecount): Checks for naive byte counts
- [iter_nth](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#iter_nth): Checks for use of `.iter().nth()` (and the related
`.iter_mut().nth()`) on standard library types with O(1) element access.
- [single_char_pattern](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#single_char_pattern): Checks for string methods that receive a single-character
`str` as an argument, e.g. `_.split("x")`.


# pedantic (Allow)

- [expl_impl_clone_on_copy](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#expl_impl_clone_on_copy): Checks for explicit `Clone` implementations for `Copy`
types.
- [result_map_unwrap_or_else](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#result_map_unwrap_or_else): Checks for usage of `result.map(_).unwrap_or_else(_)`.
- [maybe_infinite_iter](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#maybe_infinite_iter): Checks for iteration that may be infinite.
- [cast_possible_wrap](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cast_possible_wrap): Checks for casts from an unsigned type to a signed type of
the same size. Performing such a cast is a 'no-op' for the compiler,
i.e. nothing is changed at the bit level, and the binary representation of
the value is reinterpreted. This can cause wrapping if the value is too big
for the target signed type. However, the cast works as defined, so this lint
is `Allow` by default.
- [cast_sign_loss](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cast_sign_loss): Checks for casts from a signed to an unsigned numerical
type. In this case, negative values wrap around to large positive values,
which can be quite surprising in practice. However, as the cast works as
defined, this lint is `Allow` by default.
- [enum_glob_use](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#enum_glob_use): Checks for `use Enum::*`.
- [match_same_arms](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#match_same_arms): Checks for `match` with identical arm bodies.
- [single_match_else](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#single_match_else): Checks for matches with a two arms where an `if let` will
usually suffice.
- [pub_enum_variant_names](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#pub_enum_variant_names): Detects enumeration variants that are prefixed or suffixed
by the same characters.
- [use_self](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#use_self): Checks for unnecessary repetition of structure name when a
replacement with `Self` is applicable.
- [option_map_unwrap_or_else](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#option_map_unwrap_or_else): Checks for usage of `_.map(_).unwrap_or_else(_)`.
- [items_after_statements](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#items_after_statements): Checks for items declared after some statement in a block.
- [empty_enum](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#empty_enum): Checks for `enum`s with no variants.
- [needless_continue](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#needless_continue): The lint checks for `if`-statements appearing in loops
that contain a `continue` statement in either their main blocks or their
`else`-blocks, when omitting the `else`-block possibly with some
rearrangement of code can make the code easier to understand.
- [string_add_assign](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#string_add_assign): Checks for string appends of the form `x = x + y` (without
`let`!).
- [used_underscore_binding](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#used_underscore_binding): Checks for the use of bindings with a single leading
underscore.
- [cast_possible_truncation](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cast_possible_truncation): Checks for on casts between numerical types that may
truncate large values. This is expected behavior, so the cast is `Allow` by
default.
- [doc_markdown](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#doc_markdown): Checks for the presence of `_`, `::` or camel-case words
outside ticks in documentation.
- [unseparated_literal_suffix](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unseparated_literal_suffix): Warns if literal suffixes are not separated by an
underscore.
- [if_not_else](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#if_not_else): Checks for usage of `!` or `!=` in an if condition with an
else branch.
- [filter_map](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#filter_map): Checks for usage of `_.filter(_).map(_)`,
`_.filter(_).flat_map(_)`, `_.filter_map(_).flat_map(_)` and similar.
- [stutter](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#stutter): Detects type names that are prefixed or suffixed by the
containing module's name.
- [similar_names](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#similar_names): Checks for names that are very similar and thus confusing.
- [replace_consts](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#replace_consts): Checks for usage of `ATOMIC_X_INIT`, `ONCE_INIT`, and
`uX/iX::MIN/MAX`.
- [option_map_unwrap_or](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#option_map_unwrap_or): Checks for usage of `_.map(_).unwrap_or(_)`.
- [inline_always](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#inline_always): Checks for items annotated with `#[inline(always)]`,
unless the annotated function is empty or simply panics.
- [linkedlist](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#linkedlist): Checks for usage of any `LinkedList`, suggesting to use a
`Vec` or a `VecDeque` (formerly called `RingBuf`).
- [mut_mut](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mut_mut): Checks for instances of `mut mut` references.
- [non_ascii_literal](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#non_ascii_literal): Checks for non-ASCII characters in string literals.
- [unicode_not_nfc](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unicode_not_nfc): Checks for string literals that contain Unicode in a form
that is not equal to its
[NFC-recomposition](http://www.unicode.org/reports/tr15/#Norm_Forms).
- [cast_precision_loss](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#cast_precision_loss): Checks for casts from any numerical to a float type where
the receiving type cannot store all values from the original type without
rounding errors. This possible rounding is to be expected, so this lint is
`Allow` by default.
Basically, this warns on casting any integer with 32 or more bits to `f32`
or any 64-bit integer to `f64`.
- [invalid_upcast_comparisons](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#invalid_upcast_comparisons): Checks for comparisons where the relation is always either
true or false, but where one side has been upcast so that the comparison is
necessary. Only integer types are checked.


# nursery (Allow)

- [empty_line_after_outer_attr](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#empty_line_after_outer_attr): Checks for empty lines after outer attributes
- [needless_borrow](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#needless_borrow): Checks for address of operations (`&`) that are going to
be dereferenced immediately by the compiler.
- [mutex_integer](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mutex_integer): Checks for usages of `Mutex<X>` where `X` is an integral
type.
- [range_plus_one](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#range_plus_one): Checks for exclusive ranges where 1 is added to the
upper bound, e.g. `x..(y+1)`.
- [fallible_impl_from](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#fallible_impl_from): Checks for impls of `From<..>` that contain `panic!()` or `unwrap()`
- [unnecessary_unwrap](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unnecessary_unwrap): Checks for calls of `unwrap[_err]()` that cannot fail.


# restriction (Allow)

- [integer_arithmetic](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#integer_arithmetic): Checks for plain integer arithmetic.
- [shadow_reuse](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#shadow_reuse): Checks for bindings that shadow other bindings already in
scope, while reusing the original value.
- [option_unwrap_used](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#option_unwrap_used): Checks for `.unwrap()` calls on `Option`s.
- [assign_ops](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#assign_ops): Checks for compound assignment operations (`+=` and
similar).
- [shadow_unrelated](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#shadow_unrelated): Checks for bindings that shadow other bindings already in
scope, either without a initialization or with one that does not even use
the original value.
- [clone_on_ref_ptr](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#clone_on_ref_ptr): Checks for usage of `.clone()` on a ref-counted pointer,
(`Rc`, `Arc`, `rc::Weak`, or `sync::Weak`), and suggests calling Clone via unified
function syntax instead (e.g. `Rc::clone(foo)`).
- [wrong_pub_self_convention](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#wrong_pub_self_convention): This is the same as
[`wrong_self_convention`](#wrong_self_convention), but for public items.
- [indexing_slicing](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#indexing_slicing): Checks for usage of indexing or slicing.
- [float_arithmetic](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#float_arithmetic): Checks for float arithmetic.
- [string_add](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#string_add): Checks for all instances of `x + _` where `x` is of type
`String`, but only if [`string_add_assign`](#string_add_assign) does *not*
match.
- [else_if_without_else](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#else_if_without_else): Checks for usage of if expressions with an `else if` branch,
but without a final `else` branch.
- [shadow_same](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#shadow_same): Checks for bindings that shadow other bindings already in
scope, while just changing reference level or mutability.
- [missing_docs_in_private_items](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#missing_docs_in_private_items): Warns if there is missing doc for any documentable item
(public or private).
- [use_debug](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#use_debug): Checks for use of `Debug` formatting. The purpose of this
lint is to catch debugging remnants.
- [mem_forget](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#mem_forget): Checks for usage of `std::mem::forget(t)` where `t` is
`Drop`.
- [unimplemented](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#unimplemented): Checks for usage of `unimplemented!`.
- [print_stdout](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#print_stdout): Checks for printing on *stdout*. The purpose of this lint
is to catch debugging remnants.
- [result_unwrap_used](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#result_unwrap_used): Checks for `.unwrap()` calls on `Result`s.
- [multiple_inherent_impl](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#multiple_inherent_impl): Checks for multiple inherent implementations of a struct
- [decimal_literal_representation](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#decimal_literal_representation): Warns if there is a better representation for a numeric literal.
- [float_cmp_const](https://rust-lang-nursery.github.io/rust-clippy/master/index.html#float_cmp_const): Checks for (in-)equality comparisons on floating-point
value and constant, except in functions called `*eq*` (which probably
implement equality for a type involving floats).


# Rationale and alternatives
[alternatives]: #alternatives

We don't particularly _need_ a 1.0, however it's good to have a milestone here, and a general idea of stability as we move forward in this process.

It's also good to have some community involvement in the lint design/categorization process since Clippy lints
both reflect and affect the general style of the community.

# Unresolved questions
[unresolved]: #unresolved-questions

Through the process of this RFC we hope to determine if there are lints which need
to be uplifted, recategorized, or removed.

