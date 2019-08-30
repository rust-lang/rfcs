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

The variadic tuple type occurs in two form: a declarative form and an expansion form. And the variadic tuple only occurs in expansion forms.

For a variadic tuple type, the declarative form is `(..#T)` and an example of an expansion form is `(..#Vec<T>)`.

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

1. `(..#T)`: declare a single variadic tuple type identified by `T`
2. `(..#(T1, T2, ..., Tn))`: declare n variadic tuple types identified by `T1`, `T2`, ..., `Tn`, all these variadic tuple types have the same arity.

Declaration examples:

- `struct VariadicStruct<(..#T1)>` : declares a struct with a variadic tuple type identified by `T1` in its generic parameters
- `impl<(..#Head)>`:  is an implementation block that uses a variadic tuple type identified by `Head`
- `impl<A, B, C, (..#_Tail)>`:  same as above, but with other generic parameters
- `fn my_function<(..#A)>`: a function can also have variadic tuple types
- `fn my_function<A, B, (..#C), (..#D)>`: there can be several variadic tuple types declared in a generic parameter group

Usage examples:

```rust
struct VariadicStruct<(..#T)>
VariadicStruct<(usize,)> 				// => (..#T) matches (usize,)
VariadicStruct<(usize, bool)> 	// => (..#T) matches (usize, bool)

fn variadic_fn<(..#(T1, T2))>() { ... }
variadic_fn::<((usize, bool),)
// (..#(T1, T2)) matches ((usize, bool),)
// (..#T1) is (usize,)
// (..#T2) is (bool,)
variadic_fn::<((usize, bool), (String, i8)) // (..#(T1, T2)) matches ((usize, bool), (String, i8))
// (..#T1) is (usize, String)
// (..#T2) is (bool, i8)
```

### Expansion

The expansion syntax is: `..#<expr(T1, T2, ..., Tn)>` where `<expr(T1, T2, ..., Tn)>` is an expression using the variadic tuple type identifiers `T1`, `T2`, ..., `Tn`.

Note: The expression in an expansion form can be enclosed by parenthesis for clarity. Ex: `..#(T: Clone,)`.

The expansion form is allowed in all places where a type is allowed and in `where` bounds.

Examples:

```rust
type TuplesOfRef<'a, (..#T)> = (..#&'a T);
TuplesOfRef<'b, (usize, bool)>; // = (&'b usize, &'b bool)

struct MegaMap<(..#(K, V))> {
  maps: (..#HashMap<K, V>),
}
// 
// struct MegaMap<((usize, bool), (String, i8))> {
//   maps: (HashMap<usize, bool>, HashMap<String, i8>),
// }

fn append<(..#L), (..#R)>(l: (..#L), r: (..#R)) -> (..#L, ..#R)
where ..#(L: 'static + Clone), ..#(R: 'static + Clone) { ... }
//
// append<(usize, Vec<bool>), (&'static str, u8, i16)>(
//     l: (usize, Vec<bool>), 
//     r: (&'static str, u8, i16)
// ) -> (usize, Vec<bool>, &'static str, u8, i16) { ... }
```

Note: If an expansion syntax does not contains any variadic tuple type identifier, it resolves to the unit type `( )`.

Note2: If an expansion syntax contains multiple variadic tuple type identifiers, they must all have been declared together with the syntax `( ..#(T1, T2, ..., Tn))` to ensure they have the same arity.

## Variadic tuple

A _variadic tuple_ is a variable of a variadic tuple type.

### Declaration

A variadic tuple can be declared like any other variable:

```rust
fn my_func<(..#T)>(variadic_tuple: (..#T)) { ... }
```

### Destructuring a variadic tuple

The main way to use a variadic tuple is by destructuring it to access its members.

There are 3 syntaxes possible to destructure a variadic tuple for a variadic tuple `(..#T)`:

1. `(..#v)` of variadic tuple type `(..#T)`
2. `(..#(ref v))` of variadic tuple type `(..#&T)`
3. `(..#(ref mut v))` of variadic tuple type `(..#&mut T)`

Also, the destructure pattern can be combined with other members. For instance:

