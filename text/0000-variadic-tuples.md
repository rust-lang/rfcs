- Feature Name: variadic_tuples
- Start Date: 2019-08-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

Tuples types are ordered set of type, but users can only use tuple with fixed arity.

This RFC aims to allow the use of a _variadic tuple_ which is a tuple with an arbitrary arity.

# Motivation

[motivation]: #motivation

## Arbitrary tuple arity support

Currently, when a user wants to either use or add behavior to tuples, he writes an impl for each tuple arity.
For easier maintenance, it is usually done with a `macro_rules` and implemented up to 12 arity tuple. (see [ `Hash` implementation in `std`](https://github.com/rust-lang/rust/blob/master/src/libcore/hash/mod.rs)).

Variadic tuple will provide several benefits considering trait implementation for tuple or using variadic tuples:

- Implementations will be easier to write
- Implementations will be easier to read and maintain
- The compiler will compile implementation only for required tuple arity

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Let's call a _variadic tuple type_ a tuple type with an arbitrary arity and a _variadic tuple_ an instance of a variadic tuple type.

A variadic tuple type is declared with `(..T)` and a variadic tuple type can be expanded with `(..Vec<T>)`.

Note: To illustrate the RFC, we will use the current implementation of the `Hash` trait for tuples.

```rust
// Quote from Rust source code
// This macro implements `Hash` for a tuple.
// It is used like this: `impl_hash_tuple! { A B C D E F }` for a 6-arity tuple.
macro_rules! impl_hash_tuple {
    () => (
        #[stable(feature = "rust1", since = "1.0.0")]
        impl Hash for () {
            fn hash<H: Hasher>(&self, _state: &mut H) {}
        }
    );

    ( $($name:ident)+) => (
        #[stable(feature = "rust1", since = "1.0.0")]
        impl<$($name: Hash),+> Hash for ($($name,)+) where last_type!($($name,)+): ?Sized {
            #[allow(non_snake_case)]
            fn hash<S: Hasher>(&self, state: &mut S) {
                let ($(ref $name,)+) = *self;
                $($name.hash(state);)+
            }
        }
    );
}

macro_rules! last_type {
    ($a:ident,) => { $a };
    ($a:ident, $($rest_a:ident,)+) => { last_type!($($rest_a,)+) };
}
```

## Variadic tuple type

### Declaration

Variadic tuple types are always declared in a generic parameter group.

There are two different syntaxes:

1. `(..T)`: declare a single variadic tuple type identified by `T`
2. `(..(T1, T2, ..., Tn))`: declare n variadic tuple types identified by `T1`, `T2`, ..., `Tn`, all these variadic tuple types have the same arity.

Declaration examples:

- `struct VariadicStruct<(..T1)>` : declares a struct with a variadic tuple type identified by `T1` in its generic parameters
- `fn my_func<(..T1), (..T2)>()`: a function can have variadic tuple type parameters
- `impl<(..Head)>`: is an implementation block that uses a variadic tuple type identified by `Head`
- `impl<A, B, C, (.._Tail)>`:  same as above, but with other generic parameters
- `impl<A, B, (..C), (..D)>`: there can be several variadic tuple types declared in a generic parameter group
- `impl<A, B, (..(C, D)), (..E)>`: and we can mix both syntaxes

Usage examples:

```rust
struct VariadicStruct<(..T)>
VariadicStruct<(usize,)>                // => (..T) matches (usize,)
VariadicStruct<(usize, bool)>   // => (..T) matches (usize, bool)

impl <(..(T1, T2))> VariadicStruct<(..(T1, T2))> { ... }
VariadicStruct::<((usize, bool),)>
// (..(T1, T2)) matches ((usize, bool),)
// (..T1) is (usize,)
// (..T2) is (bool,)
VariadicStruct::<((usize, bool), (String, i8))> // (..(T1, T2)) matches ((usize, bool), (String, i8))
// (..T1) is (usize, String)
// (..T2) is (bool, i8)
```

### Expansion

The expansion syntax is: `..<expr(T1, T2, ..., Tn)>` where `<expr(T1, T2, ..., Tn)>` is an expression using the variadic tuple type identifiers `T1`, `T2`, ..., `Tn`.

Note: The expression in an expansion form can be enclosed by parenthesis for clarity. Ex: `..(T: Clone)`.

The expansion form is allowed in all places where a type is allowed and in `where` bounds.

Examples:

```rust
type TuplesOfRef<'a, (..T)> = (..&'a T);
TuplesOfRef<'b, (usize, bool)>; // = (&'b usize, &'b bool)

struct MegaMap<(..(K, V))> {
  maps: (..HashMap<K, V>),
}
// 
// struct MegaMap<((usize, bool), (String, i8))> {
//   maps: (HashMap<usize, bool>, HashMap<String, i8>),
// }

fn append<(..L), (..R)>(l: (..L), r: (..R)) -> (..L, ..R)
where 
    ..(L: 'static + Clone), 
    ..(R: 'static + Clone), {
    ...
}

//
// fn append<(usize, Vec<bool>), (&'static str, u8, i16)>(
//       l: (usize, Vec<bool>), 
//       r: (&'static str, u8, i16)
//   ) -> (usize, Vec<bool>, &'static str, u8, i16) {
//   ...
// }
```

Note: If an expansion syntax does not contains any variadic tuple type identifier, it resolves to the unit type `( )`.

Note2: If an expansion syntax contains multiple variadic tuple type identifiers, they must all have been declared together with the syntax `( ..(T1, T2, ..., Tn))` to ensure they have the same arity.

## Variadic tuple

A _variadic tuple_ is a variable of a variadic tuple type.

### Destructuring a variadic tuple

A variadic tuple can be destructured to manipulate its members.

There are 3 syntaxes possible to destructure a variadic tuple for a variadic tuple `(..T)`:

1. `(v @ ..)` of variadic tuple type `(..T)`
2. `(ref v @ ..)` of variadic tuple type `(..&T)`
3. `(ref mut v @ ..)` of variadic tuple type `(..&mut T)`

Also, the destructure pattern can be combined with other members. For instance:

```rust
{
  let source: (Head, ..Tail) = _;
  // `head` is a variable of type `&Head`
  // `tail` is a tuple variable of type `(..&Tail)`
  let (ref head, ref tail @ ..) = source;
}
{
  let mut source: (..L, ..R) = _;
  // `l` is a tuple variable of type `(..&mut L)`
  // `r` is a tuple variable of type `(..&mut R)`
  let (ref mut @ l .., ref mut r @ ..) = source;
}

```

Examples:

```rust
// The function argument is destructured as a variadic tuple with identifier `v`
fn my_func<Head, (..T)>((head, v @ ..): (Head, ..T)) -> (..T) { 
    ...
}

impl<Head, (..T)> Clone for (Head, ..T) 
where 
    ..(T: Clone),
    Head: Clone, {
  fn clone(&self) -> Self {
    // We destructure `*self` which has a variadic tuple type `(Head, ..T)`
    let (ref head, ref v @ ..) = *self;
    ...
  }
}
```

### Iterating over variadic tuple

We can iterate over the member of a variadic tuple or over the type of a variadic tuple type.

*Important note: the iteration is inlined by the compiler, it is not a generic runtime heterogenous iteration of tuple members.*

We use the following syntax to iterate on variadic tuples:

```rust
// The result of the for block is a variadic tuple made of
// the result of each iteration
let result: (..Option<&V>) = {
    // `key` and `map` are variables iterating the variadic tuples `k: (..K)` and `maps: (..&HashMap<K, V>)`, `key` will iterate by reference (because of the ref keyword)
    // `KEY` and `VALUE` are type variables iterating the variadic tuple types `(..K)` and `(..V)`
    // `(k, maps)` declares the iterated variadic tuples `k` and `maps`
    // `<K, V>` declares the iterated variadic tuple types
    (for (ref key, map) <KEY, VALUE> @in (k, maps) <K, V> {
        HashMap::<KEY, VALUE>::get(&map, key)
    })
};
```

Note: when iterating over multiple variadic tuple or variadic tuple types, they must have all the same arity. To ensure this, all variadic tuple types involved must have been declared together.

Examples:

```rust
impl<(..(K, V))> MegaMap<(..(K, V))>
where ..(K: Hash), {
    fn get(&self, k: (..K)) -> (..Option<V>) {
        let result: (..Option<&V>) = {
            (for (ref k, map) <Key, Value> @in (k, &self.maps) <K, V> {
                HashMap::<Key, Value>::get(&map, k)
            })
        };

        result
    }
}

impl<(..T), Last> Hash for (..T, Last)
where
    ..(T: Hash),
    Last: Hash + ?Sized, {

    #[allow(non_snake_case)]
    fn hash<S: Hasher>(&self, state: &mut S) {
        let (ref tuple @ .., ref last) = *self;
       
        // Use case: only variadic tuple
        for member @in tuple {
          member.hash(state);
        };
        last.hash(state);

        // Use case: variadic tuple and type
        for member <H> @in tuple <T> {
          <T as Hash>::hash(&member, state);
        };
        last.hash(state);
    }
}

trait Merge<(..R)> {
    type Value;
    fn merge(self, r: (..R)) -> Self::Value;
}

impl<(..L), (..R)> Merge<(..R)> for (..L) {
    type Value = (..L, ..R);

    fn merge(self, r: (..R)) -> Self::Value {
        (
            for l1 @in self { l1 },
            for r1 @in r { r1 },
        )
    }
}

trait Integer {
    fn one() -> Self;
}

fn add_one<(..T)>((..t): (..T)) -> (..T)
where
    ..(T: Integer + Add), {
    (for t1 <T1> @in t <T> { t1 + T1::one() })
}
```

## The `Hash` trait

Let's implement the `Hash` trait:

```rust
// For the example, we consider the impl for (A, B, C). So `(..T)` matches `(A, B, C)`
// We have the first expansion here, `(..T, Last)` expands to `(A, B, C, Last)`
impl<(..T), Last> Hash for (..T, Last) 
where
        // Expands to `A: Hash, B: Hash, C: Hash,`
    ..(T: Hash,),
    Last: Hash + ?Sized, {

    #[allow(non_snake_case)]
    fn hash<S: Hasher>(&self, state: &mut S) {
        // Destructure self to a variadic tuple `tuple` and a variable `last`. The variadic tuple type of `tuple` is `(..&T)`
        // So it will be equivalent to `let (ref a, ref b, ref c, ref last) = *self; let tuple = (a, b, c);`
        let (ref tuple @ .., ref last) = *self;
        for member @in tuple {
          member.hash(state);
        };
        // The for loop will be inlined as: 
        // ( 
        //    { v.0.hash(state); },
        //    { v.1.hash(state); },
        //    { v.2.hash(state); },
        // );
        last.hash(state);
    }
}
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

## Syntax

Note EBNF syntax is based on [Rust's grammar](https://doc.rust-lang.org/nightly/grammar.html).

### Variadic tuple type declaration

A variadic tuple type identifier identifies a list of types.

When declared togethers, each identifiers identifies a list of types. For instance:

```rust
struct MyStruct<(..(L, R))> { ... }
MyStruct::<((usize, bool), (i8, f32))> { ... }
// `(..(L, R))` matches `((usize, bool), (i8, f32))`
// `(..L)` is `(usize, i8)`
// `(..R)` is `(bool, f32)`
```

variadic tuple type declaration
```ebnf
var_tuple_type_decl : single_var_tuple_decl | multiple_var_tuple_decl;
single_var_tuple_decl : "(.." ident ")";
multiple_var_tuple_decl: "(..(" ident ["," ident] * "))";
```

Note: Although this looks like a type-level pattern matching, it can match against only tuple of identifiers. So the following declaration is invalid: `struct MyStruct<(..(L, Vec<R>))> { ... }`

### Variadic tuple type expansion

On location where a type is expected, the expansion will resolve to a type. On where bounds it can be used to declare bounds on the type contained in the variadic tuple type.

Examples:

```rust
type TupleOfVec<(..T)> = (..Vec<T>);

struct MyStruct<(..T)>
where ..(T: Clone) { ... }
```

* variadic tuple type
```ebnf
var_tuple_type_exp : raw_var_tuple_type_exp | par_var_tuple_type_exp;
raw_var_tuple_type_exp : ".." type_expr;
par_var_tuple_type_exp : "..(" type_expr ")";
```

* variadic bounds
```ebnf
var_tuple_type_exp_where : "..(" type_expr ":" bound-list ")";
```

### Destructuring a variadic tuple

When destructuring a variadic tuple, we can destructure the variadic parts into tuple variables. The identifier is a variable of type `(..T)` or `(..&T)` or `(..&mut T)`, depending on the syntax used.

```rust
{
  let mut source: (..T, Tail) = _;
  // v is a variable of type `(..T)`
  let (v @ .., tail) = source;
  // v is a variable of type `(..&T)`
  let (ref v @ .., ref tail) = source;
  // v is a variable of type `(..&mut T)`
  let (ref mut v @ .., ref mut tail) = source;
}

// If we use `(..T)` = `(A, B, C)` as an example
// Then `let (..(ref v), ref tail) = &source`
// is equivalent to:
// `let (ref a, ref b, ref c, ref tail) = &source;`
// `let v = (a, b, c);`
```

* variadic tuple destructuration
```ebnf
tuple_destr : "(" tuple_destr_ident_any [ "," tuple_destr_ident_any ] * ")";
tuple_destr_ident_any : [ tuple_destr_ident_var | tuple_destr_ident ];
tuple_destr_ident : [ "ref" "mut" ? ] ? ident;
tuple_destr_ident_var : [ "ref" "mut" ? ] ? ident "@" "..";
```

### Variadic tuple iteration

The syntax for the variadic tuple iteration is:

```rust
for $var_id <$type_var_id> @in $variadic_tuples <$variadic_tuple_types> {
    $body
}
```

`$var_id` is a pattern matching the tuple to iterate, it follows the same rules as the variadic tuple destructuration, only 3 syntaxes are allowed for an identifier: `id`, `ref id` or `ref mut id`. (like: `(key value)`, `(ref key, value)`, `(ref mut key, value)`)

`$type_var_id` is a pattern matching the variadic tuple types to iterate, but it has only the first syntax allowed. (No ref, or mut).

`$variadic_tuples` declares the iterated variadic tuples, it has the syntax `id` or `(id1, id2, ..., idn)`.

`$variadic_tuple_types` declares the iterated variadic tuple types, it has the syntax `<ID>` or `<ID1, ID2, ..., IDn>`.

Example:

```rust
impl<(..(K, V))> MegaMap<(..(K, V))>
where ..(K: Hash), {
    fn get(&self, k: (..K)) -> (..Option<V>) {

        let result: (..Option<&V>) = {
            for (ref k, map) <Key, Value> @in (k, &self.maps) <K, V> {
                HashMap::<Key, Value>::get(&map, k)
            }
        };

        // for the compiler, the for block has a kind of
        // `[[(variadic_tuple, variadic_tuple_type)], [variadic_tuple_type]] -> (variadic_tuple, variadic_tuple_type)`
        //
        //  But, we can decompose in two separate steps:
        // `{ HashMap::<K, V>::get(&map, k) }` is a generic fn: 
        // ```rust
        // fn block_body<K, V>(k: &K, map: &HashMap<K, V>) -> Option<&V> 
        // where K: Hash { // Inherit bounds from context (here: the trait) 
        //   HashMap::<K, V>::get(&map, k)
        // }
        // ```
        // 
        // Then the for loop is an inlined for loop calling the 
        // `block_body`
        //
        // ```rust
        // (
        //    block_body::<K0, V0>(&k.0, maps.0),
        //    block_body::<K1, V1>(&k.1, maps.1),
        //    ...
        //    block_body::<Kn, Vn>(&k.n, maps.n),
        // )
        // ```

        result
    }
}
```

* variadic tuple iteration
```ebnf
for_var_tuple : "for" for_tuple_ident ? for_tuple_type_ident ? "@in" for_tuple_ident ? for_tuple_type_ident ? block_expr;
for_tuple_ident : ident | "(" ident [ "," ident ] * ")";
for_tuple_type_ident : "<" ident [ "," ident ] * ">";
```

Note: When using a single for loop to create a tuple, the outer parenthesis are optional:
```rust
// Both syntaxes are equivalent:
let result: (..Option<&V>) = for (ref k, map) <Key, Value> @in (k, &self.maps) <K, V> {
    HashMap::<Key, Value>::get(&map, k)
};

let result: (..Option<&V>) = (for (ref k, map) <Key, Value> @in (k, &self.maps) <K, V> {
    HashMap::<Key, Value>::get(&map, k)
});
```

Note: When using multiple for loops to create a tuple, the parenthesis must be explicit:
```rust
// We have a single tuple with all member at the same level
fn append<(..L), (..R)>(l: (..L), r: (..R)) -> (..L, ..R) {
    (
        for l1 @in l { l1 },
        for r1 @in r { r1 },
    )
}

// Is not equivalent to:
fn append<(..L), (..R)>(l: (..L), r: (..R)) -> ((..L), ..R) {
    (
        (for l1 @in l { l1 }),
        for r1 @in r { r1 },
    )
}
// Where only the tuple `r` is destructured.
```


## Recursion

To implement some feature, we may want to use recursion over the arity of the tuple.
For instance, let's implement a trait that gives the arity of a tuple as a `const` value:

```rust
trait Arity {
    const VALUE: usize;
}

// Termination implementation of the recursion
impl<()> Arity for () {
    const VALUE: usize = 0;
}

// Specialized implementation for the recursion
impl<Head, (..Tail)> Arity for (Head, ..Tail)
where
    (..Tail): Arity, {
    const VALUE: usize = <(..Tail) as Arity>::VALUE + 1;
}
```

Note:

- The `impl<Head, (..Tail)> Arity for (Head, ..Tail)` is the recursive implementation.
- The `impl<(..T)> Arity for (..T)` is the default implementation and will act as the termination of the recursion.

### Recursion for functions

Recursion for functions over variadic tuple is not supported.

Consider this code:
```rust
fn arity<Head, (..Tail)>((h, tail @ ..): (Head, ..Tail)) -> usize {
    arity::<(..Tail)>(tail) + 1
}
// We need to define an explicit implementation of `fn arity<()>`, but there is no mechanism in Rust
// currently to specialize a function implementation
// So we don't deal with this use case in this RFC
// Instead, the compiler will issue a compile error
```

## Errors

### Recursive function implementation over variadic tuple

Recursive function implementation over variadic tuple are not supported.

The following code:
```rust
fn arity<Head, (..Tail)>((h, tail @ ..): (Head, ..Tail)) -> usize {
    arity::<(..Tail)>(tail) + 1
}
```

Will issue:
```rust
error[EXXXX]: the function `fn arity` is recursive over variadic tuple, this is not supported
 --> src/main.rs:10:4
  |
7 |  fn arity<Head, (..Tail)>((h, tail @ ..): (Head, ..Tail)) -> usize
  |  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 
  |
  = help: The implementation of `arity` requires an implementation with `arity::<(..Tail)>`
 --> src/main.rs:11:4
  |
8 |      arity::<(..Tail)>(tail) + 1
  |      ^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = help: `arity` must be called with either the same generic arguments or without variadic tuple types
```

### Missing implementation message during variadic implementation resolution

An error can occur if the compiler don't find an implementation while generating variadic tuple implementations.

Let's consider this code:

```rust
trait Arity {
    const VALUE: usize;
}

impl<Head, (..Tail)> Arity for (Head, ..Tail)
where
    (..Tail): Arity, {
    const VALUE: usize = <(..Tail) as Arity>::VALUE + 1;
}

fn main() {
    let arity = <(usize, bool, u8) as Arity>::VALUE;
}
```

This code must not compile as the termination implementation is missing.
So we will have a `E0277`error:

```rust
error[E0277]: the trait bound `(bool, u8): Arity` is not satisfied
 --> src/main.rs:10:4
  |
7 |     let arity = <(usize, bool, u8) as Arity>::VALUE;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Arity` is not implemented for `()`
  |
  = help: impl `Arity` for `(usize, bool, u8)` requires impl `Arity` for `(bool, u8)`
  = help: impl `Arity` for `(bool, u8)` requires impl `Arity` for `(u8,)`
  = help: impl `Arity` for `(u8,)` requires impl `Arity` for `()`
```

### Variadic implementation cycle dependency error

As a variadic tuple implementation may depend on other variadic tuple implementation, there can be dependency cycle issue.

```rust
trait A { const VALUE: usize = 1; }
trait B { const VALUE: usize = 2; }

impl<Head, (..T)> A for (Head, ..T) 
where (..T): B { const VALUE: usize = 3; }

impl<Head, (..T)> B for (Head, ..T) 
where (..T): A { const VALUE: usize = 4; }

fn main() {
    let v = <(usize, bool) as A>::VALUE;
}
```

This code won't compile because the impl for `A` requires the impl for `B` and the impl for `B` requires the impl for `A`.

This kind of error can already by created without variadic tuple (`E0275`), but variadic tuple will introduce another place where this can happen. So we should have this error: 

```rust
error[E0275]: overflow evaluating the requirement `(usize, bool): A`
  --> src/main.rs:11:13
   |
10 |     let v = <(usize, bool) as A>::VALUE;
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: required by `A::VALUE`
  --> src/main.rs:1:11
   |
1  | trait A { const VALUE: usize = 1; }
   |           ^^^^^^^^^^^^^^^^^^^^^^^
```

### Invalid variadic tuple type declaration

The variadic tuple type declaration is invalid when it can't be parsed as either:

- `(..T)`
- `(..(T1, T2, .., Tn))`

```rust
struct MyStruct<(..Vec<T>)> {
  vecs: (..Vec<T>)
}
```

```rust
error[EXXXX]: invalid variadic tuple type declaration `(..Vec<T>)`
  --> src/main.rs:1:13
   |
10 |  struct MyStruct<(..Vec<T>)> {
   |                  ^^^^^^^^^^^
   |
note: expected either an identifier or a tuple of identifier instead of `Vec<T>`
```

### Invalid variadic tuple type expansion identifiers

Occurs when multiple independent variadic tuple type identifier are used in a single expansion form.

```rust
impl<(..K), (..V)> MyTrait for (..HashMap<K, V>) {
  
}
```

```rust
error[EXXXX]: invalid variadic tuple type expansion `(..HashMap<K, V>)`
  --> src/main.rs:4:13
   |
10 |  impl<(..K), (..V)> MyTrait for (..HashMap<K, V>) {
   |                                   ^^^^^^^^^^^^^^^^^
   |
note: variadic tuple type identifiers `K`, `V` were not declared together
  --> src/main.rs:4:13
   |
10 |  impl<(..K), (..V)> MyTrait for (..HashMap<K, V>) {
   |       ^^^^^^^^^^^^
   |
hint: expected `impl<(..(K, V))>` instead of `impl<(..K), (..V)>`
```

### Invalid variadic tuple pattern matching

The variadic tuple declaration is invalid when it can't be parsed.

```rust
struct MyStruct<(..T)> {
  vecs: (..Vec<T>)
}

impl<(..T)> MyTrait for (..T) {
  fn my_func(&self) {
    let (..(&i)) = &self;
  }
}
```

```rust
error[EXXXX]: invalid variadic tuple pattern `(..(&i))`
  --> src/main.rs:4:13
   |
10 |      let (..(&i)) = &self;
   |          ^^^^^^^^
   |
note: expected `(id @ ..)` or `(ref id @ ..)` or `(ref mut id @ ..)`
```

# Drawbacks

[drawbacks]: #drawbacks

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

## Iteration over variadic tuples syntax

When iterating over variadic tuples, we need to define both variable and type variable. To do so, we use a for loop syntax that is close to the existing one with the folowing differences:
- Instead of `in` we use `@in` to explicitly state which kind of for loop is expected
- Generics variable and lists are enclosed by brackets, like they are in generic parameters

```rust
let result: (..Option<&V>) = {
    for (ref k, map) <Key, Value> @in (k, maps) <K, V> {
        HashMap::<Key, Value>::get(&map, k)
    }
};
```

## Variadic tuple utilities library

The iteration for loop syntax can be combined with utilities to have more flexibility.

Example:

```rust
// Trait utilities

// Merge two tuple togethers
trait Merge<(..R)> {
    type Value;
    fn merge(self, r: (..R)) -> Self::Value;
}
impl<(..L), (..R)> Merge<(..R)> for (..L) {
    type Value = (..L, ..R);
    fn merge(self, r: (..R)) -> Self::Value {
        (for l @in self { l }, for r @in r { r })
    }
}

// Reverse a tuple's members
trait Rev {
    type Value;
    fn rev(self) -> Self::Value;
}
impl Rev for () { 
    type Value = ();
    fn rev(self) -> Self::Value { () }
}
impl<Head, (..Tail)> Rev for (Head, ..Tail) 
where
    (..Tail): Rev,
    <(..Tail) as Rev>::Value: Merge<(Head,)> {
    type Value = <<(..Tail) as Rev>::Value as Merge<(Head,)>>::Output;

    fn rev(self) -> Self::Value { 
        let (h, t @ ..) = self; 
        let rev_t = <(..Tail) as Rev>::rev(t);
        rev_t.merge((h,)) 
    }
}

// Utility to create a tuple of a single type, for instance: ToT<usize> = (..T) -> (..usize)
// We could have a syntactic sugar to do this (future RFC?)
trait ToT<T> {
    type Value = T;
}
impl<T, A> ToT<T> for A { }


// Example usage
// Reverse the tuple and provide a tuple with the hashes of the tuple members
fn reverse_tuple_and_hash<(..T), (..RevT)>(value: (..T)) -> (<(..T) as Rev>::Value, (..<T as ToT<usize>>::Value)),
where
    (..T): Rev<Value = (..RevT)>,
    ..(T: Hash), {

    let rev_t = <(..T) as Rev>::rev(value);

    // Here we use the identifiers of the reversed variadic tuple and variadic tuple type in the iteration
    let hashes = (for ref rev_t <RevT> in rev_t <RevT> { 
        let mut s = DefaultHasher::new();
        <RevT as Hash>::hash(rev_t, &mut s);
        s.finish()
    });

    (
        (for rev_t @in rev_t { rev_t }), 
        hashes,
    )
}
```

## Declaring and using multiple variadic tuple type with same arity

In C++ multiple parameter packs can be expanded in a single expansion form as long as the packs have the same number of items, but there is no constraint concerning the declaration.

For Rust, using the syntax `(..(T1, T2, ..., Tn))` embeds the constraint that the variadic tuple types `T1`, `T2`, ..., `Tn` have the same arity. This is more consistent than not grouping the declaration (ie: `(..T1), (..T2), ..., (..Tn))`) because the signature using the declaration contains all the information required.

We don't need to look at the implementation or body code to know the required constraint about the variadic tuple type arities.

# Prior art

[prior-art]: #prior-art

C++11 sets a decent precedent with its variadic templates, which can be used to define type-safe variadic functions, among other things. This RFC is comparable to the design of _type parameter packs_ (variadic tuple type) and _function parameter pack_ (variadic tuple).

# Unresolved questions

[unresolved-questions]: #unresolved-questions

## Dynamic libraries tuple implementation assumption

When using dynamic libraries, client libraries may relies that the host contains code up to a specific tuple arity. So we need to have a 
way to enforce the compiler to generate all the implementation up to a specific tuple arity. (12 will keep backward comptibility with current `std` impl)

## Variadic tuple destructuration with multiple `..`

A use case involving destructuration of multiple variadic tuple is a split operator for variadic tuples:

```rust
fn split<(..L), (..R)>(value: (..L, ..R)) -> ((..L), (..R)) {
    // This involves multiple `..` patterns which is not allowed
    // and for a reason, even if we have the variadic tuple types to`"guess"
    // how the split is performed, it is still not explicit
    let (l @ .., r @ ..) = value;
    (l, r)
}
```

Maybe this kind of use case can be solved by annotating the binding:
```rust
fn split<(..L), (..R)>(value: (..L, ..R)) -> ((..L), (..R)) {
    let (l @ .. : (..L), r @ .. : (..R)) = value;
    (l, r)
}
```


# Future possibilities

[future-possibilities]: #future-possibilities

## Supporting variadic tuple for function generic parameter groups with recursion

Supporting variadic tuple for function generic parameter groups requires to provide a specialized implementation for the recursion termination.

For instance:

```rust
fn recurse<Head, (..Tail)>((head, ..tail): (Head, ..Tail))
where Head: Display, ..(Tail: Display) {
  println!("{}", head);
  recurse(tail);
}
// Termination needs to be implemented explicitly
fn recurse<()>((): ()) { }
```

Currently, specialization or overlapping bounds are not permitted for functions. This is a quite big requirement so this feature won't be supported in this RFC.

However when such a feature will land in Rust, supporting variadic tuple for function generic parameter will be way easier.

Note, see the RFC issues [290](https://github.com/rust-lang/rfcs/issues/290) and [1053](https://github.com/rust-lang/rfcs/issues/1053).

## Make enclosing parenthesis optional in variadic tuple declarations

For generic parameter group containing one variadic tuple type, it may be conveninent to omit the parenthesis.

```rust
// Instead of 
struct MyStruct<A, (..B), C>;
// Write
struct MyStruct<A, ..B, C>;

// And the expansions will matches
// Instead of
MyStruct::<usize, (bool, i8, String), i8>;
// Write
MyStruct::<usize, bool, i8, String, i8>;
```

## Syntactic sugar to make enclosing parenthesis optional in for loop

In for loop iterating only on variadic tuples, the parenthesis may be dropped

```rust
// Instead of 
(for (k, v) @in (key, values) {
    ...
})
// Write
(for (k, v) @in key, values {
    ...
})
```


## Syntactic sugar to create tuple with the same type of a specific arity

Consider this use case:
```rust
trait ToT<T> {
    type Value = T;
}
impl<T, A> ToT<T> for A { }

fn tuple_of_hashes<(..T)>((..t): (..T)) -> (..<T as ToT<usize>::Value) {
    (for t in ..t { 
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    })
}
```

We use the `ToT` trait to produce a tuple of `usize` with the same arity of `T`.
We may find a syntactic sugar to do the same thing, like: `(@T..usize)` meaning:
evaluate the `(..usize)` variadic tuple type expansion with the arity of `T`.

So it would be rewritten as:
```rust
fn tuple_of_hashes<(..T)>((..t): (..T)) -> (@T..usize) {
    (for t in ..t { 
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    })
}
```

## Supporting bounds inside generic parameter groups

Writing such code feels natural
```rust
impl<(..(T: Clone))> Clone for (..T) {
    fn clone(&self) -> Self {
        (for c in self { c.clone() })
    }
}
```

So for variadic tuple that are declared alone, we may authorize bound lists.

## Better utilities to manipulate tuples

Some utilities can be provided as libraries (see [Variadic tuple utilities library](##variadic-tuple-utilities-library)), but some will requires implementions provided by the compiler.

Such utilities can be:
```rust
// mod std::tuple
trait Unique<(..T)> {
    // A tuple without members with the same type
    type Value;
}

trait Sorted<(..T)> {
    // A tuple where its members are sorted by TypeId
    type Value;
}

trait Get<(..T)> {
    // Get the value of the ith member of a tuple
    fn get<V>(&self, index: usize) -> Option<&V>;
}
```

## Improve the error message for `E0275`

Improve the error message for `E0275` by providing the sequence of evaluated elements to give more help to the user about what can create the overflow.

- In the context of variadic tuple, this can be the sequence of variadic tuple implementation that are tried by the compiler.
- But, in the more generic case where two traits implementations requires each others, providing the dependency cycle can be really helpful.

## Supporting recursive variadic tuple

Supporting recrusive variadic tuple (ie, declaration like: `(..((..T)))`)
