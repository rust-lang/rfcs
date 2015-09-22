- Feature Name: indexed_assignment
- Start Date: 2015-05-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `IndexAssign` trait that allows overloading "indexed assignment" expressions like `a[b] = c`.

# Motivation

Let users define syntactic sugar for operations like these:

- Inserting a *new* key-value pair into a map

``` rust
let mut map = HashMap::new();
map[key] = value;  // equivalent to `map.insert(key, value);`
```

- Setting each element of a "slice" to some value

(slice, as in a fraction of a collection, which may not necessarily be stored in contiguous memory)

``` rust
let mut matrix = { .. };

// Set each element of the second row of `matrix` to `1`
matrix[1] = 1;

// or

let mut vector = { .. };

// set first 4 elements of `vector` to zero
vector[..4] = 0;
```

- Copying a slice into another

``` rust
// Copy the third row of `another_matrix` into the second row of `matrix`
matrix[1] = &another_matrix[2]

// or

// Copy the first four elements of `another_vector` in the middle of `vector`
vector[2..6] = &another_vector[..4]
```

Also, before 1.0, `BTreeMap` and `HashMap` lost their `IndexMut` implementations to
[future-proof indexing on maps]. With this feature, it would be possible to re-implement `IndexMut`
on these maps and additionally implement `IndexAssign` on them, such that all these operations will
work:

[future-proof indexing on maps]: https://github.com/rust-lang/rust/pull/23559

``` rust
// insert new entry (`IndexAssign`)
map[key] = value;

// apply mutating method to the value associated with `key` (`IndexMut`)
map[&key].foo_mut();

// get a mutable reference to the value associated with `key` (`IndexMut`)
&mut map[&key];
```

# Detailed design

## The trait

The `IndexAssign` trait will be added to the core crate, and re-exported in the std crate. Its
signature is shown below.

``` rust
#[lang = "index_assign"]
trait IndexAssign<Index, Rhs> {
    /// `self[index] = rhs`
    fn index_assign(&mut self, index: Index, rhs: Rhs);
}
```

## Type checking `a[b] = c`

The type checker logic will be extended as follows:

> Whenever the expression `a[b] = c` is encountered, the compiler will check if `A` (`a` has type
> `A`) has *any* implementation of the `IndexAssign` trait; if that's the case then it will proceed
> to look for an applicable implementation and evaluate the expression as `a.index_assign(b, c)`,
> or, if no implementation is applicable, it will raise an error. On the other hand, if `A` doesn't
> have any `IndexAssign` implementation then the compiler will use today's logic: evaluate the
> expression as an assignment where the LHS is evaluated as an lvalue using either built-in
> indexing or the `IndexMut` trait.

Three cases are worth analyzing:

``` rust
// A
impl IndexAssign<Bar, Baz> for Foo { .. }
impl Index<Bar> for Foo { type Output = Baz; .. }
impl IndexMut<Bar> for Foo { .. }

let (foo, bar, baz): (Foo, Bar, Baz);
// ..
foo[bar] = baz;
```

Here `Foo` has an applicable `IndexAssign` implementation, so `foo[bar] = baz` is evaluated as
`foo.index_assign(bar, baz)`. Note that the `IndexMut` implementation is ignored even though
`*foo.index_mut(bar) = baz` is a valid evaluation form of `foo[bar] = baz`. Finally, one can use
the `*&mut foo[bar] = baz` expression to use `IndexMut` instead of `IndexAssign`.

``` rust
// B
impl IndexAssign<Bar, Quux> for Foo { .. }
impl Index<Baz> for Foo { type Output = Quux; .. }
impl IndexMut<Baz> for Foo { .. }

let (foo, baz, quux): (Foo, Baz, Quux);
// ..
foo[baz] = quux;
//~^ error: expected `Bar`, found `Baz`
```

In this case, `Foo` has an `IndexAssign` implementation but it's not applicable to
`foo[baz] = quux` so a compiler error is raised. Although the expression could have been evaluated
as `*foo.index_mut(baz) = quux`, the compiler won't attempt to "fall back" to the `IndexMut` trait.
See the alternatives section for a version of this RFC where the compiler does fall back to
`IndexMut`.

``` rust
// C
impl Index<Bar> for Foo { type Output = Baz; .. }
impl IndexMut<Bar> for Foo { .. }

let (foo, bar, baz): (Foo, Bar, Baz);
// ..
foo[bar] = baz;
```

The third case points out a breaking-change hazard to library authors. If the author adds e.g.
`impl IndexAssign<Baz, Quux> for Foo` to their library, the change will break all the downstream
crates that use the `IndexMut<Bar>` implementation in the form of `foo[bar] = baz`. To prevent
breakage, the author must add a `IndexAssign<Bar, Baz>` implementation (that preserves the
semantics of `foo[bar] = baz`) to `Foo` before adding any other `IndexAssign` implementation.

