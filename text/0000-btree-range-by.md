- Feature Name: `btree_range_by`
- Start Date: 2018-09-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add the methods `range_by` and `range_by_mut` to `BTreeMap`, and
add the method `range_by` to `BTreeSet`.

# Motivation
[motivation]: #motivation

`BTreeMap` and `BTreeSet` have a handy `range` function, which provide
iterators over a range of elements. They are efficient, requiring only O(log n)
time, meaning that we can quickly iterate over a small range even when the map
or set is very large. The `range` functions are useful for things other than
iteration; for example, `btree_set.range(x..).next()` is the fastest way to
find the first element of `btree_set` that is larger than or equal to `x`.

One annoyance of `BTreeSet::range` is that in order to call it you need to be
able to produce values of some type `Q` where `T: Borrow<Q>` (here `T` is the
element type of the `BTreeSet`). There are situations where you want to search
in a set, but it is difficult or impossible to produce such values. Here are
two examples:

- your set is of type `BTreeSet<(i32, LargeType)>`, you have access to `a` (of
  type `i32`) and `&b` (of type `&LargeType`). You want to find the first
  element of your set that is larger than `(a, b)`. If `LargeType: Clone`, this
  can be done with `set.range(&(a, b.clone()))` but might be expensive.
- your set is of type `BTreeSet<(i32, SomeWeirdType)>` and you want to iterate
  over all pairs whose first coordinate is at least `5`. If you could construct
  a value `b` that is smaller than every other value of type `SomeWeirdType`,
  you could do this with `set.range(&(5, b))`. But constructing such a `b`
  might be annoying, particularly if `SomeWeirdType` is a type parameter.

This RFC proposes adding a method

```rust
fn range_by<'a, F>(&self, f: F) -> Range<T>
where
    F: FnMut(&'a T) -> std::cmp::Ordering
```

to `BTreeSet<T>`, along with similar methods for `BTreeMap`. By passing a
callback function instead of a value of the appropriate type, we avoid the need
to construct problematic objects.

Similar methods already exist on the slice type. For example, `[T]` has
`binary_search` (taking an element) and `binary_search_by` (taking a callback).
RFC 2351 proposes adding both `is_sorted` (taking no parameters) and
`is_sorted_by` (taking a comparison callback) to `[T]`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Possible documentation of the new method on `BTreeSet`:

> ```rust
> fn range_by<'a, F>(&self, f: F) -> Range<T>
> where
>     F: FnMut(&'a T) -> std::cmp::Ordering
> ```
> 
> Constructs a double-ended iterator over a sub-range of elements in the set,
> namely those elements for which `f` returns `Ordering::Equal`. The function
> `f` is required to be a non-decreasing function; for example, if `f(x)` returns
> `Ordering::Equal` and `y >= x` then `f(y)` must return `Ordering::Equal` or
> `Ordering::Greater`.
>
> ## Panics
> May panic if `f` fails to be non-decreasing. The exact circumstances under
> which a panic occurs are implementation-dependent.
>
> ## Example
> ```rust
> use std::collections::BTreeSet;
> use std::cmp::Ordering::*;
> 
> let mut set = BTreeSet::new();
> set.insert((5, "hello"));
> set.insert((7, "goodbye"));
> set.insert((9, "aloha"));
> 
> // See if there's an element whose first coordinate is 5.
> println!("{}", set.range_by(|x| x.0.cmp(5)));
>
> // Iterate over all elements whose first coordinate is between 4 and 7 inclusive.
> let f = |x| match (x.0.cmp(4), x.0.cmp(7)) {
>     (Less, _) => Less,
>     (_, Greater) => Greater,
>     _ => Equal,
> };
> for pair in set.range_by(f) {
>     println!("{}", pair);
> }
> ```

Possible documentation of the new methods on `BTreeMap`:

