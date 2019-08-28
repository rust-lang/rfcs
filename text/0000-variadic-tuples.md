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

The variadic tuple occurs in two form: a declarative form and an expansion form.

The declarative form is `(..#T)` and an expansion form is `(T#..)`

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

## Declaring a variadic tuple

To declare a variadic tuple, we use `(..#T)`, where `T` is a type identifier.

For instance:

- `struct VariadicStruct<(..#T1)>` : declares a struct with a variadic tuple  identified by `T1` in its generic parameters
- `impl<(..#Head)>`:  is an implementation block that uses a variadic tuple identified by `Head`
- `impl<A, B, C, (..#_Tail)>`:  same as above, but with other generic parameters
- `fn my_function<(..#A)>`: a function can also have variadic tuple
- `fn my_function<A, B, (..#C), (..#D)>`: there can be several variadic tuple declared in a generic parameter group

You can think this like a rule you give to the compiler to generated appropriate code when it runs into specific patterns:

- `VariadicStruct<(int, usize)>` matches `VariadicStruct<(..#T1)>` where `(..#T1)` maps to `(int, usize)`
- `VariadicStruct<(int, usize, usize)>` matches `VariadicStruct<(..#T1)>` where `(..#T1)` maps to `(int, usize, usize)`
  (We will see implementation examples later, with the expansion form)

## Expanding variadic tuple

At some point, we need to use the types that are declared in the declaration form, this is where we use the expansion form.

When expanding a tuple, we use the form `T#..`, but more generally: `<expr(T)>#..` where `<expr(T)>` is pattern optionally enclosed by parenthesis using the identifier `T`.

Let's implement the `Hash` trait:

