- Start Date: 2014-5-25
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

`Coercible` and `HasPrefix` built in traits are added that allow zero cost coercions
(like `transmute`) in a safe manner in many circumstances, subsuming subtyping, mass
borrowing, and mass newtype wrapping and unwrapping.

# Motivation

## Subtyping

It is unclear, as of yet, how Rust will eventually implement inheritance. However, one key
property of it must be that if `A` is a subtype of `B`, then `&A` can be freely transformed
into `&B`. Moreover, it must be easy to abstract over these relationships.

## Mass Borrowing

Although it is very easy to borrow individual pointers (converting from `Box<T>` to `&T`),
it is currently impossible to do this to pointers within a data structure or to many pointers
at once: converting `&'a HashMap<K, Box<V>>` to `&'a HashMap<K, &'a V>` is perfectly valid,
yet the only way to do it now is to reallocate and rehash the whole map.

## Mass Wrapping and Unwrapping

Structs with single fields, or newtypes in Haskell parlance, are often used to attatch
semantic information to plain types. For example, `uint` is an generic integer type,
giving no information about what that integer might mean. To give more information, one
could create a wrapper type like so:

```rust
pub struct Age {
	pub age: uint
}
```

Unfortunately, such wrapper types are not currently free. Despite an easy conversion
between individual `Age`s and `uint`s, there is no way to convert, for example, a
`HashMap<K, Age>` to a `HashMap<K, uint>`.

## Existing `transmute`s

There are various calls to `transmute` scattered throughout the Rust codebase, and a
number of them are perfectly safe. It would be beneficial to write these in a way that
doesn't use any unsafe code but still remains zero cost.

# Detailed design

The user-facing interface is exposed primarily as a trait and a function/method:

```rust
trait Coercible<T> { }
#[inline(always)]
fn coerce<U, T: Coercible<U>>(x: T) -> U { unsafe { transmute(x) } }
```

The trait is wired-in to the compiler, and user-defined impls of it are highly restricted as
described in the implementation section talking about roles. `coerce()` would coerce between
any two types where the target type "is a proper subtype of" the input type. Note that `coerce`
is never a virtual call, as it is not a method of `Coercible`: `Coercible<T>` doesn't have a
vtable, and is considered a built-in "kind" alongside `Copy`, `Send`, etc.

Where single inheritance and subtyping conflate many different ideas, among them transparenta
ccess to superstruct fields, zero-cost conversion from sub- to supertypes, and these conversions
being implicit/automatic, `Coercible` captures and exposes only the thing which is truly important:
the zero-cost conversions, and for a much wider range of scenarios.

There would be another such wired-in trait called `HasPrefix`. `T: HasPrefix<U>` corresponds to `T`
starts-with `U`, while `T: Coercible<U>` corresponds to `T` is-a-proper-subtype-of `U`.

```rust
trait HasPrefix<T> { }
```

The primary reason `HasPrefix` is important is because it gives rise to `Coercible` relationships,
as in the example above: `T: HasPrefix<U>` => `&T: Coercible<&U>`.

One of the most important aspects of the single inheritance proposals is that you can abstract over
the inheritance, as with traits: traits can specify that they can only be implemented by structs
inheriting a given struct, and therefore fields of that struct can be accessed through trait objects
without any additional overhead. Here you can accomplish the equivalent by making `HasPrefix<Foo>` a
supertrait of your trait. More flexibly, you can also put it in the "kinds" list: `&MyTrait:HasPrefix<Foo>`.
In general, anything you could express as substruct or subtype relationships in one of the single
inheritance proposals can be expressed as `HasPrefix` and/or `Coercible` bounds, while the reverse is not true.

## Implementation

As with GHC's `Coercible`, the trait is not actually implemented by having honest-to-god wired-in impls,
but it's easier to explain pretending that there are such impls. In reality, when trying to check a
`Coercible` constraint, the compiler would repeatedly reduce the types using the rules described here,
terminating when it hits a base case.

For any two numeric types of the same size, any two zero-sized types whose implementations are visible, and
any type `B` that is a newtype (a struct with a single visible element) of `A`:

```rust
impl Coercible<A> for B { }
impl Coercible<B> for A { }
```

For singleton arrays and their element:

```rust
impl<T> Coercible<T> for [T, ..1] { }
impl<T> Coercible<[T, ..1]> for T { }
```

For tuples of a given size with all elements of the same type, and fixed-length arrays of that size and type:

```rust
impl<T, static N: uint> Coercible<[T, ..N]> for (T, T, .. times N) { } // fake syntax
impl<T, static N: uint> Coercible<(T, T, .. times N)> for [T, ..N] { }
```

For any struct B and its first field A, if it is visible (single inheritance would be a subset of this single case!):

```rust
impl HasPrefix<A> for B { }
```

For tuples and longer tuples:

```rust
impl<A, B, ..X, Y> HasPrefix<(A, B, ..X)> for (A, B, ..X, Y) { } // fake syntax
```

For arrays and longer arrays:

```rust
impl<T, static M: uint, static N: uint> where N > M HasPrefix<[T, ..M]> for [T, ..N] { } // fake syntax
```

Reflexivity:

```rust
impl<T> HasPrefix<T> for T { }
impl<T> Coercible<T> for T { }
```

Transitivity:

```rust
impl<A, B: HasPrefix<A>, C: HasPrefix<B>> HasPrefix<A> for C { }
impl<A, B: Coercible<A>, C: Coercible<B>> Coercible<A> for C { }
```

Is-a-proper-subtype-of implies starts-with:

```rust
impl<A, B: Coercible<A>> HasPrefix<A> for B { }
```

Note that this could be expressed as a subtrait relationship, but it would force implementers of `Coercible`
to also write a useless `HasPrefix` instance, wasting time and space.