> ```rust
> fn range_by<'a, F>(&self, f: F) -> Range<K, V>
> where
>     F: FnMut(&'a K) -> std::cmp::Ordering
> ```
>
> Constructs a double-ended iterator over a sub-range of elements in the map,
> namely those elements for which `f` returns `Ordering::Equal` when applied to
> the key. The function `f` is required to be a non-decreasing function; for
> example, if `f(x)` returns `Ordering::Equal` and `y >= x` then `f(y)` must
> return `Ordering::Equal` or `Ordering::Greater`.
>
> ## Panics
> May panic if `f` fails to be non-decreasing. The exact circumstances under
> which a panic occurs are implementation-dependent.
>
> ## Examples
> ```rust
> use std::collections::BTreeMap;
> use std::cmp::Ordering::*;
>
> let mut map = BTreeMap::new();
> map.insert(5, "hello");
> map.insert(7, "goodbye");
> map.insert(9, "aloha");
>
> // See if there's an element whose key is at least 5.
> // The call to `range_by` here is just a more complicated way of writing
> // `map.range(5..)`.
> println!("{}", map.range_by(|k| if k >= 5 { Equal } else { Less }).next());
>
> // Iterate over all items for which the square of the key is between 20 and
> // 30 (inclusive).
> let f = |k| match (k*k < 20, k*k > 30) {
>     (true, _) => Less,
>     (_, true) => Greater,
>     _ => Equal,
> };
> for (k, v) in map.range_by(f) {
>     println!("{}", v);
> }
> ```
>
> ```rust
> fn range_by_mut<'a, F>(&mut self, f: F) -> RangeMut<K, V>
> where
>     F: FnMut(&'a K) -> std::cmp::Ordering
> ```
> 
> Constructs a mutable double-ended iterator over a sub-range of elements in
> the map, namely those elements for which `f` returns `Ordering::Equal` when
> applied to the key. The function `f` is required to be a non-decreasing
> function; for example, if `f(x)` returns `Ordering::Equal` and `y >= x` then
> `f(y)` must return `Ordering::Equal` or `Ordering::Greater`.
>
> ## Panics
> May panic if `f` fails to be non-decreasing. The exact circumstances under
> which a panic occurs are implementation-dependent.
>
> ## Examples
> ```rust
> use std::collections::BTreeMap;
> use std::cmp::Ordering::*;
>
> let mut map = BTreeMap::new();
> map.insert((5, "h"), "hello");
> map.insert((7, "g"), "goodbye");
> map.insert((9, "a"), "aloha");
>
> let f = |k| match (k < 5, k > 8) {
>     (true, _) => Less,
>     (_, true) => Greater,
>     _ => Equal,
> };
> // This will print all the values for with the first element of the key is
> // between 5 and 8 inclusive.
> for (k, v) in map.range_by(f) {
>     println!("{}", v);
> }
> ```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes to add the following three methods:
```rust
impl<T> BTreeSet<T> {
    fn range_by<'a, F>(&self, f: F) -> Range<T>
    where
        F: FnMut(&'a T) -> std::cmp::Ordering
}

impl<K, V> BTreeMap<K, V> {
    fn range_by<'a, F>(&self, f: F) -> Range<K, V>
    where
        F: FnMut(&'a K) -> std::cmp::Ordering

    fn range_by_mut<'a, F>(&mut self, f: F) -> RangeMut<K, V>
    where
        F: FnMut(&'a K) -> std::cmp::Ordering
}
```

Each of these methods requires the function `f` to be non-decreasing. The
semantics are that the returned iterator iterates over all elements for which
`f` returns `Ordering::Equal`. The performance is the same as that of the
`range` method. That is, O(log n) to construct the iterator, and then amortized
O(1) for each iterate.

If the desired range is empty (that is, if `f` does not return
`Ordering::Equal` for andy element of the set or map), the returned iterator
will be an empty iterator. If the function `f` fails to be non-decreasing then
the methods are allowed to panic, and they are also allowed to return valid
iterators over some arbitrary sub-range of the map or set.

The implementation of these methods is expected to be a simple modification of
the implementation of `range`, in which comparisons against the range bounds
are replaced by calls to the comparison function. If it doesn't result in a
performance penalty, `range` would probably be reimplemented in terms of `range_by`.

# Drawbacks
[drawbacks]: #drawbacks

It increases the size of the standard library.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There are other possibilities for the interface of these functions. For
example, instead of passing a single function it would be possible to pass two
functions returning bools: the first would be a non-decreasing function and the
second would be a non-increasing function, and the range being iterated over
would be those values for which both functions returned true. That is, the
signature of `range_by` for `BTreeSet` would become

```rust
fn range_by<'a, F, G>(&self, lower_bound: F, upper_bound: G) -> Range<T>
where
    F: FnMut(&'a T) -> bool,
    G: FnMut(&'a T) -> bool,
```

This formulation (and its requirements on the functions) is a bit more
complicated, but it would be more convenient to use in certain circumstances.
For example, `set.range(a..b)` could be expressed as `set.range_by(|x| x >= a,
|y| y < b)`.  On the other hand, expressing `set.range(a..b)` in terms of the
main proposal requires some clunky if/then or match expressions.

Another possibility would be to add a function `range_by_key`, either instead of
or in addition to `range_by`. The signature would be something like

```rust
fn range_by_key<'a, F, Q, R>(&self, range: R, f: F) -> Range<T>
where
    R: RangeBounds<Q>,
    F: FnMut(&'a T) -> Q,
```

and the semantics would be that the returned iterator iterates over all
elements `x` for which `f(x)` belongs to `range`. (`f` would need to be a
non-decreasing function.) I think this function would fit all the use cases of
the motivation, and the `binary_search_by_key` function on slices provides
some precedent. The large number of generic parameters is a slight turn-off, though.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None so far.