```rust
// For the example, we consider the impl for (A, B, C). So `(..#T)` matches `(A, B, C)`
// We have the first expansion here, `(T#.., Last)` expands to `(A, B, C, Last)`
impl<(..#T), Last> Hash for (T#.., Last) 
where
    (T: Hash)#..,                               // Expands to `A: Hash, B: Hash, C: Hash,`
    Last: Hash + ?Sized, {

    #[allow(non_snake_case)]
    fn hash<S: Hasher>(&self, state: &mut S) {
        let ((ref T)#.., ref last) = *self;     // Expands to `let (ref A, ref B, ref C, ref last) = *self;`
        (T.hash(state)#.., last.hash(state));   // Expands to `(A.hash(state), B.hash(state), C.hash(state), last.hash(state));`
    }
}
```

## Allowed usages of variadic tuples

### Declarative form

- Struct generic parameters     : `struct MyStruct<(..#T)>`
- Function generic parameters   : `fn my_function<(..#T)>`
- Type alias declaration        : `type MyTuple<(..#T)>`
- impl block generic parameters : `impl<(..#T)>`

### Expansion form

- Struct member declaration

  ```rust
  struct MyStruct<(..#T)> {
    arrays: ([T; 32]#..),
  }
  ```

- Function arguments

```rust
fn my_function<(..#T)>(values: &(Vec<T>#..))
```

- Function return type

```rust
fn my_function<(..#T)>(values: &(Vec<T>#..)) -> (&[T]#..)
```

- Function body

```rust
fn my_function<(..#T)>(values: &(Vec<T>#..)) -> (&[T]#..) {
    let ((ref T)#..) = values;
    (T#..)
}
```

- Type alias definition

```rust
type TupleOfVec<(..#T)> = (Vec<T>#..);
```

- impl block type

```rust
impl<(..#T)> MyStruct<(HashMap<usize, T>#..)>
```

- where clause

```rust
impl<(..#T)> MyStruct<(T#..)>
where (T: Hash)#..
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

## Recursion

To implement some feature, we may want to use recursion over the arity of the tuple.
For instance, let's implement a trait that gives the arity of a tuple as a `const` value:

```rust
trait Arity {
    const VALUE: usize;
}

impl<Head, (..#Tail)> Arity for (Head, Tail#..) {
    const VALUE: usize = <(Tail#..) as Arity>::VALUE + 1;
}
impl Arity for () {
    const VALUE: usize = 0;
}
```

Note:

- The `impl<Head, (..#Tail)> Arity for (Head, Tail#..)` is the recursive implementation.
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
3. Variadic impl found: `impl<Head, (..#Tail)> Arity for (Head, Tail#..)`
4. Generate `impl` of `Arity` for `(bool, usize)`
   1. Requires `impl` of `Arity` for `(usize,)`
   2. Search `impl` of `Arity` for `(usize,)`
   3. `impl` not found, Search variadic `impl` of `Arity` for `(usize,)`
   4. Variadic impl found: `impl<Head, (..#Tail)> Arity for (Head, Tail#..)`
   5. Generate `impl` of `Arity` for `(usize,)`
      1. Requires `impl` of `Arity` for `()`
      2. Search `impl` of `Arity` for `()`
      3. `impl` found
   6. Generation of `impl` of `Arity` for `(usize,)` completed
5. Generation of `impl` of `Arity` for `(bool, usize)` completed

### Recursion errors

#### Missing implementation message

An error can occur if the compiler don't find an implementation while generating variadic tuple implementations.

Let's consider this code:

```rust
trait Arity {
    const VALUE: usize;
}

impl<Head, (..#Tail)> Arity for (Head, Tail#..) {
    const VALUE: usize = <(Tail#..) as Arity>::VALUE + 1;
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
5 | impl<Head, (..#Tail)> Arity for (Head, Tail#..) {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = help: impl `Arity` for `(bool, u8)` requires impl `Arity` for `(u8,)`.
    note: matched by variadic tuple impl of `Arity`
 --> src/main.rs:5:1
  |
5 | impl<Head, (..#Tail)> Arity for (Head, Tail#..) {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = help: impl `Arity` for `(u8,)` requires impl `Arity` for `()`.
    note: matched by variadic tuple impl of `Arity`
 --> src/main.rs:5:1
  |
5 | impl<Head, (..#Tail)> Arity for (Head, Tail#..) {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

#### Cycle dependency error

As a variadic tuple implementation may depend on other variadic tuple implementation, there can be dependency cycle issue.

```rust
trait A { const VALUE: usize = 1; }
trait B { const VALUE: usize = 2; }

impl<Head, (..#T)> A for (Head, T#..) 
where (T#..): B { const VALUE: usize = 3; }

impl<Head, (..#T)> B for (Head, T#..) 
where (T#..): A { const VALUE: usize = 4; }

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

## Using multiple variadic tuples

### Recursive variadic tuples

Recursive variadic tuples are forbidden.
Example: 

```rust
fn my_func<(..#((..#T)))>() { ..}
```

### Expansion forms without variadic tuple identifiers

Expansion forms without variadic tuple identifiers are forbidden. 

### Single identifier expansion forms

We can declare and use multiple variadic tuples, if the expansion forms only involve a single variadic tuple identifier, there is no constraints.

See this example of a `trait`that merge two tuples:

```rust
trait Merge<T> {
    type Out;

    fn append(self, value: T) -> Self::Out;
}

impl<(..#L), (..#R)> Merge<(R#..)> for (L#..) {
    type Out = (L#.., R#..);

    fn merge(self, value: (R#..)) -> Self::Out; {
        let (L#..) = self;
        let (R#..) = value;
        (L#.., R#..)
    }
}
```

Note: a variadic tuple identifier may occur more than once in an expansion form, for instance:

```rust
fn double<(..#T)>(input: (#T..)) -> (T#..)
	where (T: Add)#.., {
    ({T + T}#..)
}
```

### Multiple identifier expansion forms

An expansion form may include multiple different variadic tuple identifiers. However, both variadic tuple must have the same arity.

For instance, let's consider this `struct`:

```rust
struct MegaMap<(..#Key), (..#Value)> {
  maps: (HashMap<Key, Value>#..)
}
```

Then the following usages are valid:

```rust
MegaMap<(usize,), (bool,)>
MegaMap<(usize, i8), (String, Vec<usize>)>
```

And these one are invalid:

```rust
MegaMap<(usize,), (bool, bool)>
MegaMap<(usize, bool, String), (usize, bool)>
```

### Expansion errors

Variadic tuple expansions have their specific constraints, and if violated the compiler needs to issue an error.

Also, variadic tuple expansion will generate code and may produce obscure errors for existing compile error. To help user debug their compile issue, we need to provide information about the expansion the compiler tried to resolve.

#### Invalid variadic tuple expansion error

This will happen when the compiler tries to expand an expansion form with invalid variadic tuple.
We need to introduce a new compile error for this one, let's call it `EXXXX`

There are two kinds of invalid expansion errors

- No variadic tuple identifier is found in the expansion form
- The expansion form contains multiple variadic tuple identifiers, but those have different arities

##### Different arities in an expansion form error 

So, the following code

```rust
struct MegaMap<(..#Key), (..#Value)> {
  maps: (HashMap<Key, Value>#..)
}

impl<(..#Key), (..#Value)> for MegaMap<(Key#..), (Value#..)> {
    pub fn new() -> Self {
			  Self {
            maps: (HashMap<Key, Value>::new()#..)
        }
	  }
}

fn main() {
  let mega_map: MegaMap<(bool,), (usize, String)> = MegaMap::new();
}
```

Will produce this error

```rust
error[EXXXX]: variadic tuple expansion form `(HashMap<Key, Value>::new()#..)` can't be expanded
  --> src/main.rs:8:17
   |
10 |     maps: (HashMap<Key, Value>::new()#..)
   |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: when expanded with different variadic tuple arity `(..#Key) = (bool,)` and `(..#Value) = (usize, String)`
  --> src/main.rs:14:16
   |
14 | let mega_map: MegaMap<(bool,), (usize, String)> = MegaMap::new();
   |               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

#### Missing variadic tuple identifier error

The following code

```rust
fn make_mega_map<(..#Key), (..#Value)>() -> (HashMap<Key, Value>#..) {
  (HashMap::<Key2, Value2>::new()#..)
}

fn main() {
  let mega_map = make_mega_map::<(usize, bool), (bool, String)>();
}
```

Will produce this error

```rust
error[EXXXX]: variadic tuple expansion form `(HashMap::<Key2, Value2>::new()#..)` can't be expanded
  --> src/main.rs:2:4
   |
2  |  (HashMap::<Key2, Value2>::new()#..)
   |  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: no variadic tuple identifier was found in the expansion form
note: when expanding with `(..#Key) = (usize, bool)` and `(..#Value) = (bool, String)`
  --> src/main.rs:6:16
   |
6  | let mega_map = make_mega_map::<(usize, bool), (bool, String)>();
   |                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

#### Help and note for existing errors

##### Unknown identifier in an expansion form

If we consider this code:

```rust
fn make_mega_map<(..#Key), (..#Value)>() -> (HashMap<Key, Value>#..) {
  (HashMap::<Key, Value2>::new()#..)
}

fn main() {
  let mega_map = make_mega_map::<(usize, bool), (bool, String)>();
}
```

Then the expansion form is valid, even though the `Value2` identifier is probably mistyped.
In that case, the expansion will be resolved as:

```rust
fn make_mega_map<(usize, bool), (bool, String)>() -> (HashMap<usize, bool>, HashMap<bool, String>) {
  (HashMap::<usize, Value2>::new(), HashMap::<bool, Value2>::new())
}
```

Leading to a compile error with additional notes

```rust
error[E0412]: cannot find type `Value2` in this scope
  --> src/main.rs:10:22
   |
10 |  let mega_map = make_mega_map::<(usize, bool), (bool, String)>();
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ not found in this scope
note: when expanding with `(..#Key) = (usize, bool)` and `(..#Value) = (bool, String)`
  --> src/main.rs:2:4
   |
2  |    (HashMap::<Key, Value2>::new()#..)
   |
```

# Drawbacks

[drawbacks]: #drawbacks

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

# Prior art

[prior-art]: #prior-art

C++11 sets a decent precedent with its variadic templates, which can be used to define type-safe variadic functions, among other things. C++11 has a special case for variadic parameter packs.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- Tuple expansion may not be reserved only for variadic tuple, maybe it can be used as well on fixed arity tuple as well? (For consistency)
- When using dynamic libraries, client libraries may relies that the host contains code up to a specific tuple arity. So we need to have a 
  way to enforce the compiler to generate all the implementation up to a specific tuple arity. (12 will keep backward comptibility with current `std` impl)

- Maybe a better syntax can be found that includes the length constraint for variadic tuples.
Example, instead of
```rust
fn my_function<(..#R), (..#L)>(r: (R#..), l: (L#..)) -> ((R, L)#..) { ... }
```
Use
```rust
fn my_function<(..#(R, L))>(r: (R#..), l: (L#..)) -> ((R, L)#..) { ... }
```
To enforce that variadic tuples `R` and `L` have the same arity.

# Future possibilities

[future-possibilities]: #future-possibilities

- Be able to create identifiers in an expansion form from the variadic tuple.
  For instance, if `(..#T)` is `(A, B, C)`, then `let ((ref v%T%)#..) = value;` expands to `let (ref vA, ref vB, ref vC) = value;`
  - This feature will let user to have more flexibility when implementing code with variadic tuple
- Improve the error message for `E0275` by providing the sequence of evaluated elements to give more help to the user about what can create the overflow.
  - In the context of variadic tuple, this can be the sequence of variadic tuple implementation that are tried by the compiler.
  - But, in the more generic case where two traits implementations requires each others, providing the dependency cycle can be really helpful.

- Supporting recrusive variadic tuple (ie, declaration like: `(..#((..#T)))`)
