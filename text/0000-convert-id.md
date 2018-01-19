- Feature Name: convert_id
- Start Date: 2018-01-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Adds an identity function `pub fn id<T>(x: T) -> T { x }` as `core::convert::id`.
The function is also re-exported to `std::convert::id` as well as the prelude of
both libcore and libstd.

# Motivation
[motivation]: #motivation

## The identity function is useful

While it might seem strange to have a function that just returns back the input,
there are some cases where the function is useful.

### Using `id` to do nothing among a collection of mappers

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
map.insert("baz", id);
```

### Using `id` as a no-op function in a conditional

This reasoning also applies to simpler yes/no dispatch as below:

```rust
let mapper = if condition { some_manipulation } else { id };

// do more interesting stuff inbetween..

do_stuff(42);
```

### Using `id` to concatenate an iterator of iterators

Given the law `join = (>>= id)`, we use the identity function to perform
a monadic join on iterators in this example.

```rust
let vec_vec = vec![vec![1, 3, 4], vec![5, 6]];
let iter_iter = vec_vec.into_iter().map(Vec::into_iter);
let concatenated = iter_iter.flat_map(id).collect::<Vec<_>>();
assert_eq!(vec![1, 3, 4, 5, 6], concatenated);
```

### Using `id` to keep the `Some` variants of an iterator of `Option<T>`

We can keep all the maybe variants by simply `iter.filter_map(id)`.

```rust
let iter = vec![Some(1), None, Some(3)].into_iter();
let filtered = iter.filter_map(id).collect::<Vec<_>>();
assert_eq!(vec![1, 3], filtered);
```

### To be clear that you intended to use an identity conversion

If you instead use a closure as in `|x| x` when you need an
identity conversion, it is less clear that this was intentional.
With `id`, this intent becomes clearer.

## The `drop` function as a precedent

The `drop` function in `core::mem` is defined as `pub fn drop<T>(_x: T) { }`.
The same effect can be achieved by writing `{ _x; }`. This presents us
with a precendent that such trivial functions are considered useful and
includable inside the standard library even tho they can be written easily
inside a user's crate.

## Avoiding repetition in user crates

Here are a few examples of the identity function being defined and used:

+ https://docs.rs/functils/0.0.2/functils/fn.identity.html
+ https://docs.rs/tool/0.2.0/tool/fn.id.html
+ https://github.com/hephex/api/blob/ef67b209cd88d0af40af10b4a9f3e0e61a5924da/src/lib.rs

There's a smattering of more examples. To reduce duplication, it
should be provided in the standard library as a common place it is defined.

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

## The case for inclusion in the prelude

Let's compare the effort required, assuming that each letter
typed has a uniform cost wrt. effort.

```rust
use std::convert::id; iter.filter_map(id)

fn id<T>(x: T) -> T { x } iter.filter_map(id)

iter.filter_map(::std::convert::id)

iter.filter_map(id)
```

Comparing the length of these lines, we see that there's not much difference in
length when defining the function yourself or when importing or using an absolute
path. But the prelude-using variant is considerably shorter. To encourage the
use of the function, exporting to the prelude is therefore a good idea.

In addition, there's an argument to be made from similarity to other things in
`core::convert` as well as `drop` all of which are in the prelude. This is
especially relevant in the case of `drop` which is also a trivial function.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

An identity function is a mapping of one type onto itself such that the output
is the same as the input. In other words, a function `id : T -> T` for some
type `T` defined as `id(x) = x`. This RFC adds such a function for all types
in Rust into libcore at the module `core::convert` and defines it as:

```rust
pub fn id<T>(x: T) -> T { x }
```

This function is also re-exported to `std::convert::id` as well as
the prelude of both libcore and libstd.

It is important to note that the input `x` passed to the function is
moved since Rust uses move semantics by default.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

An identity function defined as `pub fn id<T>(x: T) -> T { x }` exists in
`core::convert::id`. The function is also re-exported to `std::convert::id`
as well as the prelude of both libcore and libstd.

Note that the identity function is not always equivalent to a closure
such as `|x| x` since the closure may coerce `x` into a different type
while the identity function never changes the type.

# Drawbacks
[drawbacks]: #drawbacks

It is already possible to do this in user code by:

+ using an identity closure: `|x| x`.
+ writing the identity function as defined in the RFC yourself.

These are contrasted with the [motivation] for including the function
in the standard library.

# Rationale and alternatives
[alternatives]: #alternatives

The rationale for including this in `convert` and not `mem` is that the
former generally deals with conversions, and identity conversion" is a used
phrase. Meanwhile, `mem` does not relate to `identity` other than that both
deal with move semantics. Therefore, `convert` is the better choice. Including
it in `mem` is still an alternative, but as explained, it isn't fitting.

The rationale for including this in the prelude has been previously
explained in the [motivation] section. It is an alternative to not do that.
If the function is not in the prelude, the utility is so low that it may
be a better idea to not add the function at all.

Naming the function `identity` instead of `id` is a possibility.
However, to make the `id` function more appetizing than using a `|x| x`, it is
preferrable for the identity function to have a shorter but still clear name.

# Unresolved questions
[unresolved]: #unresolved-questions

There are no unresolved questions.