```rust
{
  let source: (Head, ..#Tail) = _;
  let (ref head, ..#(ref tail)) = &source;
}
{
  let mut source: (..#L, ..#R) = _;
  let (..#(ref mut l), ..#(ref mut r)) = &mut source;
}

```

Examples:

```rust
// The function argument is destructured as a variadic tuple with identifier `v`
fn my_func<(..#T)>((..#v): (..#T)) -> (..#T) { 
	(..#(v + v))
}

impl<(..#T)> Clone for (..#T) 
where ..#(T: Clone) {
  fn clone(&self) -> Self {
    // We destructure `*self` which has a variadic tuple type `(..#T)`
    let (..#(ref v)) = *self;
    (..#v.clone())
  }
}
```

### Expansion

An expansion form for variadic tuple has the syntax: `..#<expr(T1, T2, ..., Tn, id1, id2, ..., idm)>` where `T1`, `T2`, ..., `Tn` are variadic tuple type identifiers and `id1`, `id2`, ..., `idn` are variadic tuple identifiers.

Note 1: All variadic tuple type used in the expansion form must have been declared together. The variadic tuple type used are the variadic tuple types identified by `T1`, `T2`, ..., `Tn` and the type of the variadic tuple identified by `id1`, `id2`, ..., `idn`.

Note 2: An expansion form without any identifier resolves to the unit type `()`.

Note 3: The expression in an expansion form can be enclosed by parenthesis or braces for clarity.

Examples:

```rust
fn my_func<(..#T)>((..#i): (..#T)) {
  (..#{ println!("{}", i) })
}

fn clone_add<(..#T)>((..#i): (..#T)) -> (..#T) 
where ..#(T: Clone + Add) {
  (..#(<T as Clone>::clone(&i) + i))
}

fn merge_into<(..#(L, R))>((..#l): (..#L), (..#r): (..#R)) -> (..#L, ..#L) 
where ..#(L: From<R>) {
   (..#l, ..#(<R as Into<L>>::into(r)))
}
```

## The `Hash`trait

Let's implement the `Hash` trait:

```rust
// For the example, we consider the impl for (A, B, C). So `(..#T)` matches `(A, B, C)`
// We have the first expansion here, `(..#T, Last)` expands to `(A, B, C, Last)`
impl<(..#T), Last> Hash for (..#T, Last) 
where
		// Expands to `A: Hash, B: Hash, C: Hash,`
    ..#(T: Hash,),
    Last: Hash + ?Sized, {

    #[allow(non_snake_case)]
    fn hash<S: Hasher>(&self, state: &mut S) {
      	// Destructure self to a variadic tuple `v` and a variable `last`. The variadic tuple type of `v` is `(..#&T)`
      	// So it will be equivalent to `let (ref a, ref b, ref c, ref last) = *self; let v = (a, b, c);`
        let (..#(ref v), ref last) = *self;			 
      	
      	// Expands to `(v.0.hash(state), v.1.hash(state), v.2.hash(state), last.hash(state));`
        (..#v.hash(state), last.hash(state));   
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
fn my_func<(..#(L, R))>() { }
my_func::<((usize, bool), (i8, f32))>();
// `(..#(L, R))` matches `((usize, bool), (i8, f32))`
// `(..#L)` is `(usize, i8)`
// `(..#R)` is `(bool, f32)`
```

Note: Although this looks like a type-level pattern matching, it can match against only tuple of identifiers. So the following declaration is invalid: `fn my_func<(..#(L, Vec<R>))>() { ... }`

### Variadic tuple type expansion

On location where a type is expected, the expansion will resolve to a type. On where bounds it can be used to declare bounds on the type contained in the variadic tuple type.

Examples:

```rust
type TupleOfVec<(..#T)> = (..#Vec<T>);

fn my_func<(..#T)>() 
where ..#(T: Clone) { ... }
```

### Variadic tuple declaration

The declaration of a variadic tuple variable is still a variable. Nothing new here.

```rust
fn my_func<(..#T)>(input: (..#T)) { ... }
```

### Variadic tuple destructuration

When destructuring a variadic tuple it declares a variadic tuple identifiers that can be used in expansion forms. The identifier is a variable of type `(..#T)` or `(..#&T)` or `(..#&mut T)`, depending on the syntax used.

```rust
{
  let source: (..#T, Tail) = _;
  let (..#v, tail) = source;
  // v is a variable of type `(..#T)`
  let (..#(ref v), ref tail) = &source;
  // v is a variable of type `(..#&T)`
  let (..#(ref mut v), ref mut tail) = &mut source;
    // v is a variable of type `(..#&mut T)`
}

// If we use `(..#T)` = `(A, B, C)` as an example
// Then `let (..#(ref v), ref tail) = &source`
// is equivalent to:
// `let (ref a, ref b, ref c, ref tail) = &source;`
// `let v = (a, b, c);`
```

### Variadic tuple expansion

The variadic tuple expansion are "expression template". By replacing the identifiers by its appropriate value, the variadic tuple expansion will result in a list of expressions. The full expression form will be replaced by the resolved list of expression.

```rust
fn my_function<(..#T)>((..#i): (..#T)) {
  (..#i.clone())
  // `i.clone()` is the expression template parameterized by `i`
  // it will resolve into a list of expressions: `i.0.clone(), i.1.clone(), ..., i.n.clone()`
  // Finally, `..#i.clone()` will be replaced by the resolved list of expressions
}
```

## Recursion

To implement some feature, we may want to use recursion over the arity of the tuple.
For instance, let's implement a trait that gives the arity of a tuple as a `const` value:

```rust
trait Arity {
    const VALUE: usize;
}

impl<Head, (..#Tail)> Arity for (Head, ..#Tail) {
    const VALUE: usize = <(..#Tail) as Arity>::VALUE + 1;
}
impl Arity for () {
    const VALUE: usize = 0;
}
```

Note:

- The `impl<Head, (..#Tail)> Arity for (Head, ..#Tail)` is the recursive implementation.
- The `impl Arity for ()` is the termination of the recursive implementation.

And when we compile the following code:

```rust
fn main() {
    println!("Arity of (bool, usize): {}", <(bool, usize) as Arity>::VALUE);
}
```

The compiler will execute these steps:

1. Search `impl` of `Arity` for `(bool, usize)`
2. `impl` not found, Search variadic `impl` of `Arity` for `(bool, usize)`
3. Variadic impl found: `impl<Head, (..#Tail)> Arity for (Head, ..#Tail)`
4. Generate `impl` of `Arity` for `(bool, usize)`
   1. Requires `impl` of `Arity` for `(usize,)`
   2. Search `impl` of `Arity` for `(usize,)`
   3. `impl` not found, Search variadic `impl` of `Arity` for `(usize,)`
   4. Variadic impl found: `impl<Head, (..#Tail)> Arity for (Head, ..#Tail)`
   5. Generate `impl` of `Arity` for `(usize,)`
      1. Requires `impl` of `Arity` for `()`
      2. Search `impl` of `Arity` for `()`
      3. `impl` found
   6. Generation of `impl` of `Arity` for `(usize,)` completed
5. Generation of `impl` of `Arity` for `(bool, usize)` completed

### Recursion with functions

```rust
fn recurse<Head, (..#Tail)>((head, ..#tail): (Head, ..#Tail))
where Head: Debug, ..#(Tail: Debug) {
  println!("{}", head);
  recurse((..#tail));
}
// Termination needs to be implemented explicitly
fn recurse<()>((): ()) { }
```

## Errors

#### Missing implementation message during variadic implementation resolution

An error can occur if the compiler don't find an implementation while generating variadic tuple implementations.

Let's consider this code:

```rust
trait Arity {
    const VALUE: usize;
}

impl<Head, (..#Tail)> Arity for (Head, ..#Tail) {
    const VALUE: usize = <(..#Tail) as Arity>::VALUE + 1;
}

fn main() {
    let arity = <(usize, bool, u8) as Arity>::VALUE;
}
```

This code must not compile as the termination implementation is missing.
So we will have a `E0277`error:

```rust
error[E0277]: the trait bound `(): Arity` is not satisfied
 --> src/main.rs:10:4
  |
7 |     let arity = <(usize, bool, u8) as Arity>::VALUE;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Arity` is not implemented for `()`
  |
  = help: impl `Arity` for `(usize, bool, u8)` requires impl `Arity` for `(bool, u8)`
    note: matched by variadic tuple impl of `Arity`
 --> src/main.rs:5:1
  |
5 | impl<Head, (..#Tail)> Arity for (Head, ..#Tail) {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = help: impl `Arity` for `(bool, u8)` requires impl `Arity` for `(u8,)`.
    note: matched by variadic tuple impl of `Arity`
 --> src/main.rs:5:1
  |
5 | impl<Head, (..#Tail)> Arity for (Head, ..#Tail) {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = help: impl `Arity` for `(u8,)` requires impl `Arity` for `()`.
    note: matched by variadic tuple impl of `Arity`
 --> src/main.rs:5:1
  |
5 | impl<Head, (..#Tail)> Arity for (Head, ..#Tail) {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

#### Variadic implementation cycle dependency error

As a variadic tuple implementation may depend on other variadic tuple implementation, there can be dependency cycle issue.

```rust
trait A { const VALUE: usize = 1; }
trait B { const VALUE: usize = 2; }

impl<Head, (..#T)> A for (Head, ..#T) 
where (..#T): B { const VALUE: usize = 3; }

impl<Head, (..#T)> B for (Head, ..#T) 
where (..#T): A { const VALUE: usize = 4; }

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

TODO

### Invalid variadic tuple declaration

TODO

### Invalid variadic tuple type expansion identifiers

TODO: Add an error when multiple independent variadic tuple identifier are used in a single expansion form.
ie: `fn my_func<(..#K), (..#V)>() -> (..#HashMap<K, V>) { ... }` -> `K` and `V` may have different arities.

### Invalid variadic tuple expansion identifiers

TODO: Occurs when a variadic tuple expansion form used variadic tuple types with different arities (not declared together)

 ```rust
 fn my_func<(..#L), (..#R)>((..#l): (..#L), (..#r): (..#R)) { let _ = (..#(l, r)); }
 ```

### Invalid variadic tuple destructuration

TODO

## Help and note for existing errors

Variadic tuple expansion will generate code and may produce obscure errors for existing compile error. To help user debug their compile issue, we need to provide information about the expansion the compiler tried to resolve.

##### Unknown identifier in an expansion form

If we consider this code:

```rust
fn make_mega_map<(..#(Key, Value))>() -> (..#HashMap<Key, Value>) {
  (..#HashMap::<Key, Value2>::new())
}

fn main() {
  let mega_map = make_mega_map::<(usize, bool), (f32, String)>();
}
```

Then the expansion form is valid, even though the `Value2` identifier is probably mistyped.
In that case, the expansion will be resolved as:

```rust
fn make_mega_map<(usize, bool), (f32, String)>() -> (HashMap<usize, bool>, HashMap<f32, String>) {
  (HashMap::<usize, Value2>::new(), HashMap::<f32, Value2>::new())
}
```

Leading to a compile error with additional notes

```rust
error[E0412]: cannot find type `Value2` in this scope
  --> src/main.rs:10:22
   |
10 |  let mega_map = make_mega_map::<(usize, bool), (f32, String)>();
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ not found in this scope
note: when expanding with `(..#(Key, Value)) = ((usize, bool), (f32, String))`
  --> src/main.rs:2:4
   |
2  |    (..#HashMap::<Key, Value2>::new())
   |
```

# Drawbacks

[drawbacks]: #drawbacks

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

# Prior art

[prior-art]: #prior-art

C++11 sets a decent precedent with its variadic templates, which can be used to define type-safe variadic functions, among other things. This RFC is comparable to the design of _type parameter packs_ (variadic tuple type) and _function parameter pack_ (variadic tuple).

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- When using dynamic libraries, client libraries may relies that the host contains code up to a specific tuple arity. So we need to have a 
  way to enforce the compiler to generate all the implementation up to a specific tuple arity. (12 will keep backward comptibility with current `std` impl)

# Future possibilities

[future-possibilities]: #future-possibilities

- Improve the error message for `E0275` by providing the sequence of evaluated elements to give more help to the user about what can create the overflow.
  - In the context of variadic tuple, this can be the sequence of variadic tuple implementation that are tried by the compiler.
  - But, in the more generic case where two traits implementations requires each others, providing the dependency cycle can be really helpful.

- Supporting recrusive variadic tuple (ie, declaration like: `(..#((..#T)))`)
