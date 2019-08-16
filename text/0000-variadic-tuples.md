- Feature Name: variadic_tuples
- Start Date: 2019-08-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Tuples types are ordered set of type, but users can only use tuple with a specified number of types.

This RFC aims to allow the use of a _variadic tuple_ to be able to write implementation for tuples with an arbitrary number of type.

# Motivation
[motivation]: #motivation

## Arbitrary tuple arity support

Currently, when a user wants to either use or add behavior to tuples, he writes an impl for each size of tuple.
For easier maintenance, it is usually done with a `macro_rules` and implements up to 12 arity tuple. (ex: `Hash` implementation in `std`).

The proposed RFC provides an easier way to define the implementation for those tuples and don't limit the arity of tuple supported.
Also, the compiler will compile only required tuple arity implementation.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The _variadic tuple_ occurs in two form: a declarative form and an expansion form.

The declarative form is `..#T` and an expansion form is `T#..`.

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

## Declaring a _variadic tuple_

To declare a _variadic tuple_, we use `(..#T)`, where `T` is a type identifier.

For instance:
* `struct VariadicStruct<(..#T1)> ..`
* `impl<(..#Head)> ..`
* `impl<A, B, C, (..#_Tail)> ..`
* `fn my_function<(..#A)> ..`
* `fn my_function<A, B, C, (..#D)> ..`

You can think this like a rule you give to the compiler to generated appropriate code when it runs into specific patterns:
* `VariadicStruct<(int, usize)>` matches `VariadicStruct<(..#T1)>` where `(..#T1)` maps to `(int, usize)`
* `VariadicStruct<(int, usize, usize)>` matches `VariadicStruct<(..#T1)>` where `(..#T1)` maps to `(int, usize, usize)`
(We will see implementation examples later, with the expansion form)

## Expanding _variadic tuple_

When expanding a tuple, we use the form `T#..`, but more generally: `<pattern(T)>#..` where `<pattern(T)>` is an expression or a block expression using the identifier `T`.

Let's implement the `Hash` trait:

```rust
// For the example, we consider the impl for (A, B, C). So `..#T matches `A, B, C`
// We have the first expansion here, `T#..` expands to `A, B, C`
impl<(..#T), Last> Hash for (T#.., Last) 
where
    {T: Hash}#..,                               // Expands to `A: Hash, B: Hash, C: Hash`
    Last: Hash + ?Sized, {

    #[allow(non_snake_case)]
    fn hash<S: Hasher>(&self, state: &mut S) {
        let ({ref T}#.., ref last) = *self;     // Expands to `let (ref A, ref B, ref C, ref last) = *self;`
        (T.hash(state)#.., last.hash(state));   // Expands to `(A.hash(state), B.hash(state), C.hash(state), last.hash(state));`
    }
}
```

## Allowed usages of _variadic tuple_

### Declarative form

* Struct generic parameters     : `struct MyStruct<(..#T)>`
* Function generic parameters   : `fn my_function<(..#T)>`
* Type alias declaration        : `type MyTuple<(..#T)>`
* impl block generic parameters : `impl<(..#T)>`

### Expansion form

* Struct member declaration:
  ```rust
  struct MyStruct<(..#T)> {
    arrays: ([T; 32]#..),
  }
  ```
* Function arguments        : `fn my_function<(..#T)>(values: &(Vec<T>#..))`
* Function return type      : `fn my_function<(..#T)>(values: &(Vec<T>#..)) -> (&[T]#..)`
* Function body             : 
```rust
fn my_function<(..#T)>(values: &(Vec<T>#..)) -> (&[T]#..) {
    let ({ref T}#..) = values;
    (T#..)
}
```
* Type alias definition     : `type TupleOfVec<(..#T)> = (Vec<T>#..);`
* impl block type           : `impl<(..#T)> MyStruct<T#..>`
* where clause              :
```rust
impl<(..#T)> MyStruct<(T#..)>
where {T: Hash}#..
```




Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Recursion

To implement some feature, we may want to use recursion over the arity of the tuple.
For instance, let's implement a trait that gives the arity of a tuple:

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
* The `impl<Head, (..#Tail)> Arity for (Head, Tail#..)` is the recursive implementation.
* The `impl Arity for ()` is the termination of the recursive implementation.

And when we compile the following code:
```rust
fn main() {
    println!("Arity of (bool, usize): {}", <(bool, usize) as Arity>::VALUE);
}
```
The compiler will execute these steps:
1. Search `impl` of `Arity` for `(bool, usize)`
1. `impl` not found, Search variadic `impl` of `Arity` for `(bool, usize)`
1. Variadic impl found: `impl<Head, (..#Tail)> Arity for (Head, Tail#..)`
1. Generate `impl` of `Arity` for `(bool, usize)`
    1. Requires `impl` of `Arity` for `(usize,)`
    1. Search `impl` of `Arity` for `(usize,)`
    1. `impl` not found, Search variadic `impl` of `Arity` for `(usize,)`
    1. Variadic impl found: `impl<Head, (..#Tail)> Arity for (Head, Tail#..)`
    1. Generate `impl` of `Arity` for `(usize,)`
        1. Requires `impl` of `Arity` for `()`
        1. Search `impl` of `Arity` for `()`
        1. `impl` found
    1. Generation of `impl` of `Arity` for `(usize,)` completed
1. Generation of `impl` of `Arity` for `(bool, usize)` completed

## Using multiple _variadic tuple_

```rust
trait Append<T> {
    type Out;

    fn append(self, value: T) -> Self::Out;
}

impl<(..#L), (..#R)> Append<(R#..)> for (L#..) {
    type Out = (L#.., R#..);

    fn append(self, value: (R#..)) -> Self::Out; {
        let (L#..) = self;
        let (R#..) = value;
        (L#.., R#..)
    }
}
```




This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Prior art
[prior-art]: #prior-art

C++11 sets a decent precedent with its variadic templates, which can be used to define type-safe variadic functions, among other things. C++11 has a special case for variadic parameter packs.

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Tuple expansion may not be reserved only for _variadic tuple_, maybe it can be used as well on fixed arity tuple as well? (For consistency)
* When using dynamic libraries, client libraries may relies that the host contains code up to a specific tuple arity. So we need to have a 
  way to enforce the compiler to generate all the implementation up to a specific tuple arity. (12 will keep backward comptibility with current `std` impl)


- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

* Be able to create identifiers in an expansion form from the _variadic tuple_.
  For instance, if `(..#T)` is `(A, B, C)`, then `let ({ref v%T%}#..) = value;` expands to `let (ref vA, ref vB, ref vC) = value;`




Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how the this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
