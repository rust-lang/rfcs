- Start Date: 2014-07-26
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)


# Summary

Introduce a `newtype` construction allowing newtypes to use the
capabilities of the underlying type while keeping type safety.


# Motivation

Consider the situation where we want to create separate primitive
types. For example we want to introduce an `Inch` and a `Cm`. These
could be modelled with `uint`, but we don't want to accidentally
mix the types.

With the current newtypes:

```
struct Inch(uint);
struct Cm(uint);

// We want to do generic manipulations
fn calc_distance<T: Sub<T, T>>(start: T, end: T) -> T {
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
We could explicitly define traits for the types, but that's duplication
if want the same capabilities as the underlying type.

Another option is to use the `type` keyword, but then we loose type safety:

```
type Inch = uint;
type Cm = uint;

let inch: Inch = 10;
let cm: Cm = 2;

let oops = inch + cm; // not safe!
```


# Detailed design

Introduce a new keyword: `newtype`. It introduces a new type, inheriting the
trait implementations from the underlying type, but keeping the types separate.

```
newtype Inch = uint;
newtype Cm = uint;

// We want to do generic manipulations
fn calc_distance<T: Sub<T, T>>(start: T, end: T) -> T {
    end - start
}

// Initialize the same way as the underlying types
let (start_inch, end_inch): (Inch, Inch) = (10, 18);
let (start_cm, end_cm): (Cm, Cm) = (2, 5);

// Here `calc_distance` operates on the types `Inch` and `Cm`,
// where previously we had to cast to and from `uint`.
let inch_dist = calc_distance(start_inch, end_inch);
let cm_dist = calc_distance(start_cm, end_cm);

println!("dist: {} and {}", inch_dist, cm_dist);

// Disallowed compile time
let not_allowed = calc_distance(start_inch, end_cm);
```


It would also allow generics, like `type`:

```
struct A<N, M> { n: N, m: M }
newtype B<T> = A<uint, T>;

let b = B { n: 2u, m: "this is a T" };
```

It would not be possible to use the `newtype` in place of the parent type,
we would need to resort to traits.

```
fn bad(x: uint) { ... }
fn good<T: Sub>(x: T) { ... }

newtype Foo = uint;
let a: Foo = 2;
bad(a); // Not allowed
good(a); // Ok, Foo implements Sub
```


## Derived traits

In the derived trait implementations the basetype will be replaced by the newtype.

So for example as `uint` implements `Add<uint, uint>`, `newtype Inch = uint`
would implement `Add<Inch, Inch>`.

This would present a problem when we have a specialization with the same parameter
as a another parameter with a fixed type. For example `trait Shl<RHS, Result>` where
`RHS = uint` in  all implementations. Specifically:

```
impl Shl<uint, int> for int
impl Shl<uint, i8> for i8
impl Shl<uint, uint> for uint
impl Shl<uint, u8> for u8
...
```

`newtype Inch = int` would implement `Shl<uint, Inch>` but `newtype Inch = uint`
would implement `Shl<Inch, Inch>`, which is not what we want.

A solution would be to allow `Self` in trait implementations. This would require
us to change `Shl` to:

```
impl Shl<uint, Self> for int
impl Shl<uint, Self> for i8
impl Shl<uint, Self> for uint
impl Shl<uint, Self> for u8
...
```

Then `newtype Inch = uint` would implement `Shl<uint, Inch>`.

`Self` in trait implementations might require a new RFC.



## Scoping

`newtype` would follow the natural scoping rules:

```
newtype Inch = uint; // Not accessible from outside the module
pub newtype Cm = uint; // Accessible

use module::Inch; // Import into scope
pub use module::Inch; // Re-export
```


## Casting

Newtypes can explicitly be casted to their base types, and vice versa.
Implicit conversions should not be allowed.

```
newtype Inch = uint;

fn frobnicate(x: uint) -> uint { x * 2 + 14 - 3 * x * x }

let x: Inch = 2;
println!("{}", frobnicate(x as uint));

let a: uint = 2;
let i: Inch = a; // Compile error, implicit conversion not allowed
let i: Inch = a as Inch; // Ok
```


## Grammar

The grammar rules will be the same as for `type`.



# Drawbacks

It adds a new keyword to the language and increases the language complexity.

Automatically deriving all traits may not make sense in some cases.
For example deriving multiplication for `Inch` doesn't make much sense, as it would
result in `Inch * Inch -> Inch` but semantically `Inch * Inch -> Inch^2`.

This is a deficiency in the design and a better approach may be to explicitly
specify which traits to derive.


# Alternatives

* Explicitly derive selected traits

    Similar to GHC's [`GeneralizedNewtypeDeriving`][newtype-deriving]. E.g.:

    ```
    #[deriving(Sub)]
    struct Inch(uint);

    #[deriving(Sub)]
    struct Cm(uint);
    ```

    This would avoid the problems with automatically deriving all traits,
    while some would not make sense.

    We could save a keyword with this approach and we might consider a generalization
    over all tuple structs.


* Keep it the same

    It works, but life could be simpler.

* `as` could be used to convert from a newtype to the underlying value:

    ```
    struct Inch(int);

    let v: Inch = Inch(10);
    println!("inches: {}", v as int);
    ```

    But we still need to explicitly cast when using generic functions:

    ```
    let dist = Inch(calc_distance(start_inch as int, end_inch as int));
    ```

    It also loses type safety.

* Implement similar behaviour with a macro instead, similar to `bitflags!`

    This would not allow us to derive all trait implementations automatically however.
    It would work for only primitive types.


# Unresolved questions

Not sure how to actually implement it.

[newtype-deriving]: https://www.haskell.org/ghc/docs/7.8.1/html/users_guide/deriving.html#newtype-deriving

