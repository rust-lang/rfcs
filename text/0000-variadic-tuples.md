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

Variadic tuple will provide several benefits considering trait implementation for tuple or using tuples:

- Implementations will be easier to write
- Implementations will be easier to read and maintain
- The compiler will compile implementation only for required tuple arity

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Let's call a _variadic tuple type_ a tuple type with an arbitrary arity and a _variadic tuple_ an instance of a variadic tuple type.

A variadic tuple type is declared with`(..T)` and a variadic tuple type can be expanded with `(..Vec<T>)`.

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

*Note: variadic tuples are not supported in generic parameter group of `fn`*

There are two different syntaxes:

1. `(..T)`: declare a single variadic tuple type identified by `T`
2. `(..(T1, T2, ..., Tn))`: declare n variadic tuple types identified by `T1`, `T2`, ..., `Tn`, all these variadic tuple types have the same arity.

Declaration examples:

- `struct VariadicStruct<(..T1)>` : declares a struct with a variadic tuple type identified by `T1` in its generic parameters
- `impl<(..Head)>`:  is an implementation block that uses a variadic tuple type identified by `Head`
- `impl<A, B, C, (.._Tail)>`:  same as above, but with other generic parameters
- `impl<A, B, (..C), (..D)>`: there can be several variadic tuple types declared in a generic parameter group

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

trait Append<(..L), (..R)>
where ..(L: 'static + Clone), ..(R: 'static + Clone) {
    fn append(l: (..L), r: (..R)) -> (..L, ..R)
}

//
// trait Append<(usize, Vec<bool>), (&'static str, u8, i16)> {
//  fn append(
//       l: (usize, Vec<bool>), 
//       r: (&'static str, u8, i16)
//   ) -> (usize, Vec<bool>, &'static str, u8, i16) { ... }
// }
```

Note: If an expansion syntax does not contains any variadic tuple type identifier, it resolves to the unit type `( )`.

Note2: If an expansion syntax contains multiple variadic tuple type identifiers, they must all have been declared together with the syntax `( ..(T1, T2, ..., Tn))` to ensure they have the same arity.

## Variadic tuple

A _variadic tuple_ is a variable of a variadic tuple type.

### Declaration

A variadic tuple can be declared like any other variable:

```rust
trait MyFunc<(..T)> {
    fn my_func(variadic_tuple: (..T)) { ... }
}
```

### Destructuring a variadic tuple

The main way to use a variadic tuple is by destructuring it to access its members.

There are 3 syntaxes possible to destructure a variadic tuple for a variadic tuple `(..T)`:

1. `(..v)` of variadic tuple type `(..T)`
2. `(..(ref v))` of variadic tuple type `(..&T)`
3. `(..(ref mut v))` of variadic tuple type `(..&mut T)`

Also, the destructure pattern can be combined with other members. For instance:

```rust
{
  let source: (Head, ..Tail) = _;
  let (ref head, ..(ref tail)) = &source;
}
{
  let mut source: (..L, ..R) = _;
  let (..(ref mut l), ..(ref mut r)) = &mut source;
}

```

Examples:

```rust
// The function argument is destructured as a variadic tuple with identifier `v`
trait MyFunc<(..T)> {
    fn my_func((..v): (..T)) -> (..T) { 
        ...
    }
}

