- Start Date: 2014-07-11
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Either rename the types `int` and `uint` to `index` and `uindex` to avoid
misconceptions and misuses, or specify that they're always at least 32-bits wide
to avoid the worst portability problems. Also document when to use and not use
these types and which integer type to pick "by default." (See below for the
meaning of "by default.")


# Motivation

So Rust libraries won't have new overflow bugs when run on embedded devices with
16-bit addressing, ditto for code written for 64-bit addressing then run in
32-bit environments. Rust is a very compelling replacement for C/C++ in embedded
devices, "Internet of Things" devices, and safety-critical robotics actuators.

So programmers will know when to use which integer types.


# Background

Rust defines types `int` and `uint` as integers that are wide enough to hold a
pointer. The language uses them for array indexes since `uint` is large enough
to index into any memory array and `int` is useful for a difference between two
pointers or array indexes.

(A rationale given for using unsigned array indexes is to allow array bounds
checking in one comparison rather than two. However, a compiler can generate one
unsigned comparison to bounds-check a signed integer index as long as the lower
bound is 0.)

`int`/`uint` can also good choices for indexing and sizing in-memory containers.

The point of an integer type that depends on the target address space is to give
the same code compact array indexes in small-address targets while supporting
huge arrays in large-address targets. But using these types for computations
that are not limited by addressing leads to code that's not portable to
smaller-address targets than it was developed and tested on.

From decades of C/C++ experience, programmers have learned to pick `int`/`uint`
as the "default" integer types where not particularly constrained by
requirements, e.g.:

  * where any modest-size integer will do (e.g. a loop index)
  * to avoid cluttering APIs with ungermane integer sizes
  * to hold tags and other values supplied by callers

(Java programmers are also accustomed to `int` as the default integer type, but
a Java `int` is always 32-bits.)

Programmers should figure out a value's needed integer range then maybe widen to
a "default" type for easy interconnections. For a value in the range 1 .. 100,
you can pick from 10 types. Choosing an 8-bit or 16-bit integer is an
optimization. Premature? Which integer type should you pick when you're writing
exploratory code and haven't yet done the range analysis? What if you're passing
values through many layers of code and computations?

A default is handy but a target-dependent size does not make a good default. And
yet `int` and `uint` _look_ like default integer types.

To clear up some misconceptions from C/C++:

  * _They're not the fastest integers._ Example: x86_64 and ARM64 have 64-bit address spaces and 64-bit integer registers, but 32-bit integers are faster since those arithmetic instructions are faster, more data fits in cache, and the vector instructions can operate on twice as many values at a time.
  * _They're not "native" size or register size._ Example: The [x32 ABI](https://en.wikipedia.org/wiki/X32_ABI) has 64-bit general purpose registers but 32-bit pointers so its `int`/`uint` are 32-bit.
  * _They're not necessarily the same size as C `int`._ C doesn't define `int` the same way.
  * _They're not wide enough to casually pick,_ given 16-bit address spaces.
  * _They're not "portable."_ They overflow differently and take different numbers of binary I/O bytes on different platforms.

These misconceptions lead to misuses and thus to code with overflow bugs
(checked or unchecked) when running in a smaller address space than originally
considered and tested.

The worst failure mode is in libraries written with desktop CPUs in mind and
then used in small embedded devices.


# Detailed design

Change these two type names so they're self-documenting and less misused. The
names `index` and `uindex` are meant to convey their use in array indexing. Use
them more narrowly.

Alternate name choices:

  - `isize` and `usize`, [borrowing from C's](http://en.cppreference.com/w/cpp/types/integer) `ssize_t` and `size_t` but adopting Rust's integer prefixes.
  - `intptr` and `uintptr`, [borrowing from C's](http://en.cppreference.com/w/cpp/types/integer) `intptr_t` and `uintptr_t`. These names are awkward by design.
  - `PointerSizedInt` and `PointerSizedUInt`.
  - `intps` and `uintps`.

To ease the transition, first deprecate the old types.

**Alternative:** specify that these two integer types are _at least 32-bits
wide_ on every target architecture. That avoids the worst failure mode although
it doesn't help when code tested in a 64-bit address space later runs in a
32-bit address space.

**Either way:** The style guide should document when to use and not use these
types and elect a particular integer type for programmers to pick "by default".
This RFC recommends `i32`.

The style guide should also recommend using signed integers except when unsigned values are required such as for modulo 2^N arithmetic. The
[Google Style Guide](http://google-styleguide.googlecode.com/svn/trunk/cppguide.xml#Integer_Types) explains:

> In particular, do not use unsigned types to say a number will never be negative. Instead, use assertions for this. ...
>
> Some people, including some textbook authors, recommend using unsigned types to represent numbers that are never negative. This is intended as a form of self-documentation. However, in C, the advantages of such documentation are outweighed by the real bugs it can introduce.

Furthermore:

> You should assume that an `int` is at least 32 bits, but don't assume that it has more than 32 bits.

This assumption does not hold for PalmOS even on 32-bit ARM, where `int` is
16-bits for backward compatibility with PalmOS running on Motorola 68000.


# Drawbacks

  - Renaming `int`/`uint` requires figuring out which of the current uses to replace with `index`/`uindex` vs. `i32`/`u32`/`BigInt`.
  - The new names are more verbose.


# Alternatives

  1. Set a coding style guide and code review expectation to use `int`/`uint` only for array indexing and related operations despite C programmers' expectations. Elect an integer type such as `i32` to use "by default." Update the existing libraries.
  2. Fix the portability bugs later.


# Notes

See the discussions from many contributors to [Issue #14758](https://github.com/rust-lang/rust/issues/14758) and [Issue #9940](https://github.com/rust-lang/rust/issues/9940).

Also see [Issue #11831](https://github.com/rust-lang/rust/issues/11831) about
keeping pointer sized integers as the default. If people are happy with that
choice, then this RFC is about making `int`/`uint` at least 32-bits wide and
setting style guidelines for integer types.

[Daniel Micay notes](https://github.com/rust-lang/rust/issues/9940#issuecomment-32104831):

> If you're using `int` as a "default", then you're not using it correctly. It
> will be 16-bit on an architecture with a 16-bit address space, 32-bit or
> 64-bit. If you're doing your testing on a 64-bit architecture, you're going to
> miss plenty of bugs.

[Carter Tazio notes](https://github.com/rust-lang/rust/issues/9940#issuecomment-32088729)
that system-dependent integers in GHC Haskell cause recurring problems, and
there's some buy-in for fixing it.

[Niko Matsakis requested](https://github.com/rust-lang/rust/issues/9940#issuecomment-32119318)
a survey of uses of `int` and `uint` showing how many of them are
appropriate / inappropriate / borderline.

More recently, [type inference no longer falls back to `int`/`uint`](https://github.com/rust-lang/rust/issues/6023) and there's an RFC for
[Scoped attributes for checked arithmetic](https://github.com/rust-lang/rfcs/pull/146).


# Not in scope of this RFC

Changes in overflow handling.


# Unresolved questions

Who'll implement the changes?