To deal with data structures and pointers, we need to introduce the concept of *roles*, similar to what are
described in [Safe Coercions][0]. Each parameter to a generic type is marked either nominal, representational,
phantom, or covariant. Nominal paramaters cannot be changed by `coerce`, representational parameters can only
by changed to something they can `coerce` to, phantom parameters can change to anything, and covariant parameters
can only change to something that is a prefix of the original type. In summary, if `Foo<N, R, P, C>` has nominal `N`,
representational `R`, phantom `P`, and covariant `C`, then we have the following instance:

```rust
impl<N, R2, P2, C2, R1: Coercible<R2>, P1, C1: HasPrefix<C2>> Coercible<Foo<N, R2, P2, C2>> for Foo<N, R1, P1, C1> { }
```

For example:

```rust
// Box<T> has representational T
impl<Out, In: Coercible<Out>> Coercible<Box<Out>> for Box<In> { }

// &T has covariant T
impl<'a, Out, In: HasPrefix<Out>> Coercible<&'a Out> for &'a In { }

// HashMap<K, V> has nominal K and representational V
//
// We require K to be nominal because changing the type could change the
// hash function and so the internal structure
impl<K, Out, In: Coercible<Out>> Coercible<HashMap<K, Out>> for HashMap<K, In> { }

// Phantom parameters are fairly rare, but can be useful when doing
// complex things in the type system.
pub struct Ignore<T> { }

impl<T, U> Coercible<Ignore<U>> for Ignore<T> { }
```

Note that nominal parameters are a strict subset of representational parameters, which are a strict
subset of covariant parameter, which, in turn, are a strict subset of phantom parameters.

To declare the roles of each parameter, users must write impls of `Coercible` that follow the above
patterns - each variable must be independent and only bounded by either `HasPrefix` or `Coercible`.
By default, every parameter is nominal.

These impls will be rejected if they mark a paramater looser than it is used. For example:

```rust
struct Foo1<T> {
	val: T
}

// Error: marking parameter T as covariant when it must be at least representational by its direct inclusion in Foo1
impl<U, T: HasPrefix<U>> Coercible<Foo1<U>> for Foo1<T> { }

struct Foo2<T> {
	val: HashMap<T, uint>
}

// Error: marking parameter T as representational when it must be at least nominal by the use of HashMap<T, uint> in Foo2
impl<U, T: Coercible<U>> Coercible<Foo2<U>> for Foo2<T> { }
```

With roles in place, it is possible to properly discribe the mass borrowing mechanism. We can't directly coerce from
`Box<T>` to `&'s T` because we have no idea what `'s` should be, but it works if the whole thing is frozen for `'s` by
an outer reference. This is valid to do through anything representational or looser. So, assuming that `R` is some structure
with a representational parameter, we have the following impls:

```rust
impl<'s,     T> Coercible<&'s R<&'s T>>         for &'s R<Box<T>> { }
impl<'s,     T> Coercible<&'s mut R<&'s mut T>> for &'s mut R<Box<T>> { }
impl<'s, 't, T> Coercible<&'s R<&'t T>>         for &'s R<&'t mut T> { }
```

## Comparison to GHC

Unlike GHC, we do not have symmetry in general: a whole lot of conversions are in one direction only. As in GHC, these
make-believe impls are wildly overlapping and incoherent, but that doesn't matter, because we don't care which impl is
selected (they have no vtable), only whether or not one exists.

And also as in GHC, to preserve abstraction boundaries, as a general principle, for those impls which involve conversions
between user-defined types, they would only "be in scope" when the means to do the conversion manually are in scope. This
means that you could only cast `&Struct` to `&FirstFieldOfStruct` if the first field of the struct is visible to you, you could
only cast `Foo` to `NewTypeOfFoo` if its constructor is visible, and other similar rules along these lines (described above).

# Drawbacks

- This add a completely new, fairly large, and highly specialized piece of code to the trait matching system.
- Visibility rules start to matter in trait matching.
- An inheritance framework could be accepted that uses a different method of subtyping, creating redundancy. I think this is fairly
  unlikely, as the `Coercible` mechanism is quite general.
- Subtyping conversions are explicit. This could be an advantage, depending on your point of view.

# Alternatives

- Somehow generalize the system so that users can put arbitrary bounds on `Coercible` instances. This sounds good, but could be
  added on later (it is completely backwards compatible) and is hard to implement or prove correct.
- Use `as` instead of a standalone function. While this might work, it prevents inference of the final type and conflicts with
  the fact that today's conversions using `as` are not necessarily free.
- Make everything use the loosest role possible by default. This would be fine from a memory safety standpoint, and is in fact
  what GHC does. However, it can often break assumptions made by libraries, and Rust's philosophy of safety by default seems to
  encourage using nominal by default. Additionally, this is more consistent with using opt-in kinds.
- Only implement subtype relationships in some inheritance framework. While the subtyping can be very useful, there are a lot
  of applications that don't follow a strict tree of types. The most common of these is the newtype pattern, which is discouraged
  if we don't give support for coercing within a data structure. Also, most of the inheritance proposals involve complications of
  the syntax and/or codegen. This proposal only touches the type system.
- Not do this at all. This discourages newtype patterns and increases the frequency of unsafe code because of `transmute`. Moreover,
  cheap subtyping becomes impossible.

# Unresolved questions

- Is there any way to generalize the mass borrowing mechanism to not special case `Box`?
- Should contravariant and contravariant representational roles be added? Unlike in GHC, where coercions are symmetric, these are
  needed to coerce the arguments to functions.
- How does the "first field of struct" rule interact with undefined struct layout?
- How can clear error messages be written for breaking the role system?

[0]: https://www.cis.upenn.edu/~eir/papers/2014/coercible/coercible-ext.pdf
