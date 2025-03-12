- Feature Name: `convert_identity`
- Start Date: 2018-01-19
- RFC PR: [rust-lang/rfcs#2306](https://github.com/rust-lang/rfcs/pull/2306)
- Rust Issue: [rust-lang/rust#53500](https://github.com/rust-lang/rust/issues/53500)

# Summary
[summary]: #summary

Adds an identity function `pub const fn identity<T>(x: T) -> T { x }`
as `core::convert::identity`. The function is also re-exported to
`std::convert::identity`.

# Motivation
[motivation]: #motivation

## The identity function is useful

While it might seem strange to have a function that just returns back the input,
there are some cases where the function is useful.

### Using `identity` to do nothing among a collection of mappers

When you have collections such as maps or arrays of mapping functions like
below and you watch to dispatch to those you sometimes need the identity
function as a way of not transforming the input. You can use the identity
function to achieve this.

```rust
// Let's assume that this and other functions do something non-trivial.
fn do_interesting_stuff(x: u32) -> u32 { .. }

// A dispatch-map of mapping functions:
let mut map = HashMap::new();
map.insert("foo", do_interesting_stuff);
map.insert("bar", other_stuff);
map.insert("baz", identity);
```

### Using `identity` as a no-op function in a conditional

This reasoning also applies to simpler yes/no dispatch as below:

```rust
let mapper = if condition { some_manipulation } else { identity };

// do more interesting stuff inbetween..

do_stuff(42);
```

### Using `identity` to concatenate an iterator of iterators

We can use the identity function to concatenate an iterator of iterators
into a single iterator.

```rust
let vec_vec = vec![vec![1, 3, 4], vec![5, 6]];
let iter_iter = vec_vec.into_iter().map(Vec::into_iter);
let concatenated = iter_iter.flat_map(identity).collect::<Vec<_>>();
assert_eq!(vec![1, 3, 4, 5, 6], concatenated);
```

While the standard library has recently added `Iterator::flatten`,
which you should use instead, to achieve the same semantics, similar situations
are likely in the wild and the `identity` function can be used in those cases.

### Using `identity` to keep the `Some` variants of an iterator of `Option<T>`

We can keep all the maybe variants by simply `iter.filter_map(identity)`.

```rust
let iter = vec![Some(1), None, Some(3)].into_iter();
let filtered = iter.filter_map(identity).collect::<Vec<_>>();
assert_eq!(vec![1, 3], filtered);
```

### To be clear that you intended to use an identity conversion

If you instead use a closure as in `|x| x` when you need an
identity conversion, it is less clear that this was intentional.
With `identity`, this intent becomes clearer.

## The `drop` function as a precedent

The `drop` function in `core::mem` is defined as `pub fn drop<T>(_x: T) { }`.
The same effect can be achieved by writing `{ _x; }`. This presents us
with a precedent that such trivial functions are considered useful and
includable inside the standard library even though they can be written easily
inside a user's crate.

## Avoiding repetition in user crates

Here are a few examples of the identity function being defined and used:

+ https://docs.rs/functils/0.0.2/functils/fn.identity.html
+ https://docs.rs/tool/0.2.0/tool/fn.id.html
+ https://github.com/hephex/api/blob/ef67b209cd88d0af40af10b4a9f3e0e61a5924da/src/lib.rs

There's a smattering of more examples. To reduce duplication,
it should be provided in the standard library as a common place it is defined.

## Precedent from other languages

There are other languages that include an identity function in
their standard libraries, among these are:

+ [Haskell](http://hackage.haskell.org/package/base-4.10.1.0/docs/Prelude.html#v:id), which also exports this to the prelude.
+ [Scala](https://www.scala-lang.org/api/current/scala/Predef$.html#identity[A](x:A):A), which also exports this to the prelude.
+ [Java](https://docs.oracle.com/javase/8/docs/api/java/util/function/Function.html#identity--), which is a widely used language.
+ [Idris](https://www.idris-lang.org/docs/1.0/prelude_doc/docs/Prelude.Basics.html), which also exports this to the prelude.
+ [Ruby](http://ruby-doc.org/core-2.5.0/Object.html#method-i-itself), which exports it to what amounts to the top type.
+ [Racket](http://docs.racket-lang.org/reference/values.html)
+ [Julia](https://docs.julialang.org/en/release-0.4/stdlib/base/#Base.identity)
+ [R](https://stat.ethz.ch/R-manual/R-devel/library/base/html/identity.html)
+ [F#](https://msdn.microsoft.com/en-us/visualfsharpdocs/conceptual/operators.id%5B%27t%5D-function-%5Bfsharp%5D)
+ [Clojure](https://clojuredocs.org/clojure.core/identity)
+ [Agda](http://www.cse.chalmers.se/~nad/repos/lib/src/Function.agda)
+ [Elm](http://package.elm-lang.org/packages/elm-lang/core/latest/Basics#identity)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

An identity function is a mapping of one type onto itself such that the output
is the same as the input. In other words, a function `identity : T -> T` for
some type `T` defined as `identity(x) = x`. This RFC adds such a function for
all `Sized` types in Rust into libcore at the module `core::convert` and
defines it as:

```rust
pub const fn identity<T>(x: T) -> T { x }
```

This function is also re-exported to `std::convert::identity`.

It is important to note that the input `x` passed to the function is
moved since Rust uses move semantics by default.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

An identity function defined as `pub const fn identity<T>(x: T) -> T { x }`
exists as `core::convert::identity`. The function is also re-exported as
`std::convert::identity`-

Note that the identity function is not always equivalent to a closure
such as `|x| x` since the closure may coerce `x` into a different type
while the identity function never changes the type.

# Drawbacks
[drawbacks]: #drawbacks

It is already possible to do this in user code by:

+ using an identity closure: `|x| x`.
+ writing the `identity` function as defined in the RFC yourself.

These are contrasted with the [motivation] for including the function
in the standard library.

# Rationale and alternatives
[alternatives]: #alternatives

The rationale for including this in `convert` and not `mem` is that the
former generally deals with conversions, and identity conversion" is a used
phrase. Meanwhile, `mem` does not relate to `identity` other than that both
deal with move semantics. Therefore, `convert` is the better choice. Including
it in `mem` is still an alternative, but as explained, it isn't fitting.

Naming the function `id` instead of `identity` is a possibility.
This name is however ambiguous with *"identifier"* and less clear
wherefore `identifier` was opted for.

# Unresolved questions
[unresolved]: #unresolved-questions

There are no unresolved questions.

# Possible future work

A previous iteration of this RFC proposed that the `identity` function
should be added to prelude of both libcore and libstd.
However, the library team decided that for the time being, it was not sold on
this inclusion. As we gain usage experience with using this function,
it is possible to revisit this in the future if the team chances its mind.

The section below details, for posterity,
the argument for inclusion that was previously in the [motivation].

## The case for inclusion in the prelude

Let's compare the effort required, assuming that each letter
typed has a uniform cost with respect to effort.

```rust
use std::convert::identity; iter.filter_map(identity)

fn identity<T>(x: T) -> T { x } iter.filter_map(identity)

iter.filter_map(::std::convert::identity)

iter.filter_map(identity)
```

Comparing the length of these lines, we see that there's not much difference in
length when defining the function yourself or when importing or using an absolute
path. But the prelude-using variant is considerably shorter. To encourage the
use of the function, exporting to the prelude is therefore a good idea.

In addition, there's an argument to be made from similarity to other things in
`core::convert` as well as `drop` all of which are in the prelude. This is
especially relevant in the case of `drop` which is also a trivial function.
