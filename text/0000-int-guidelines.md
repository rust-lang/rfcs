- Start Date: 2015-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC proposes usage guidelines for the various integer types.

# Motivation

The goal of this RFC is to help people decide what integer types to use when they need
to make a decision for a new API.

It builds on [https://github.com/rust-lang/rfcs/pull/560](the integer overflow RFC),
which provides debug-time assertions for overflow and underflow. One of the goals
of this RFC is to provide guidance for when to use unsigned types, and these
assertions affect the traditional tradeoffs about unsigned types, which are described
under the detailed design below.

It also draws inspiration from the [Google C++ Style Guide](http://google-styleguide.googlecode.com/svn/trunk/cppguide.html#Integer_Types)

This RFC attempts to balance several concerns:

* Rust developers should have an easy, go-to heuristic for deciding what
  integer sizes to use when they need to use a number type.
* 32-bit integers are considerably faster than 64-bit integers in some
  situations, so using 64-bit integers for tiny numbers, especially in
  hot code, can result in unnecessarily slower programs.
* Reflexive use of 32-bit integers too-often results in overflows when the
  numbers aren't expected to be "laughably smaller" than the maximum
  32-bit number.
* Occasional use of `usize` when building a brand new data structure may
  be appropriate, but use of `usize` in general can introduce portability
  hazards when the use-case is not proportional to the amount of
  addressable memory.
* Using of unsigned integers is traditionally thought to be error-prone, and
  style guides often suggest avoiding them. That said, Rust's unsigned integers
  have built-in underflow assertions, which changes the analysis.

# Detailed design

When deciding what integer type to use for a given situation, you can make the decision
to a first approximation through this heuristic:

* If you expect all uses of this API to use numbers "laughably smaller" then 32-bits,
  use 32-bit numbers. Otherwise, prefer 64-bit numbers.
* If you are sure that the number will never be less than 0, use unsigned integers.
  Otherwise, use signed numbers.
* If you are building a new data structure, and your number refers to the size of memory,
  you may want to use `usize`, which is described below.

> Note: Rust does not yet have BigInts in `std`, which might, in theory, be a better
> go-to big integer type. These guidelines will probably be revised in the future
> once that changes.

## Signed vs. Unsigned Integers

Traditionally, style guides for low-level languages have [warned against the use
of unsigned integers](http://google-styleguide.googlecode.com/svn/trunk/cppguide.html#Integer_Types) because
they can introduce bugs when comparing unsigned value with 0
(`for (unsigned int i = foo.len() - 1; i >= 0; --i) ...`). The Google Style Guide
argues that instead of unsigned integers, programmers should use signed
integers and assertions that the value does not go below 0.

In Rust, unsigned integers have underflow checking assertions built-into
the type (assuming that RFC XXX is accepted), so using a `u32` is equivalent
to the advice in Google style guide (with a larger maximum value).

## The "size" Types

The `isize` and `usize` types represent numbers that are proportional to
the size of available memory. Most normal uses of `isize` and `usize`
should arise through the use of existing data structures that expose
values with those types.

For example, if a new structure wanted to store the length of a `Vec` in
one of its fields, it should store it as a `usize`, because that's the return
value of `vec.len()`.

When building entirely new low-level data structures, you may want to use
`usize` to represent a value (e.g. node id) that scales linearly with the amount
of addressable memory.

## Mixing and Matching (Casting)

You may occasionally encounter a situation where the integer types provided
by one API does not match your own storage for the value or the integer
type taken by another API.

If you are working with your own storage, you should change your own
internal storage to match the value you have received. For example, if
you have a field that stores a number of nanoseconds as a `u32`, and
`precise_time_nanos` returns a time as `u64`, you should change your
field to a `u64`. In this situation, you should avoid casting the value.

If you are passing a value provided by one API into another API, is it
generally safe to cast a **smaller** sized value into a **larger** sized
value.

For example, it is generally safe to cast an `i32` to an `i64`. It is also
generally safe to case a `usize` to a `u64`, since it will never be bigger
than `u64`.

However, you should avoid **truncating** casts, which cast a larger sized
integer to a smaller sized integer. You should also avoid casting between
signed and unsigned integers (in either direction).

If you need to truncate a number, or cast between signed and unsigned
types, you should carefully consider what will happen if the source number
is outside the bounds of the target type, and handle that case explicitly.

# Drawbacks

The primary drawback of this approach is the same as the drawback of
not having a default integer type in the first place: unnecessary
incompatibilities between APIs that chose different sides of the API
tradeoff.

In today's Rust, a library or program that chooses to use `u32`
in "laughably small" cases would be incompatible with libraries
that used the more conservative `u64`. This will result in more
casting overall, likely making people less concerned when they
perform more dangerous, truncating casts.

We have previously discussed the possibility of automatically
performing "widening" casts (`u32` to `u64`). This would allow
libraries to freely be as conservative as they want, and
reserve casts for dangerous situations that truly require
thought.

# Alternatives

We could decide not to issue guidance here, instead relying
on sub-ecosystems in Rust to make decisions appropriate to
their domains. The drawback of that approach is that developers
new to Rust will lack the benefit of any heuristic, and will
be more likely to cargo-cult solutions.

We could decide to encourage the use of `u64` even for small
integers. While this would have the benefit of being
significantly simpler, it would also mean that programs using
tiny numbers would be unnecessarily slow. Because of this,
it is doubtful that many people would even follow such a
guideline, making it a dead letter and being practically
similar to having no guideline at all.

We could also encourage the use of `u32` as a default integer,
discouraging the use of `u64` unless the user has a specific
reason to believe the number would exceed `u32`. This has
two problems.

1. This guideline doesn't work very well in libraries,
   which don't usually have a very concrete understanding
   of what integers they will be used with.
2. We want to discourage the use of `u32` in situations
   where 32-bit overflow is a possibility. The
   "laughably smaller" guideline, originally proposed by
   @Valloric, captures most of the performance-critical
   cases involving small numbers without reintroducing
   significant opportunities for 32-bit overflows.

# Unresolved questions

Should we implement automatic widening coercions?
