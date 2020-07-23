- Feature Name: index_get_set
- Start Date: 2020-06-25
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC modifies IndexMut and IndexSet to allow overriding indexed assignment. This RFC also adds a
corresponding IndexGet trait that allows for returning an item by value from an indexing operation.

# Motivation
[motivation]: #motivation

Some collections are unable to return direct references to their elements, such as a BitSet, Cache,
or collections managed via FFI. In addition, structs such as `HashMap` would like to generate mutable references
while also allowing custom insertion semantics such as `map[key] = value`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Currently, Rust indexing traits are defined as follows.
```rust
pub trait Index<Idx: ?Sized> {
    type Output: ?Sized;
    fn index(&self, index: Idx) -> &Self::Output;
}
pub trait IndexMut<Idx: ?Sized>: Index<Idx> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output;
}
```
One limitation of these existing traits is that there is no way to override assignment operators.  For backwards compatibility reasons, this RFC proposes to adjust `IndexMut` as follows.
```rust
pub trait IndexMut<Idx: ?Sized>: Index<Idx> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output;
    fn index_set(&mut self, index: Idx, value: Self::Output)
        where <Self as Index<Idx>>::Output: Sized,
    {
        *self.index_mut(index) = value;
    }
}
```

This RFC also adds two new traits, `IndexGet` and `IndexSet`, defined as follows.
```rust
pub trait IndexGet<Idx: ?Sized> {
    type Output: ?Sized;
    fn index_get(&self, index: Idx) -> Self::Output;
}
pub  trait IndexSet<Idx: ?Sized> {
    type Input: ?Sized;
    fn index_set(&mut self, index: Idx, value: Self::Input);
}
impl<T: IndexMut<Idx>, Idx> IndexSet<Idx> for T where <Self as Index<Idx>>::Output: Sized {
    type Input = <Self as Index<Idx>>::Output;
    fn index_set(&mut self, index: Idx, value: Self::Input) {
        <Self as IndexMut<Idx>>::index_set(self, index, value)
    }
}
```
These traits can be used to overload the `[]` operator for collections offering value semantics.

```rust
#[derive(Default)]
pub struct BitSet {
    bits: u64
}

impl IndexGet<u8> for BitSet {
    type Output = bool;
    fn index_get(&self, index: u8) -> bool {
        if index >= 64 {
            panic!("index must be < 64")
        }
        (self.bits >> index) & 1u64 == 1
    }
}

impl IndexSet<u8> for BitSet {
    type Input = bool;

    fn index_set(&mut self, index: u8, value: bool) {
        if index >= 64 {
            panic!("index must be < 64")
        }
        
        if value {
            self.bits |= 1u64 << index;
        } else {
            self.bits &= !(1u64 << index);
        }
    }
}
```
IndexGet and IndexSet behave fairly obviously, with some caveats. For the BitSet above, the following code compiles.

```rust
let val = BitSet::default();
let a = val[3];
let b = &val[3];
val[3].foo(); // where foo() takes `self` or `&self`
val[3] = !val[3];
val[5] = val[3] || val[2];
```

But the following does not.
```rust
let a = &mut val[3];    // desugars into a call to IndexMut, explained later
                        // one can always do this:
                        // let mut a = val[3]; a = &mut av
val[3] ^= true; // compound assignment (of any kind) is not currently supported
                // and is out of scope for this RFC
```

The followings are some examples of what methods Rust would call for different indexing operations.

```rust
let a: impl IndexMut<usize> + Index<usize>; //not real Rust
a[b] = c; // *a.index_mut(b) = c
let _ = &mut a[b]; // a.index_mut(b)
let _ = &a[b]; // a.index(b)
let _ = a[b]; // a.index(b) + Copy semantics
```

```rust
let a: impl IndexMut<usize> + Index<usize> + IndexGet<usize> + IndexSet<usize, T>; //not real Rust
a[b] = c; // a.index_set(b, c)
let _ = &mut a[b]; // a.index_mut(b)
let _ = &a[b]; // a.index(b)
let _ = a[b]; // a.index_get(b)
```

```rust
let a: impl IndexGet<usize> + IndexSet<usize, T>; //not real Rust
a[b] = c; // a.index_set(b, c)
let _ = &mut a[b]; // compile error
let _ = &a[b]; // &a.index_get(b)
let _ = a[b]; // a.index_get(b)
```
Any other combinations of traits should follow naturally.

```rust
pub trait Index<Idx: ?Sized> {
    type Output: ?Sized;
    fn index(&self, index: Idx) -> &Self::Output;
}
pub trait IndexGet<Idx: ?Sized> {
    type Output: ?Sized;
    fn index_get(&self, index: Idx) -> Self::Output;
}
```

It would be impossible to apply these overload rules if implementations for `Index` and `IndexGet` were incompatible. 
Rust would require that the `Output` associated type in both impl blocks be equal.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
This RFC would require the compiler to implement an overload resolution algorithm in order to decide which trait implementation should be called.

1. An assignment uses IndexSet. If not implemented, error.
2. A mutable place context uses IndexMut. If not implemented, error.
3. If attempting to move, use IndexGet. If not implemented, continue.
4. Use Index if implemented. If not implemented, continue.
5. Use IndexGet. If not implemented, error.

The overload resolution interacts with `Deref` as follows - the compiler will follow the entire overload resolution tree for each struct/enum/union, only
dereferencing if it would otherwise error due to a lack of any implementation.

The complex nature of rules 3, 4, 5 are to allow collections that implement either `Index` or `IndexGet` to work as expected, while
still allowing `IndexGet` to supersede `Index` in the case when the result is taken by value.

## Desugarings

An indexed assignment `a[b] = c`, will desugar to an `IndexSet` call of `a.index_set(b, c)`.

An expression of the form `&mut a[b]` will desugar to an `IndexMut` call of `a.index_mut(b)`.

An index of the form `a[b]` will prefer an implementation of `IndexGet`, but will fail back to `Index`.

A borrow of the form `&a[b]` will prefer an implementation of `Index`, but will fall back to `IndexGet`.

A method call or function invocation will behave similarly, with objects taken by-value preferring `IndexGet`, and object
taken by-reference preferring `Index`.

# Drawbacks
[drawbacks]: #drawbacks

This would require complex [if-else-if logic in the typechecker](https://github.com/rust-lang/rfcs/pull/1129#issuecomment-162036985). It would add some mental complexity
around the edge cases of indexing.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives               

## Mututal exclusion of IndexGet and Index

`Index` and `IndexGet` could be made exclusive instead of requiring their `Output` associated type to be equal.
There is no compelling reason to keep the traits compatible, but making them exclusive does result in a less powerful system.

## Specialization

Instead of adding `index_set` as a method to `IndexMut`, specialization on `IndexSet` could be used to 
have a default implementation for `IndexMut`.


# Prior art
[prior-art]: #prior-art

This proposal was heavily inspired by the `get` and `set` of more dynamic langauges, such as
Kotlin's [index operator](https://kotlinlang.org/docs/reference/operator-overloading.html#indexed) or C# [indexers](https://docs.microsoft.com/en-us/dotnet/csharp/programming-guide/indexers/).

Prior RFCs include #159 and #1129.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None so far.

# Future possibilities
[future-possibilities]: #future-possibilities

## Emplacement
These traits would likely be compatible with #2884, with #2884 introducing no new syntax that might lead to the proliferation
of ways to insert (one of the reasons #1129 was postponed).
