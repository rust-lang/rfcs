- Feature Name: function_output_type_parameters
- Start Date: 2015-10-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Enable monomorphized return types of a function that are only abstractly defined by an trait interface by introducing a new set of "output" generic parameters that can be defined for a function, and extending trait bounds with the concept of "conditional" bounds.

# Motivation

For the issue this tries to solve, see
http://aturon.github.io/blog/2015/09/28/impl-trait/ and
https://github.com/rust-lang/rfcs/pull/105.

For the motivation of this proposal, see discussion at
https://www.reddit.com/r/rust/comments/3mrsvx/resurrecting_impl_trait_aaron_turon/.

The goal of this variation around abstract return types
is to be more general, flexible and ergonomic than the discussed options,
while still being a system to define fully abstract types with less
effort than defining newtypes.

The proposed features should also be maximally consistent with
other parts of the language.

Non goals are to introduce global return type inference in any way, including
leakage of OIBITs.

# Detailed design

Introduce these new features in the language:

## Abstract type aliases

Introduce a new form of type aliases that are abstract, only defined
in terms of trait bounds:

```rust
abstract type Foo: Clone = ();
```

It is legal to "coerce" to and from the underlying type in the
same privacy scope the abstract type is define in:

```rust
mod x {
    pub abstract type Foo: Clone = ();

    pub fn new_foo() -> Foo { () } // OK
}
let a: x::Foo = x::new_foo();      // OK
let b: x::Foo = a.clone();         // OK
let c: x::Foo = ();                // ERR
```

Semantically, they should behave just like existing generic type parameters
with bounds do. That is to say, the language should not expose
any information about the type not defined by its bounds:

```rust

mod foo {
    abstract type X: Clone = ();
}

fn bar<T: Clone>(a: T, b: foo::X) {
    // T and foo::X don't differ in any way here.
}

```

Internally, the compiler would have full knowledge about the actual type,
it just can't be named nor be accessed by the user because its hidden
behind a privacy boundary.

Abstract type aliases can be both standalone items and associated items.

Interestingly, at the moment it is actually legal to define a regular
type alias with trait bounds as referring to an private type:

```rust
// compiles today (2015-10-02):
mod foo {
    pub type Bar where Bar: Clone = Baz;
    struct Baz;
}
let _: foo::Bar;
```

In the current form, this just seems like a number of bugs caused by the
compiler not checking certain things.

One alternative option here would be to change the semantic of existing type aliases
to that of the proposed abstract type aliases if they refer to private types.

The benefit would be that there would be one additional feature less in the
language to know about. The disadvantage would be that it prevents
hiding the actual type if the type in question is public.

## Output type parameters on functions

Add a new set of generic parameters to function items.
The exact syntax is up to debate, this RFC will put them as a new `<>`
delimited list after the `()`:

```rust
fn foo<T, U: A>()<V, W: B> -> X<V, W> where T: C, V: D { ... }
```

The identifiers introduced like this would

- define `abstract type` associated items on the concrete function item.
- be required to appear in the return type itself.
- be defined by type inference on the inside of the function body.
- would have their underlying type be private to the function body.
- would have their trait bounds defined the same way as for the regular
  input type parameters; either as `<T: X>` bounds or as part of
  the `where` clauses.

```rust
fn foo()<T: Clone> -> T where T: Mul { 42 }

let a = foo();     // OK
let b = a.clone(); // OK
let c = a + b;     // ERR, foo::T not known to be Add
let d = a * b;     // OK
let e = a * b;     // ERR, a and b moved, caused by foo::T not known to be Copy

let x: foo::T = foo() // OK
let y: foo::T = 0     // ERR, foo::T not known to be an integer

```

This feature would interact with other function-related items in the same way
as generics already do:

- Works for standalone functions, associated functions or methods.
- Not definable for closure literals, since they use concrete, inferred types.
- The `Output` associated type of closure traits
  for such a concrete function item would be simply defined in terms of the
  abstract output types of the function item itself:

```rust
fn foo()<T: Clone> -> T where T: Mul { 42 }
// from compiler generated:
impl Fn<()> for foo {
    type Output = foo::T;
    // ...
}

let x: foo::T = foo();
let y: foo::Output = x;        // OK
let z: &Fn() -> foo::T = &foo; // OK (but not very useful)

```

## Conditional bounds

Extend `where` bounds with the concept of "conditional bounds":

```rust
T: Foo if U: Bar
```

Semantically, they express that the bound on the lhs of `if` is only
fulfilled if the bound on the rhs is.

They can appear in any location where `where` clauses are legal,
including in trait definitions and abstract return types:

```rust
trait Foo where Self: Add if Self: Clone { }

fn foo<T>(t: T)<U> -> U
    where U: Clone if T: Clone,
          U: Foo,
{ t }

impl Foo for i32 { }    // OK
let x = foo::<i32>(42); // OK
let y = x.clone();      // OK
let z = x + y;          // OK
let e = x * y;          // ERR, foo<i32>::U not known to be Mul

struct X;
impl Foo for X { }   // OK
let a = foo::<X>(X); // OK
let b = a.clone();   // ERR, foo<X>::U is not Clone because X is not Clone
let c = a + b;       // ERR, foo<X>::U is not Add because foo<X>::U is not Clone

#[derive(Clone)]
struct Y;
impl Foo for Y { }   // ERR, Y needs to implement Add if it implements Clone.

```

