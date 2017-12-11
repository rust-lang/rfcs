- Feature Name: semantic_newtypes
- Start Date: 2014-07-26
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Introduce a newtype construction allowing newtypes to use the
capabilities of the underlying type while keeping type safety.

# Motivation

Consider the situation where we want to create separate primitive
types. For example we want to introduce an `Inch` and a `Cm`. These
could be modelled with `usize`, but we don't want to accidentally
mix the types.

With the current newtypes:

```rust
struct Inch(usize);
struct Cm(usize);

// We want to do generic manipulations
fn calc_distance<T: Sub>(start: T, end: T) -> T {
    end - start
}

let (start_inch, end_inch) = (Inch(10), Inch(18));
let (start_cm, end_cm) = (Cm(2), Cm(5));

// We must explicitly destruct to reach the values
let (Inch(start), Inch(end)) = (start_inch, end_inch);
let inch_dist = Inch(calc_distance(start, end));

let (Cm(start), Cm(end)) = (start_cm, end_cm);
let cm_dist = Cm(calc_distance(start, end));

let (Inch(inch_val), Cm(cm_val)) = (inch_dist, cm_dist);
println!("dist: {} and {}", inch_val, cm_val);

// Disallowed compile time
let not_allowed = calc_distance(start_inch, end_cm);
```

This is verbose, but at least the types don't mix.
We could explicitly define traits for the types, but that's a lot of duplication
if we want the same capabilities as the underlying type. Additionally, if
someone defines a custom trait in a downstream crate for an upstream type, we
any users of our newtype would not be able to use the newtype where the
downstream trait is used as a bound.

Another option is to use the `type` keyword, but then we loose type safety:

```rust
type Inch = usize;
type Cm = usize;

let inch: Inch = 10;
let cm: Cm = 2;

let oops = inch + cm; // not safe!
```

# Guide-level explanation

Imagine you have many `Vec`s in your code that are all indexeable by some
different kind of id. As a small example, you have a `Vec<User>` and a `Vec<Pet>`.
If you get a `usize` for a userid, you can accidentally use it to index the
`Vec<Pet>`. Since these ids have nothing in common, it might be desirable to
make sure that you have a custom id type that cannot be confused with any other
id type:

```rust
type UserIndex is new usize;
// assume TVec is essentially `Vec` with a generic arg for the index type
type UserVec is new TVec<UserIndex, User>;
type PetIndex is new usize;
type PetVec is new TVec<PetIndex, Pet>;

fn foo(&mut self, u: UserIndex, p: PetIndex) {
    self.users[p].pets.add(u); // ERROR users array can only be indexed by UserIndex
    self.users[u].pets.add(p); // correct
}
```

# Reference-level explanation

Steal the `is new` syntax from Ada's newtypes by extending type aliases
declarations with the `type` keyword.

```rust
type Inch is new usize;
type Cm is new usize;

// We want to do generic manipulations
fn calc_distance<T: Sub>(start: T, end: T) -> T {
    end - start
}

// Initialize the same way as the underlying types
let (start_inch, end_inch): (Inch, Inch) = (10, 18);
let (start_cm, end_cm): (Cm, Cm) = (2, 5);

// Here `calc_distance` operates on the types `Inch` and `Cm`,
// where previously we had to cast to and from `usize`.
let inch_dist = calc_distance(start_inch, end_inch);
let cm_dist = calc_distance(start_cm, end_cm);

println!("dist: {} and {}", inch_dist, cm_dist);

// Disallowed at compile time
let not_allowed = calc_distance(start_inch, end_cm);
```

It would also allow generics:

```rust
struct A<N, M> { n: N, m: M }
type B<T> is new A<usize, T>;

let b = B { n: 2u, m: "this is a T" };
```

It would not be possible to use the newtype in place of the parent type,
we would need to resort to traits.

```rust
fn bad(x: usize) { ... }
fn good<T: Sub>(x: T) { ... }

type Foo is new usize;
let a: Foo = 2;
bad(a); // Not allowed
good(a); // Ok, Foo implements Sub
```

