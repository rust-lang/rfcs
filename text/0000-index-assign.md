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

Today, the expression `a[b] = c` is always "evaluated" as an assignment, where the LHS may be
evaluated:

- using "built-in" indexing (which is only applicable to the types `[T]` and `[T; N]`), or
- using the `IndexMut` trait, i.e. as `*a.index_mut(b)`

The type check section of the compiler will choose which evaluation form to use based on the types
of `a`, `b` and `c`, and the traits that `a` implements, or raise an error if neither form is
applicable.

To additionally support evaluating `a[b] = c` as `a.index_assign(b, c)`, the type checking logic
will be *extended* as follows:

> Just like today, try to evaluate `a[b] = c` as an assignment, if the expression can't be
> evaluated as an assignment, then instead of raising an error, try to evaluate the expression
> as an indexed assignment using the `IndexAssign` trait.

Here's an example of how type checking will work:

``` rust
struct Array([i32; 32]);

impl IndexMut<Range<usize>> for Array {
    fn index_mut(&mut self, r: Range<usize>) -> &mut [i32] {
        &mut self.0[r]
    }
}

impl IndexAssign<Range<usize>, i32> for Array {
    fn index_assign(&mut self, r: Range<usize>, rhs: i32) {
        for lhs in &mut self[r] {
            *lhs = rhs;
        }
    }
}

// type check as assignment
//     `IndexMut<Range<usize>>` is not applicable because RHS is `i32`, expected `[i32]`
// type check as indexed assignment
//     `IndexAssign<Range<usize>, i32>` is applicable
// -> Expression will be evaluated as `array.index_assign(4..10, 0)`
array[4..10] = 0;
```

From the extended type check logic, it follows that in the case that both `IndexMut` and
`IndexAssign` are applicable, the `IndexMut` implementation will be favored [1].

``` rust
impl IndexMut<usize> for Array {
    fn index_mut(&mut self, i: usize) -> &mut i32 {
        &mut self.0[i]
    }
}

impl IndexAssign<usize, i32> for Array {
    fn index_assign(&mut self, _: usize, _: i32) {
        unreachable!()
    }
}

// type check as assignemnt
//     `IndexMut<usize, Output=i32>` is applicable
// -> Expression will be evaluated as `*array.index_mut(0) = 1`
array[0] = 1;
```

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
like `map[&key] = value` and `map[key] = value` which don't compile today.

# Drawbacks

None that I can think of

# Alternatives

## Bridge `IndexAssign` and `IndexMut`

Because `IndexMut` has "higher priority" than `IndexAssign`, it's possible to (unintentionally?)
change the semantics of the `a[b] = c` expression when a `IndexMut` implementation is added [2].
For example:

``` rust
struct Map(..);

impl IndexAssign<i32, i32> for Map {
    fn index_assign(&mut self, key: i32, value: i32) {
        println!("via IndexAssign");
        ..
    }
}

// Expression will be evaluated as `map.index_assign(0, 1)`
map[0] = 1;  // prints "via IndexAssign"

// But if this implementation is added
impl IndexMut<i32> for Map {
    fn index_mut(&mut self, k: i32) -> &mut i32 {
        panic!("no indexing for you")
    }
}

// Now the expression will be evaluated as `*map.index_mut(0) = 1`
map[0] = 1;  // nows panics
```

This hazard (?) can be avoided by "bridging" the `IndexMut` and `IndexAssign` traits with a blanket
implementation:

``` rust
impl<Idx, T> IndexAssign<Idx, T::Output> for T where
    T: IndexMut<Idx>,
{
    fn index_assign(&mut self, idx: Idx, rhs: T::Output) {
        *self.index_mut(idx) = rhs;
    }
}
```

Now it's impossible to implement `IndexMut<B, Output=C>` on a type `A`, if it already implements
`IndexAssign<B, C>` and vice versa.

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

---

### Author notes

[1] The compiler does something similar when type checking an expression where both built-in
indexing and the `Index[Mut]` trait are applicable - it favors built-in indexing.

[2] `a[b]` is another expression where one can change its semantics by implementing a trait:

``` rust
struct Array([i32; 32]);

impl Deref for Array {
    type Target = [i32; 32];

    fn deref(&self) -> &[i32; 32] {
        println!("via Deref")
        &self.0
    }
}

// Will be evaluated as `array.deref()[0]`
array[0];  // prints "via Deref"

impl Index<usize> for Array {
    type Output = i32;

    fn index(&self, _: usize) -> &i32 {
        panic!("no indexing for you")
    }
}

// Now will be evaluated as `*array.index(0)`
array[0];  // now panics
```

However, I don't think either case is a problem in practice. It seems unlikely that a library
author will purposefully override the semantics of an operator, and it seems less likely that they
would do it unintentionally, without triggering a unit test failure.