The bound needs to apply if the condition is met. Eg, for the example above:

- If a type implements `Clone` but not `Add`,
  implementing `Foo` for it is an type error.
- If a type does not implement `Clone`, then it does not matter
  if the type implements `Add` or not, but a `Foo` bound will not know
  about `Self: Add`.

## Sugar for defining generic type parameters

A problem with the existing generic syntax is
that its often more verbose than the non-generic version of the API. Compare:

```rust
fn take_iter<I: Iterator>(i: &mut I) { ... }
fn take_iter(i: &mut Iterator) { ... }
```

Adding additional output type parameters to functions will only increase this
issue.

This proposes two incremental syntax sugars for shrinking down the size of
generic declarations:

1. If a generic type parameter only appears in one location in a function
   signature, it may be declared directly inline with the syntax
   `type IDENT[: BOUNDS]`.
   Example:
   ```rust
      fn foo<T>(t: T);
   => fn foo(t: type T);
      fn bar()<O> -> O;
   => fn bar() -> type O;
   ```
2. If the nominal name of a type parameter is not needed, the prior syntax
   may be shortened further to `impl BOUNDS`.
   Example:
   ```rust
      fn foo<T: X>(t: T);
   => fn foo(t: type T: X);
   => fn foo(t: impl X);
      fn bar()<O: X> -> O;
   => fn bar() -> type O: X;
   => fn bar() -> impl X;
   ```
   These will still define input and abstract output type parameters, they will
   just be anonymous in the same sense that function items and closure types
   are today. This sugar might be referred to as "type elision". ;)

In both cases the type positions in the argument list will declare parameters in the
input parameter list, and type positions in the return type will declare
parameters in the output parameter list.

---

So in summary, this enables

- abstract return types without introducing global type inference.
- abstract return types implemented with a feature that is consistent
  with the existing generic system (associated/output type parameters),
  and that can be evolved in a consistent way in the future.
  (Eg, adding HKT support: `fn foo()<M<*>: Monad> -> M<u32>`)
- abstract types in general, enabling more easier data hiding in libraries.
- the ability to name abstract return types, making them usable for
  type annotations in custom types.
- shorter syntax for declaring generic types in function signatures,
  making them as ergonomic to use as trait objects.
- the ability to define trait bounds in conditional ways, allowing more
  ways to define and type check interfaces.

As an example, this is how the iterator adapter
situation in std might look like under this RFC:


```rust
mod iter {
    // Define the interface of an iterator adapter
    // at a central location. This helps for reuse and documentation.
    // (This would benefit from an feature for defining trait aliases easily)
    trait Adapter<I> where
        Self: Iterator,
        I: Iterator,
        Self: DoubleEndedIterator if I: DoubleEndedIterator,
        Self: Clone if I: Clone,
        Self: Send if I: Send, // OIBITs can just be conditionally encoded as well
        Self: Sync if I: Sync,
        {}
    impl<T, I> Adapter<I> for T where
        T: Iterator,
        I: Iterator,
        T: DoubleEndedIterator if I: DoubleEndedIterator,
        T: Clone if I: Clone,
        T: Send if I: Send,
        T: Sync if I: Sync,
        {}

    trait Iterator {
        // ...

        fn enumerate(self) -> impl Adapter<Self> {
            Enumerate { ... }
        }

        fn chain(self, other: type O: IntoIterator) -> impl Adapter<Self>
            where O::Item = Self::Item
        {
            Chain { ... }
        }

        fn map(self, f: impl FnMut(Self::Item) -> type T) -> impl Adapter<Self> {
            Map { ... }
        }

        fn filter(self, f: impl FnMut(&Self::Item) -> bool) -> impl Adapter<Self> {
            Filter { ... }
        }
    }
}
```

# Drawbacks

As always, introducing major features into the language increases complexity,
and might thus not be desirable in favor of keeping the language simpler.

# Alternatives

Discussions about different designs can be found at the links in the
motivation section. As for this design, there are a number of variations:

- Syntactic bikesheds like position of generic parameters,
  or `abstract type T;` vs `abstract T;`
- Don't introduce the `type T` sugar in function signatures, because it might
  not be as a clear as an improvement as the `impl` one.
- Don't introduce the `impl` sugar in function signatures, because the
  anonymous generic parameters required for it might cause complications.
- Restrict the `impl` sugar to once in the return type only, and
  let it generate a predictable type name like `Abstract`.
- The above, but without having manually definable output
  type parameters for function items at all.
  Instead, have a special keyword for the abstract return type, like
  `fn foo() -> impl Foo; let x: foo::return = foo();`
- The above, but without having abstract type aliases as a separate concept,
  instead it being a special aspect of the `foo::return` type.


# Unresolved questions

- Are all parts of this proposal feasible to implement?
- How do anonymous type parameters as introduced with the `impl` sugar
  interact with the language?