## Feature gating

The feature itself will land behind a `indexed_assignment` feature gate, and the `IndexAssign`
trait will be marked as unstable under a `index_assign` feature gate.

The expression `a[b] = c` will be gated only if it must be evaluated using the `IndexAssign` trait.

An example below:

``` rust
// aux.rs

// required to implement the unstable `IndexAssign` trait
#![feature(index_assign)]

pub struct Map(HashMap<i32, i32>);

impl IndexAssign<i32, i32> for Map { .. }
```

``` rust
// main.rs

extern crate aux;

use aux::Map;

fn main() {
    let map: Map = { .. };

    // would be evaluated as `map.index_assign(0, 1)`
    map[0] = 1;  //~ error: overloaded indexed assignment is unstable
    //~^ help: add `#![feature(indexed_assignment)]` to enable

    let mut v = vec![0, 1, 2];
    // This is OK, because `v[0]` goes through the `IndexMut` trait
    // will be evaluated as `*v.index_mut(0) = 1`
    v[0] = 1;
}
```

## Changes in the standard library

The `IndexMut` implementations of `HashMap` and `BTreeMap` will be restored, and additionally both
maps will implement the `IndexAssign<K, V>` trait such that `map[key] = value` will become sugar
for `map.insert(key, value)`.

## Backward compatibility

Adding this feature is a backward compatible change because expressions like `a[b] = c` that work
today, will continue to work with unaltered semantics.

The proposed library changes are also backward compatible, because they will enable expressions
like `map[&key].foo_mut()` and `map[key] = value` which don't compile today.

# Drawbacks

There is sugar for map insertion (`map[xey] = value`), but not for updating the value associated to
an existing key (some people actually consider that this is actually a good thing). The closest
thing to sugar for the update operation is `*&mut map[&key] = value`, which is totally not obvious.

This situation could be improved using an extension trait (which doesn't even need to be defined in
the `std` crate):

``` rust
/// `mem::replace` as a method
trait Swap {
    /// `mem::replace(self, new_value)`
    fn swap(&mut self, new_value: Self) -> Self;
}

impl<T> Swap for T {
    fn swap(&mut self, value: T) -> T {
        mem::replace(self, value)
    }
}

let mut map: HashMap<String, Thing>;
let (new_thing, another_new_thing) = (Thing, Thing);
// ..
// Update the value associated to `"foo"`, the old value is discarded
map["foo"].swap(new_thing);  // instead of the more obscure `*&mut map["foo"] = new_thing`

// As a bonus, you can actually retrieve the old value
let old_thing = map["bar"].swap(another_new_thing);
```

# Alternatives

## Fall back to `IndexMut`

As shown in the case B of the type checking section, when checking `a[b] = c` the compiler will
error if none of the `IndexAssign` implementations is applicable, even if a `IndexMut`
implementation could have been used. This alternative proposes falling back to the `IndexMut`
trait in such scenario. Under this alternative the case B example would compile and `foo[baz] =
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

For more details about this alternative check the previous git revision of this RFC, where this
alternative was the main proposal.

## Bridge `IndexAssign` and `IndexMut`

As shown in the case C of the type checking section adding an `IndexAssign` implementation to a
struct that already implements the `IndexMut` trait can cause breakage of downstream crates if one
is not careful. This hazard can be eliminated by "bridging" the `IndexMut` and `IndexAssign` traits
with a blanket implementation:

``` rust
impl<Idx, T> IndexAssign<Idx, T::Output> for T where
    T: IndexMut<Idx>,
{
    fn index_assign(&mut self, idx: Idx, rhs: T::Output) {
        *self.index_mut(idx) = rhs;
    }
}
```

Now it's impossible to forget to implement `IndexAssign<B, C>` on a type `A` that already
implements `IndexMut<B, Output=C>` because it's automatically handled by the blanket
implementation.

However this blanket implementation creates coherence problems for the planned changes to
`BTreeMap` and `HashMap`:

``` rust
// NOTE Omitting all the bounds for simplicity

// "Mutable" lookup
impl<'a, K, V> IndexMut<&'a K> for HashMap<K, V> {
    // Output = V;
    fn index_mut(&mut self, k: &K) -> &mut V { .. }
}

// By extension: HashMap<K, V> also implements IndexAssign<&K, V>

// Insertion
impl<K, V> IndexAssign<K, V> for HashMap<K, V> {
//~^ this conflicts with the other `IndexAssign` implementation, because the `K` in this
// `IndexAssign` includes all the references of the form `&'a _`
    fn index_assign(&mut self, k: K, v: V) { .. }
}
```

So it's not a viable alternative.

# Unresolved questions

None so far
