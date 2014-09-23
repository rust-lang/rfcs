- Start Date: 2014-09-23
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add support for default and variable-arity functions via a small
language feature and a simple programming pattern.
The language feature is solely a pair of changes to the treatment of
function call-sites:

* If there are excess arguments at a given call site, then
  automatically turn convert excess arguments into a tuple of the
  final argument and the excess arguments.  I call this "auto-tupling."
* If there are insufficient arguments at a given call site,
  then automatically replace each of the omitted arugments with
  the unit value (`()`).  I call this "auto-unit'ing." [*]

The above two transformations can be used in tandem with the trait
system as a way to express optional parameters and multiple-arity
functions.

[*] Note that "auto-unit'ing" is pronounced with a short "i", like
    "pit" (or "unit"), as opposed to "auto-uniting", which would
    be pronounced with a long "i", like "pie" (or "unite").

# Motivation

People have been asking for optional arguments for a while,

* On the mailing list: [Polymorphism & default parameters in rust](https://mail.mozilla.org/pipermail/rust-dev/2012-August/002228.html)

* On the rust repo: [Default arguments and keyword arguments](https://github.com/rust-lang/rust/issues/6973)

* On the RFC repo: [optional parameters](https://github.com/rust-lang/rfcs/pull/152), [Arity-based parameter overloading](https://github.com/rust-lang/rfcs/pull/153), [Default and positional arguments](https://github.com/rust-lang/rfcs/pull/257).

Auto-tupling at the call site provides a clean syntax for defining
functions that support a variety of calling protocols: you make the
last argument for the function a trait, and then implement the trait
for every combination of tuple that you want to support.

Meanwhile, auto-unit'ing provides a clean way to describe trailing
optional arguments, resolved by their position (as opposed to some
sort of name-based scheme): You make a trait for each optional
argument, and then you provide an implementation of the trait for both
`()` and for the actual kind of value you expect to be provided.

Therefore, these features support optional arguments and arity-based
overloading for statically-dispatched call sites.

At the same time, it is a relatively simple change to the language:
nothing changes about function definitions nor about the calling
convention. It is just a local transformation on each call-site where
the number of actual arguments does not match the number of formal
parameters.

In addition, the programming pattern espoused by this RFC leverages
our existing trait infrastructure.  This is important because it means
that client code of some function using optional arguments or
variable-arity functions is likely to have more options available to
it, since it will be free to implement the provided traits in whatever
manner it likes, without being beholden to the particular protocol
originally envisaged by the library developer.


The expected outcome is that we reap many of the benefits already
associated with optional arguments and arity-based overloading,
assuming that the standard library is revised to make full use of the
feature.

# Detailed design

## Preliminary Definitions

Let us define a "parametric" formal argument to a function as
any formal argument that has as its type, some type parameter to the
function, trait, or impl.  So for example:

```
type V = char;
trait Foo<W> {
    type X;
    fn bar<Y>(t: Option<Y>, u: u8, v: V, w: W, x: X, y: Y);
}
```

the formal arguments to `Foo::bar` are {`t`, `u`, `v`, `w`, `x`, `y`},
and the parametric ones are `w` and `y`.

(Note that even though the type of `t`, `Option<Y>`, contains the type
parameter `Y`, `t` itself is not considered parametric under the
definition of this RFC.  I know this may be unintuitive; suggestions
for improvements to this terminology are welcome!)

Continuing with the above example:
```rust
struct Baz<Z> { ... }

impl<A> Foo<A> for Baz<A> {
    type X = A;
    fn bar<Y>(t: Option<Y>, u: u8, v: V, w: A, x: A, y: Y) { ... }
}
```

Likewise in this case, the formal arguments to `<Baz as Foo>::bar` are
again {`t`, `u`, `v`, `w`, `x`, `y`}; and now the parametric arguments
are `w`, `x`, and `y`, since the first two are typed by the type
parameter `A` and the third by the type parameter `Y`.

For any function F, let "F has trailing parametric formals" mean that
F, in its function definition, takes i+j formal parameters, for some
non-negative i and positive j, where the i is chosen to be the last
non-parametric formal argument to F (and thus i is zero if and only if
F has *only* parametric formals), and j is the number of trailing
parametric formals in F.

Thus, in the following:
```rust
fn no_trailing<X>(x0: X, x1: X, y: int) { ... }
fn trailing_1<X>(w: int, x: X) { ... }
fn trailing_2<Y,Z>(w: int, y: Y, z: Z) { ... }
fn trailing_3<Y,Z>(y: Y, z: Z) { ... }
```

the `no_trailing` function does not have trailing parametric formals,
while both `trailing_1`, `trailing_2`, and `trailing_3` have trailing
parametric formals (`x` in the first and {`y`,`z`} in the latter two).
For `trailing_1`, i=1 and j=1; for `trailing_2`, i=1 and j=2; and for
`trailing_3`, i=0 and j=2.

## Call Site Overloading

For any F that has trailing parametric formals (and thus has i
"normal" arguments plus j trailing parametric formals), any call to F
is allowed to take any number of actual arguments greater than or
equal to i.

Let the number of actual arguments be denoted by k.

When F is passed k = i+j expressions as actual arguments, then
everything operates the same as today (i.e. this RFC has no effect on
it).

When k < i+j, then every missing argument expression is filled in with
the unit value `()`.

When k > i+j, then the j'th, j+1'th, ... k'th argument expressions
(here denoted `e_j, e_j+1, ... e_k`) are all replaced with a single
tuple expression: `(e_j, e_j+1, ..., e_k)`.

The rest of the compilation procedes as normal.  In particular, either
the rewritten code will work because the generic type(s) in question
are compatible with either unit (in the k < i+j case) or with the
tuple expression `(e_j, e_j+1, ..., e_k)`.

In the common case, the final argument to F will have one or more
trait bounds, and the call sites will be expected to pass a set of
arguments whose auto-tupling is compatible with those trait bounds.
That is how we get all the way to enforcing a strict protocol on what
the optional arguments are, or what multiple arities of F are.

Note: The strategy of this RFC does not work in general for closures
and dynamic dispatch because closures are monomorphic and object
methods cannot have generic type parameters.  I deem this an
acceptable price to pay to keep the language change simple: (In
general, supporting a combination of optional arguments and dynamic
dispatch would require some way of communicating the type and number
of parameters from the call-site to the method definition.)

As a concrete example, assume the following definition (where nothing
new from this RFC is being used):

```rust
fn foo<T:FooArgs>(required_x: int, rest: T) -> int {
    required_x + rest.y() + rest.z()
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

Finally, as an example that uses auto-tupl'ing for optional arguments,
which also illustrates how encouraging this programming pattern can
provide extra flexibility to client code (assuming the system is
appropriately architected).

The two examples above followed a general rule of treating the trait
as a bundle of all of the remaining arguments.  However, the scheme of
this RFC can also express multiple-arity dispatch, where one may want
a function to have two different behaviors depending on the arguments
passed at the call-site.  The way you do this: just make the trait
implementation itself hold the bulk of the function's behavior, rather
than the function body, which just dispatches off to the trait.

So as an example:
```rust
fn print_report<F:ReportFormat,P:ReportPrinter>(report: &Report, format: F, output: P) {
    output.print_it(report, format.date(), format.currency());
}

trait ReportPrinter {
    fn print_it(&self, report: &Report, date: DateFormat, decimal: DecimalFormat);
}

trait ReportFormat {
    fn date(&self) -> DateFormat;
    fn decimal(&self) -> DecimalFormat;
}

impl ReportFormat for () {
    fn date(&self) -> DateFormat { MM_DD_YYYY_IS_SO_GR8 }
    fn decimal(&self) -> DecimalFormat { NUMBER_DOT_FRACTION }
}

struct Format { date: DateFormat, decimal: DecimalFormat }
impl ReportFormat for Format {
    fn date(&self) -> DateFormat { self.date }
    fn decimal(&self) -> DecimalFormat { self.decimal }
}

impl ReportPrinter for () {
    fn print_it(&self, report: &Report, date: DateFormat, decimal: DecimalFormat) {
        /* just print to stdout */
    }
}

impl<'a> ReportPrinter for &'a std::io::File {
    fn print_it(&self, report: &Report, date: DateFormat, decimal: DecimalFormat) {
        /* print to the file */
    }
}

impl<'a> ReportPrinter for &'a gui::Window {
    fn print_it(&self, report: &Report, date: DateFormat, decimal: DecimalFormat) {
        /* print to an html formatted box in the window */
    }
}

let the_report = ...;
print_report(&the_report); // prints to stdout with the default formatting
let the_file : std::io::File = ...;
let the_format = Format { date: YYYY_MM_DD, decimal: NUMBER_COMMA_FRACTION };
print_report(&the_report, the_format, &the_file);
```

The design philosophy espoused by this RFC allows for client code to
add new instances of the arguments trait.  As a concrete example, in
the previous example of `ReportPrinter`, its entirely possible that
the code for `impl<'a> ReportPrinter for &'a gui::Window` lives in the
crate that defines `gui::Window`, rather than the crate that defines
`fn print_report`.  (Of course it falls upon the author of the
`ReportPrinter` trait to document its API well-enough to support such
usage, if that is desired.)

# Drawbacks

* Some people may prefer explicit syntax on the function definition
  site to indicate optional arguments and/or argument-based dispatch,
  rather than indirectly expressing it via a trait.  So adopting
  auto-tupling may not satisfy such persons' desire for so-called
  "true" optional arguments.

  * As a concrete example of why one might prefer baked-in support: I
    do not think rustdoc would show you the various potential argument
    types with which one might invoke the function, (unless we revise
    rustdoc to be smarter about these cases).  This is a bit of a
    strawman example, since rustdoc would need to be revised in
    response to *any* change we make to support optional arguments
    and/or variable-arity functions.

* Related to the previous bullet: Auto-tupling may hide errors entirely

  Since auto-tupling requires no change in syntax at the function
  definiton site, it is conceivable that someone unaware of the
  auto-tupling rule in the language could hypothetically
  make the following change from version 1 to version 2 of `foo`:
  ```
  #[cfg(version_1)]
  fn foo<Z>(x: int, y: int, z: Z) { ... }

  #[cfg(version_2)]
  fn foo<Z>(x: int, z: Z) { ... }
  ```

  and then be upset that the old call `foo(1, 2, 3)` is not flagged as
  a mismatched argument count error.

  I see this as a small risk, or at least a reasonable price to retain
  overall language simplicity.

* Auto-tupling may delay the reporting of legitimate errors.
  Reporting errors as eagerly as possible is the reason I included the
  condition that only functions with trailing parametric formals be
  subject to the call-site rewriting, but obviously that still does not catch
  the case where one e.g. invokes `vec4(1.0f32, 2.0f32)`, which would
  expand into `vec4((1.0f32, 2.0f32))` and lead to an error like:
  "error: failed to find an implementation of trait Vec4Args for (f32,f32)";
  Presumably the rustc compiler can be adapted to report a better
  error message when a tuple has been introduced by auto-tupling
  and then fails to actually implement the required trait.

# Alternatives

## Status quo

We can choose to not add any support for optional arguments at all.
We have been getting by without them.

## Look beyond the call-site

We can add a more complex protocol for supporting optional arguments
that includes changes at the function definition site (and potentially
the calling convention, depending on how extreme you want to be).  The
main reason I could see for going down that path is to support
optional arguments on closures and object methods.

  * Another reason to go down this road woudl be to support
    non-trailing optional arguments (though I personally prefer
    optional arguments be restricted to trailing).

I think many of the other proposals for optional and/or keyword
arguments and/or variable arity functions have gone down this road; I
am explicitly trying to avoid it.

## Generalized auto-tupling alone

My original proposal that I posted to [discuss] did not have
"auto-unit'ing".  Instead it used a more general notion of
auto-tupling, where all omitted arguments where replaced with a single
unit `()` value.  While this orignally appealed to me, "auto-unit'ing"
allow for clean code in many cases.

[discuss]: http://discuss.rust-lang.org/t/pre-rfc-auto-tupling-at-call-sites/175/

The main reason I was considering generalized auto-tupling was to
enable client-side flexibility in more cases (i.e. in the
`print_report` example above, under generalized auto-tupling, both the
`format` and `output` arguments to `print_report` would forced to be
carried in a single paramteric formal argument at the end of the
argument list, and thus client code would be able to freely override
the protocol for how either is handled).  However, I think this
motivation seems relatively weak (since it requires much foresight on
the part of the library designer and also much ingenuity on the part
of the client), so I was willing to forego this approach and instead
propose auto-unit'ing as a cleaner alternative.


# Unresolved questions

In the first example from the Detailed Design, the formal parameter
`x` was non-parametric in `trait Foo` (since it was an
associated type, not a type parameter that could be freely chosen),
while in the `impl<A> Foo<A> for Baz<A>`, the formal parameter `x` was
now the type parameter `A` and thus `x` is parametric.  (I am
assuming such an `impl` is a legal use of the associated
`type X = A;` in the impl.)

What if I had written instead:
```
fn bar<Y>(&self, u: u8, v: V, w: A, x: X, y: Y) { ... }
                                    ^~~~
```
where now `x` again has type `X` instead of `A`?  Would we still treat
that as being a parametric, since in this case, `X` resolves to `A`?

In other words, what can we conclude about the legality of
```rust
<Baz as Foo>::bar(1u8, '2');
```
in the latter scenario?  Would it rewrite to:
```rust
<Baz as Foo>::bar(1u8, '2', (), (), ());
```
or would it be flagged as an error by the compiler?

None yet.