## Derived traits

In the derived trait implementations the basetype will be replaced by the newtype.

So for example as `usize` implements `Add<usize>`, `type Inch is new usize`
would implement `Add<Inch>`.

## Scoping

Newtypes would follow the natural scoping rules:

```rust
type Inch is new usize; // Not accessible from outside the module
pub type Cm is new usize; // Accessible

use module::Inch; // Import into scope
pub use module::Inch; // Re-export
```

### Reexporting private types

Newtypes are allowed to be newtypes over private types:

```rust
mod foo {
    struct Foo;
    pub type Bar is new Foo;
}
let x: foo::Bar = ...; // OK
let x: foo::Foo = ...; // Not OK
```

## Casting

Newtypes can explicitly be converted to their base types, and vice versa.
Implicit conversions are not allowed.
This is achieved via the `Into` trait, since newtypes automatically implement
`From<BaseType>` and `From<NewType> for BaseType`. In order to not expose new
`as` casts, the automatically generated implementation simply contains a
`transmute`.

```rust
type Inch is new usize;

fn frobnicate(x: usize) -> usize { x * 2 + 14 - 3 * x * x }

let x: Inch = 2;
println!("{}", frobnicate(x.into()));

let a: usize = 2;
let i: Inch = a; // Compile error, implicit conversion not allowed
let i = Inch::from(a); // Ok
let i: Inch = a.into(); // Ok
let b = usize::from(i); // Ok
```

## Grammar

The grammar rules will be the same as for `type`, but there are two new
contextual keywords `is` and `new`. The reason for using `is new` instead of
another sigil is that `type X = Y;` would be very hard to distinguish from any
alternative like `type X <- Y;` or just `type X is Y;`.

## Implementation

The compiler would treat newtypes as a thin wrapper around the original type.
This means that just declaring a newtype does *not* generate any code, because
the trait and inherent implementations of the base type are reused.

# Drawbacks

It adds a new contextual keyword pair to the language and increases the language complexity.

This requires nontrivial implementation work in the compiler and will touch
essentially the entire compilation pipeline.

Automatically deriving all traits may not make sense in some cases. For example
deriving multiplication for `Inch` doesn't make much sense, as it would result
in `Inch * Inch -> Inch` but semantically `Inch * Inch -> Inch^2`. This is a
deficiency in the design and may be addressed by allowing overwriting trait
implementations on newtypes. Such a change would be strictly backwards
compatible in the language, even if creating overwriting trait impls won't be
backwards compatible for libraries.

Types like `Vec<T>` can't have their index type overwritten by a newtype. With
the increased availability of newtypes this could be resolved by a new generic
argument to `Vec`, which defaults to `usize` and requires an `Into<usize>` impl.

# Alternatives

* Explicitly derive selected traits

    The [`newtype_derive`](https://crates.io/crates/newtype_derive) crate allows
    deriving common traits that just forward to the inner value.

    ```rust
    #[macro_use] extern crate custom_derive;
    #[macro_use] extern crate newtype_derive;

    custom_derive! {
        #[derive(NewtypeFrom, NewtypeAdd, NewtypeMul(i32))]
        pub struct Happy(i32);
    }
    ```

    This would avoid the problems with automatically deriving common traits,
    while some would not make sense.

    We could save a keyword with this approach and we might consider a generalization
    over all tuple structs.

    This approach requires not only two crates, a macro invocation and a list
    of derives, it also doubles the amount of code generated compared to the
    newtype approach.

* Keep it the same

    It works, but life could be simpler. The amount of workarounds, macros and
    complaints about it seem to suggest that something needs to be done. Even
    the compiler itself uses generated newtypes extensively for special `Vec`s that
    have a newtype index type instead of `usize`.

# Unresolved questions

* Conversion from basetype to newtype and vice versa not via `From`?
    * might cause accidental usage of basetype where newtype was expected (e.g. in heavily generic code)