impl<(..T)> Clone for (..T) 
where ..(T: Clone) {
  fn clone(&self) -> Self {
    // We destructure `*self` which has a variadic tuple type `(..T)`
    let (..(ref v)) = *self;
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
    // `key` and `map` are variables iterating the variadic tuples `(..k): (..K)` and `(..maps): (..&HashMap<K, V>)`, `key` will iterate by reference (because of the ref keyword)
    // `KEY` and `VALUE` are type variables iterating the variadic tuple types `(..K)` and `(..V)`
    // `..(k, maps)` declares the iterated variadic tuples `(..k)` and `(..maps)`
    // `..(K, V)` declares the iterated variadic tuple types
    (for (ref key, map) type (KEY, VALUE) in ..(k, maps) type ..(K, V) {
        HashMap::<KEY, VALUE>::get(&map, key)
    })
};
```

Note: when iterating over multiple variadic tuple or variadic tuple types, they must have all the same arity. To ensure this, all variadic tuple types involved must have been declared together.

Examples:

```rust
impl<(..(K, V))> MegaMap<(..(K, V))>
where ..(K: Hash), {
    fn get(&self, (..k): (..K)) -> (..Option<V>) {
        let (..ref maps) = &self.maps;

        let result: (..Option<&V>) = {
            (for (ref k, map) type (K, V) in ..(k, maps) type ..(K, V) {
                HashMap::<K, V>::get(&map, k)
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
        let (..ref tuple, ref last) = *self;
       
        // Use case: only variadic tuple
        (for member in ..(tuple,) {
          member.hash(state);
        });
        last.hash(state);

        // Use case: variadic tuple and type
        (for member type (H,) in ..(tuple,) type ..(T,) {
          <T as Hash>::hash(&member, state);
        });
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
        let (..l) = self;
        (
            for (l1,) in ..(l,) { l1 },
            for (r1,) in ..(r,) { r1 },
        )
    }
}
```

## The `Hash`trait

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
        let (..ref tuple, ref last) = *self;
        (for member in ..(tuple,) {
          member.hash(state);
        });
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

Note: Although this looks like a type-level pattern matching, it can match against only tuple of identifiers. So the following declaration is invalid: `struct MyStruct<(..(L, Vec<R>))> { ... }`

### Variadic tuple type expansion

On location where a type is expected, the expansion will resolve to a type. On where bounds it can be used to declare bounds on the type contained in the variadic tuple type.

Examples:

```rust
type TupleOfVec<(..T)> = (..Vec<T>);

struct MyStruct<(..T)>
where ..(T: Clone) { ... }
```

### Variadic tuple declaration

The declaration of a variadic tuple variable is still a variable. Nothing new here.

```rust
trait MyFunc<(..T)> {
    fn my_func(input: (..T)) { ... }    
}

```

### Variadic tuple destructuration

When destructuring a variadic tuple it declares a variadic tuple identifiers that can be used in expansion forms. The identifier is a variable of type `(..T)` or `(..&T)` or `(..&mut T)`, depending on the syntax used.

```rust
{
  let source: (..T, Tail) = _;
  let (..v, tail) = source;
  // v is a variable of type `(..T)`
  let (..(ref v), ref tail) = &source;
  // v is a variable of type `(..&T)`
  let (..(ref mut v), ref mut tail) = &mut source;
    // v is a variable of type `(..&mut T)`
}

// If we use `(..T)` = `(A, B, C)` as an example
// Then `let (..(ref v), ref tail) = &source`
// is equivalent to:
// `let (ref a, ref b, ref c, ref tail) = &source;`
// `let v = (a, b, c);`
```

### Variadic tuple iteration

The syntax for the variadic tuple iteration is:

```rust
for $var_id type $type_var_id in $variadic_tuples type $variadic_tuple_types {
    $body
}
```

`$var_id` is a pattern matching the tuple to iterate, it follows the same rules as the variadic tuple destructuration, only 3 syntaxes are allowed for an identifier: `id`, `ref id` or `ref mut id`. (like: `(key value)`, `(ref key, value)`, `(ref mut key, value)`)

`$type_var_id` is a pattern matching the variadic tuple types to iterate, but it has only the first syntax allowed. (No ref, or mut).

`$variadic_tuples` declares the iterated variadic tuples, it has the syntax `..id` or `..(id1, id2, ..., idn)`.

`$variadic_tuple_types` declares the iterated variadic tuple types, it has the syntax `..ID` or `..(ID1, ID2, ..., IDn)`.

Example:

```rust
impl<(..(K, V))> MegaMap<(..(K, V))>
where ..(K: Hash), {
    fn get(&self, (..k): (..K)) -> (..Option<V>) {
        let (..ref maps) = &self.maps;

        let result: (..Option<&V>) = {
            (for (ref k, map) type (K, V) in ..(k, maps) type ..(K, V) {
                HashMap::<K, V>::get(&map, k)
            })
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



## Recursion

To implement some feature, we may want to use recursion over the arity of the tuple.
For instance, let's implement a trait that gives the arity of a tuple as a `const` value:

```rust
trait Arity {
    const VALUE: usize;
}

// Default implementation for all tuples
impl<(..T)> Arity for (..T) {
    default const VALUE: usize = 0;
}

// Specialized implementation for the recursion
impl<Head, (..Tail)> Arity for (Head, ..Tail) {
    const VALUE: usize = <(..Tail) as Arity>::VALUE + 1;
}
```

Note:

- The `impl<Head, (..Tail)> Arity for (Head, ..Tail)` is the recursive implementation.
- The `impl<(..T)> Arity for (..T)` is the default implementation and will act as the termination of the recursion.

## Errors

#### Missing implementation message during variadic implementation resolution

An error can occur if the compiler don't find an implementation while generating variadic tuple implementations.

Let's consider this code:

```rust
trait Arity {
    const VALUE: usize;
}

impl<Head, (..Tail)> Arity for (Head, ..Tail) {
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
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Arity` is not implemented for `(bool, u8)`
  |
  = help: impl `Arity` for `(usize, bool, u8)` requires impl `Arity` for `(bool, u8)`
```

#### Variadic implementation cycle dependency error

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
10 |  impl<(..#K), (..#V)> MyTrait for (..HashMap<K, V>) {
   |                                   ^^^^^^^^^^^^^^^^^
   |
note: variadic tuple type identifiers `K`, `V` were not declared together
  --> src/main.rs:4:13
   |
10 |  impl<(..K), (..V)> MyTrait for (..HashMap<K, V>) {
   |       ^^^^^^^^^^^^
   |
hint: expected `(..(K, V))`
```

### Invalid variadic tuple pattern matching

The variadic tuple declaration is invalid when it can't be parsed as either:

- `(..id)`
- `(..(ref id))`
- `(..(ref mut id))`

```rust
struct MyStruct<(..Vec<T>)> {
  vecs: (..Vec<T>)
}

impl<(..T)> MyTrait for (..#T) {
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
note: expected `(..id)` or `(..(ref id))` or `(..(ref mut id))`
```

## Help and note for existing errors

Variadic tuple expansion will generate code and may produce obscure errors for existing compile error. To help user debug their compile issue, we need to provide information about the expansion the compiler tried to resolve.

##### Unknown identifier in an expansion form

If we consider this code:

```rust
trait MakeMegaMap<(..(Key, Value))> {
    fn make_mega_map() -> (..HashMap<Key, Value>) {
        for () type (KEY, VALUE) in () type ..(Key, Value2) {
            HashMap::<KEY, VALUE>::new()
        }
    }
}

impl<(..(Key, Value))> MakeMegaMap<(..(Key, Value))> for () {}

fn main() {
  let mega_map = <() as MakeMegaMap<<(usize, bool), (f32, String)>>::make_mega_map();
}
```

Then the expansion form is valid, even though the `Value2` identifier is probably mistyped.
Leading to a compile error with additional notes

```rust
error[E0412]: cannot find type `Value2` in this scope
  --> src/main.rs:10:22
   |
10 |  let mega_map = <() as MakeMegaMap<<(usize, bool), (f32, String)>>::make_mega_map();
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ not found in this scope
note: when expanding with `(..(Key, Value)) = ((usize, bool), (f32, String))`
  --> src/main.rs:2:4
   |  for () type (KEY, VALUE) in () type ..(Key, Value2) {
2  |    HashMap::<KEY, VALUE>::new()              ^^^^^^^
   |  }
```

# Drawbacks

[drawbacks]: #drawbacks

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

## Recursive implementations of trait with variadic tuple

Recursive implementation is done by implementing the termination on all types and then the recursive implementation as a specialization.

Doing this way, the Rust compiler is able to validate the generic parameters before monomorphization.

```rust
trait Arity {
    const VALUE: usize;
}

// Default implementation for all tuples
impl<(..T)> Arity for (..T) {
    default const VALUE: usize = 0;
}

// Specialized implementation for the recursion
impl<Head, (..Tail)> Arity for (Head, ..Tail) {
    // (..#Tail) does implement `Arity`, so 
    // <(..#Tail) as Arity> is valid.
    const VALUE: usize = <(..Tail) as Arity>::VALUE + 1;
}
```

## Iteration over variadic tuples syntax

When iterating over variadic tuples, we need to define both variable and type variable. To do so, we use the for loop syntax and separate variables and type variables with the `type` keyword.

This keyword is already reserved and has no meaning inside a for loop, so it can be used here.

```rust
let result: (..Option<&V>) = {
    for (ref k, map) type (K, V) in ..(k, maps) type ..(K, V) {
        HashMap::<K, V>::get(&map, k)
    }
};
```



## Declaring and using multiple variadic tuple type with same arity

In C++ multiple parameter packs can be expanded in a single expansion form as long as the packs have the same number of items, but there is no constraint concerning the declaration.

For Rust, using the syntax `(..(T1, T2, ..., Tn))` embeds the constraint that the variadic tuple types `T1`, `T2`, ..., `Tn` have the same arity. This is more consistent than not grouping the declaration (ie: `(..T1), (..#T2), ..., (..Tn))`) because the signature using the declaration contains all the information required.

We don't need to look at the implementation or body code to know the required constraint about the variadic tuple type arities.

# Prior art

[prior-art]: #prior-art

C++11 sets a decent precedent with its variadic templates, which can be used to define type-safe variadic functions, among other things. This RFC is comparable to the design of _type parameter packs_ (variadic tuple type) and _function parameter pack_ (variadic tuple).

# Unresolved questions

[unresolved-questions]: #unresolved-questions

## Dynamic libraries tuple implementation assumption

When using dynamic libraries, client libraries may relies that the host contains code up to a specific tuple arity. So we need to have a 
way to enforce the compiler to generate all the implementation up to a specific tuple arity. (12 will keep backward comptibility with current `std` impl)

# Future possibilities

[future-possibilities]: #future-possibilities

## Supporting variadic tuple for function generic parameter groups

Supporting variadic tuple for function generic parameter groups requires to provide a specialized implementation for the recursion termination.

For instance:

```rust
fn recurse<Head, (..Tail)>((head, ..tail): (Head, ..Tail))
where Head: Display, ..(Tail: Display) {
  println!("{}", head);
  recurse((..tail));
}
// Termination needs to be implemented explicitly
fn recurse<()>((): ()) { }
```

Currently, specialization or overlapping bounds are not permitted for functions. This is a quite big requirement so this feature won't be supported in this RFC.

However when such a feature will land in Rust, supporting variadic tuple for function generic parameter will be way easier.

Note, see the RFC issues [290](https://github.com/rust-lang/rfcs/issues/290) and [1053](https://github.com/rust-lang/rfcs/issues/1053).

## Better utilities to manipulate tuples

If we consider tuples as a list of types we can perform more computation at compile time and provide more possibilities for zero cost abstractions.

Such utilities can be:

- `TupleContains<T>`:  implemented by tuples containing the type `T` in its members
- `UniqueTuple`: implemented by all tuple, an associated type is the based on the same tuple but with only unique types
- `SortedTuple`: implemented by all tuple, an associated type is the based on the same tuple but with only sorted types
- `Arity`: a trait implemented by all tuple providing the arity of the tuple with a `const`
- An equivalent to C++'s  `std::get`

Those are not directly related to this RFC, but those utilities will be a natural additional step to better support tuple.

## Improve the error message for `E0275`

Improve the error message for `E0275` by providing the sequence of evaluated elements to give more help to the user about what can create the overflow.

- In the context of variadic tuple, this can be the sequence of variadic tuple implementation that are tried by the compiler.
- But, in the more generic case where two traits implementations requires each others, providing the dependency cycle can be really helpful.

## Supporting recursive variadic tuple

Supporting recrusive variadic tuple (ie, declaration like: `(..((..T)))`)
