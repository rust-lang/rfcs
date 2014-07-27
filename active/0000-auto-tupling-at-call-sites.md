- Start Date: 2014-07-16
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Automatically turn excess arguments at a given call site into a tuple
of the final argument and the excess arguments.  Automatically turn an
omitted argument at a given call site into unit (`()`).
transformations (which I am calling "auto-tupling") can be used in
tandem with the trait system as a way to express optional parameters
and multiple-arity functions.

# Motivation

People have been asking for optional arguments for a while,

* On the mailing list: [Polymorphism & default parameters in rust](https://mail.mozilla.org/pipermail/rust-dev/2012-August/002228.html)

* On the rust repo: [Default arguments and keyword arguments](https://github.com/rust-lang/rust/issues/6973)

* On the RFC repo: [optional parameters](https://github.com/rust-lang/rfcs/pull/152), [Arity-based parameter overloading](https://github.com/rust-lang/rfcs/pull/153)

Auto-tupling at the call site provides a clean syntax for defining
functions that support a variety of calling protocols: you make the
last argument for the function a trait, and then implement the trait
for every combination of tuple that you want to support.

This strategy supports optional arguments and arity-based overloading
for statically-dispatched call sites.

At the same time, it is a relatively simple change to the language:
nothing changes about function definitions nor about the calling
convention; it is just a local transformation on each call-site where
the number of actual arguments does not match the number of formal
parameters.

The expected outcome is that we reap many of the benefits already
associated with optional arguments and arity-based overloading,
assuming that the standard library is revised to make full use of the
feature.

# Detailed design

For any function F, if the following two conditions hold for its definition:
* F is defined as taking `k+1` arguments, and
* F where the final formal argument to the function is some generic type parameter,
then at all of the call sites for F, it can be passed any number of
arguments >= `k`.

When F is passed `k` arguments, then the missing final `k+1`'th
argument is automatically inserted as the unit value `()`.

When F is passed `k+1` arguments, then everything operates the same as
today (i.e. this RFC has no effect on it).

When F is passed `k+j` arguments, then the final `j` arguments are
converted into a tuple of length `j`.

The rest of the compilation procedes as normal.

In the common case, the final argument to F will have one or more
trait bounds, and the call sites will be expected to pass a set of
arguments whose auto-tupling is compatible with those trait bounds.
That is how we get all the way to enforcing a strict protocol on what
the optional arguments are, or what multiple arities of F are.

Note: The strategy of this RFC does not work for closures and dynamic
dispatch because closures are monomorphic and object methods cannot
have generic type parameters.  I deem this an acceptable price to pay
to keep the language change simple: (In general, supporting a
combination of optional arguments and dynamic dispatch would require
some way of communicating the type and number of parameters from the
call-site to the method definition.)

As a concrete example, assume the following definition (where nothing
new from this RFC is being used):

```rust
fn foo<T:FooArgs>(required_x: int, rest: T) -> int {
    required_ + rest.y() + rest.z()
}

trait FooArgs {
    fn y(&self) -> int;
    fn z(&self) -> int;
}

impl FooArgs for () {
    fn y(&self) -> int { 0 }
    fn z(&self) -> int { 0 }
}

impl FooArgs for int {
    fn y(&self) -> int { *self }
    fn z(&self) -> int { 0 }
}

impl FooArgs for (int, int) {
    fn y(&self) -> int { self.val0() }
    fn z(&self) -> int { self.val1() }
}
```

Under this RFC, here are some legal expressions:
```rust
foo(1)       // expands to foo(1, ())
foo(1, 2)    // expands to foo(1, 2)
foo(1, 2, 3) // expands to foo(1, (2, 3))
```

This illustrates how one expresses optional arguments for `foo` under
this RFC.

As another example, the GLM library for C++ defines
`vec2`/`vec3`/`vec4` structures that define vectors of 2/3/4 numeric
components, respectively.  The constructors provided in GLM for `vecN`
(for `N` in {2,3,4}) include both a unary and `N`-ary variant: the
unary variant copies its input argument to all `N` members, and the
`N`-ary variant copies each of the inputs to the corresponding member.

Without this RFC, one can emulate this in Rust via tuples:
```rust
fn vec4<A:Vec4Args>(a: A) -> Vec4 {
    Vec4{ x: a.x(), y: a.y(), z: a.z(), w: a.w() }
}

impl Vec4Args for f32 {
    fn x(&self) -> f32 { *self }
    fn y(&self) -> f32 { *self }
    fn z(&self) -> f32 { *self }
    fn w(&self) -> f32 { *self }
}

impl Vec4Args for (f32,f32,f32,f32) {
    fn x(&self) -> f32 { self.val1() }
    fn y(&self) -> f32 { self.val2() }
    fn z(&self) -> f32 { self.val3() }
    fn w(&self) -> f32 { self.val0() }
}

vec4(9.0f32)                           // ==> Vec4{ x: 9.0, y: 9.0, z: 9.0, w: 9.0 }
vec4((1.0f32, 2.0f32, 3.0f32, 4.0f32)) // ==> Vec4{ x: 1.0, y: 2.0, z: 3.0, w: 4.0 }
```

But with this RFC in place, the syntax for the last line becomes a bit nicer:
```rust
vec4(1.0f32, 2.0f32, 3.0f32, 4.0f32)   // ==> Vec4{ x: 1.0, y: 2.0, z: 3.0, w: 4.0 }
```

The two examples above followed a general rule of treating the trait
as a bundle of all of the remaining arguments.  However, the scheme of
this RFC can also express multiple-arity dispatch, where one may want
a function to have two different behaviors depending on the arguments
passed at the call-site.  The way you do this: just make the trait
implementation itself hold the bulk of the function's behavior, rather
than the function body, which just dispatches off to the trait.

So as an example:
```rust
fn print_report<P:ReportPrinter>(report: &Report, output: P) {
    output.print_it(report)
}

impl ReportPrinter for () {
    fn print_it(&self) { /* just print to stdout */ }
}

impl ReportPrinter for std::io::File {
    fn print_it(&self) { /* print to the file*/ }
}

impl ReportPrinter for gui::Window {
    fn print_it(&self) { /* print to a text area in the window */ }
}


The design philosophy espoused by this RFC allows for client code to
add new instances of the arguments trait.  As a concrete example, in
the previous example of `ReportPrinter`, its entirely possible that
the code for `impl ReportPrinter for gui::Window` lives in the crate
that defines `gui::Window`, rather than the crate that defines `fn
print_report`.  (Of course it falls upon the author of the
`ReportPrinter` trait to document its API well-enough to support such
usage, if that is desired.)

# Drawbacks

* Some people may prefer explicit sugar on the function definition to
  indicate optional arguments and/or argument-based dispatch, rather
  than indirectly expressing it via a trait.  So adopting auto-tupling
  may not satisfy such persons' desire for so-called "true" optional
  arguments.

  * As a concrete example of why one might prefer baked-in support:
    rustdoc would not show you the various potential arguments with
    which one might invoke the function.

* Auto-tupling may delay the reporting of legitimate errors.
  Reporting errors as eagerly as possible is the reason I included the
  condition that the final formal argument to the function be some
  generic type parameter, but obviously that still does not catch
  the case where one e.g. invokes `vec4(1.0f32, 2.0f32)`, which would
  expand into `vec4((1.0f32, 2.0f32))` and lead to an error like:
  "error: failed to find an implementation of trait Vec4Args for (f32,f32)";
  Presumably the rustc compiler can be adapted to report a better
  error message when a tuple has been introduced by auto-tupling.

# Alternatives

We can choose to not add any support for optional arguments at all.
We have been getting by without them.

We can add a more complex protocol for supporting optional arguments
that includes changes at the function definition site (and potentially
the calling convention, depending on how extreme you want to be).  The
main reason I could see for going down that path is to support
optional arguments on closures and object methods.

# Unresolved questions

None yet.
