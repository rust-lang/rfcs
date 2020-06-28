- Feature Name: index_get_set
- Start Date: 2020-06-25
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add an IndexSet trait that allows for overloading index assignment `a[b] = c`. Add a corresponding
IndexGet trait that allows for returning an item by value from an indexing operation.

# Motivation
[motivation]: #motivation

Some collections are unable to return direct references to their elements, such as a BitSet, Cache,
or collections managed via FFI. It is desirable however, to support the `[]` operator when using such
collections.

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
One limitation of these existing traits is that there is no way for a container to work solely on objects by-value. This RFC adds two new traits, `IndexGet` and `IndexSet`, defined as follows.

```rust
pub trait IndexGet<Idx: ?Sized> {
    type Output: ?Sized;
    fn index_get(&self, index: Idx) -> Self::Output;
}
pub trait IndexSet<Idx: ?Sized, Val: ?Sized> {
    fn index_set(&mut self, index: Idx, value: Val);
}
```
These traits can be used in isolation to overload the `[]` operator for collections offering value semantics.

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

impl IndexSet<u8, bool> for BitSet {
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
IndexGet and IndexSet behave fairly obviously, with some caveats. In note, for the BitSet above, the following code compiles.

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
                        // let a = val[3]; a = &mut a;
val[3] ^= true; // compound assignment (of any kind) is not currently supported
                // there are hidden costs that are out of scope for this RFC
```


When implemented alongside `Index(Mut)`, `IndexSet` and `IndexGet` changes the behavior of existing indexing operators.

An indexed assignment `a[b] = c`, where `a` implements `IndexMut`, is syntax sugar for a dereference of the returned pointer `*a.index_mut(b) = c`.
If `a` implements `IndexSet` on the other hand, `a[b] = c` will become syntax sugar for `a.index_set(b, c)`, even if such an expression will fail to
compile. In other words, implementing `IndexSet<A, B>` (for any A and B), will prevent assignments from desugaring to `IndexMut`, even if said desugaring
is legal without `IndexSet` and even if it would allow the program to compile.

An expression of the form `&mut a[b]` is always syntax sugar for a method call of `a.index_mut(b)`, and this will always fail to compile if an
implementation of `IndexMut` is not applicable.

A let binding of the form `a[b]` will prefer an implementation of `IndexGet`, but will fail back to `Index`.

A borrow of the form `&a[b]` will prefer an implementation of `Index`, but will fall back to `IndexGet`.

A method call or function invocation will behave similarly, with objects taken by-value preferring `IndexGet`, and object
taken by-reference preferring `Index`.

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
Rust then, requires that the `Output` associated type in both implementations be equal.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
This RFC would require the compiler to implement an overload resolution algorithm in order to decide which trait implementation should be called.

1. Assignments use IndexSet iff any implementation exists, else use IndexMut. If not implemented, error.
2. Mutable place context uses IndexMut. If not implemented, error.
3. If attempting to move, use IndexGet. If not implemented, continue.
4. Use Index if implemented. If not implemented, continue.
5. Use IndexGet. If not implemented, error.

'Any implementation exists' in this context does not equal 'implemented'. For a container of type `T`, 
an index operation is considered to have any implementation iff `T` implements `Index(Mut/Get/Set)<...>`, where the generic parameters can be any type.

Note: if the following is the only implementation of `IndexSet`, it prohibits `a[k] = v` syntax, even if `IndexMut` is implemented (and rustc should not be too clever
in realizing this call is impossible). This also means that any implementation of `IndexSet` is **backwards-incompatible**
unless appropriate forwarding implementations are present.

```rust
impl<T> IndexSet<usize, !> for Vec<T> {
    fn index_set(&mut self, index: usize, value: !) {}
}
```

The overload resolution interacts with `Deref` as follows - the compiler will follow the entire overload resolution tree for each struct/enum/union, only
dereferencing if it would otherwise error due to a lack of any implementation.

The complex nature of rules 3, 4, 5 are to allow collections that implement either `Index` or `IndexGet` to work as expected, while
still allowing `IndexGet` to supersede `Index` in the case when the result is taken by value. Examples are as follows.

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
let a: impl IndexMut<usize> + Index<usize> + IndexSet<usize, T>; //not real Rust
a[b] = c; // *a.index_mut(b) = c
let _ = &mut a[b]; // a.index_mut(b)
let _ = &a[b]; // a.index(b)
let _ = a[b]; // a.index_get(b)
```

```rust
let a: impl IndexMut<usize> + Index<usize> + IndexGet<usize>; //not real Rust
a[b] = c; // *a.index_mut(b) = c
let _ = &mut a[b]; // a.index_mut(b)
let _ = &a[b]; // a.index(b)
let _ = a[b]; // a.index_get(b)
```

```rust
let a: impl Index<usize> + IndexGet<usize> + IndexSet<usize, T>; //not real Rust
a[b] = c; // a.index_set(b, c)
let _ = &mut a[b]; // compile error
let _ = &a[b]; // a.index(b)
let _ = a[b]; // a.index_get(b)
```

```rust
let a: IndexGet<usize> + IndexSet<usize, T>; //not real Rust
a[b] = c; // a.index_set(b, c)
let _ = &mut a[b]; // compile error
let _ = &a[b]; // &a.index_get(b)
let _ = a[b]; // a.index_get(b)
```
Any other combinations of traits should follow naturally.

With regard to the compiler enforcing compatible implementations of `Index` and `IndexGet`, the same mechanism as coherence could apply.
if two implementations for `Index` and `IndexGet` overlap according to coherence rules (as if `Index` and `IndexGet` were the same trait, 
their `Output` type must be identical. 

# Drawbacks
[drawbacks]: #drawbacks

This would require complex [if-else-if logic in the typechecker](https://github.com/rust-lang/rfcs/pull/1129#issuecomment-162036985). It would add a lot of mental complexity
around the edge cases of indexing. The trait searching logic is also possibly unique with no similar behavior anywhere.

Under the current rules of no fall-through, it is impossible to add `IndexSet` to `T` without breaking newtypes that implement `Deref<T>` and `IndexMut`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Fall back to IndexMut

From #1129:

As shown in the case B of the type checking section, when checking `a[b] = c` the compiler will
error if none of the `IndexSet` implementations is applicable, even if a `IndexMut`
implementation could have been used. This alternative proposes falling back to the `IndexMut`
trait in such a scenario. Under this alternative the case B example would compile and `foo[baz] =
quux` would be evaluated as `*foo.index_mut(baz) = quux`.

The most visible consequence of this change is that we'd have sugar for updating a key value pair
in a map:

``` rust
map[key] = value;       // insert a `(key, value)` pair
map[&key] = new_value;  // update the value associated to `key`
```

However, some people deem this as a footgun because its easy to confuse the insertion operation
and the update one resulting in programs that always panic:

``` rust
let (key, value): (&Key, Value);
// ..
let mut map: HashMap<Key, Value> = HashMap::new();
map[key] = value;  // Compiles, but always panics!
```

The programmer meant to write `map[key.clone()] = value` to use the insertion operation, but they
won't notice the problem until the program crashes at runtime.

## Bridge `IndexSet` and `IndexMut`

Also from #1129:

As shown in the case C of the type checking section adding an `IndexSet` implementation to a
struct that already implements the `IndexMut` trait can cause breakage of downstream crates if one
is not careful. This hazard can be eliminated by "bridging" the `IndexMut` and `IndexSet` traits
with a blanket implementation:

``` rust
impl<Idx, T> IndexSet<Idx, T::Output> for T where
    T: IndexMut<Idx>,
{
    fn index_assign(&mut self, idx: Idx, rhs: T::Output) {
        *self.index_mut(idx) = rhs;
    }
}
```

Now it's impossible to forget to implement `IndexSet<B, C>` on a type `A` that already
implements `IndexMut<B, Output=C>` because it's automatically handled by the blanket
implementation.

However, this blanket implementation creates coherence problems for the planned changes to
`BTreeMap` and `HashMap`:

``` rust
// NOTE Omitting all the bounds for simplicity

// "Mutable" lookup
impl<'a, K, V> IndexMut<&'a K> for HashMap<K, V> {
    // Output = V;
    fn index_mut(&mut self, k: &K) -> &mut V { .. }
}

// By extension: HashMap<K, V> also implements IndexSet<&K, V>

// Insertion
impl<K, V> IndexSet<K, V> for HashMap<K, V> {
//~^ this conflicts with the other `IndexSet` implementation, because the `K` in this
// `IndexSet` includes all the references of the form `&'a _`
    fn index_assign(&mut self, k: K, v: V) { .. }
}
```

So it's not a viable alternative.

## Mututal exclusion of IndexGet and Index

Under the current model `Index` and `IndexGet` are not exclusive. One proposed alternative is
to make them exclusive, which would slightly simplify the overload resolution rules. This is similar to a
proposed alternative in #159, with the exception that `IndexSet` is its own trait.

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

## Marker Traits
An auto trait of the form `trait FallbackToIndexMut {}` may be possible to allow users to opt out of the no-fallback behavior, 
but it is unlikely to be of much use.