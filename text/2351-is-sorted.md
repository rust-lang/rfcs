- Feature Name: `is_sorted`
- Start Date: 2018-02-24
- RFC PR: [rust-lang/rfcs#2351](https://github.com/rust-lang/rfcs/pull/2351)
- Rust Issue: [rust-lang/rust#53485](https://github.com/rust-lang/rust/issues/53485)

# Summary
[summary]: #summary

Add the methods `is_sorted`, `is_sorted_by` and `is_sorted_by_key` to `[T]`;
add the methods `is_sorted` and `is_sorted_by` to `Iterator`.

# Motivation
[motivation]: #motivation

In quite a few situations, one needs to check whether a sequence of elements
is sorted. The most important use cases are probably **unit tests** and
**pre-/post-condition checks**.

The lack of an `is_sorted()` function in Rust's standard library has led to
[countless programmers implementing their own](https://github.com/search?l=Rust&q=%22fn+is_sorted%22&type=Code&utf8=%E2%9C%93).
While it is possible to write a one-liner using iterators (e.g.
`(0..arr.len() - 1).all(|i| arr[i] <= arr[i + 1])`¹), it is still unnecessary
mental overhead while writing *and* reading the code.

In [the corresponding issue on the main repository](https://github.com/rust-lang/rust/issues/44370)
(from which a few comments are referenced) everyone seems to agree on the
basic premise: we want such a function.

Having `is_sorted()` and friends in the standard library would:
- prevent people from spending time on writing their own,
- improve readbility of the code by clearly showing the author's intent,
- and encourage to write more unit tests and/or pre-/post-condition checks.

Another proof of this functions' usefulness is the inclusion in the
standard library of many other languages:
C++'s [`std::is_sorted`](http://en.cppreference.com/w/cpp/algorithm/is_sorted),
Go's [`sort.IsSorted`](https://golang.org/pkg/sort/#IsSorted),
D's [`std.algorithm.sorting.is_sorted`](https://dlang.org/library/std/algorithm/sorting/is_sorted.html)
and others. (Curiously, many (mostly) more high-level programming language –
like Ruby, Javascript, Java, Haskell and Python – seem to lack such a function.)

¹ In the initial version of this RFC, this code snippet contained a bug
(`<` instead of `<=`). This subtle mistake happens very often: in this RFC,
[in the discussion thread about this RFC](https://github.com/rust-lang/rfcs/pull/2351#issuecomment-370126518),
in [this StackOverflow answer](https://stackoverflow.com/posts/51272639/revisions)
and in many more places. Thus, avoiding this common bug is another good
reason to add `is_sorted()`.

## Fast Implementation via SIMD

Lastly, it is possible to implement `is_sorted` for many common types with SIMD
instructions which improves speed significantly. It is unlikely that many
programmers will take the time to write SIMD code themselves, thus everyone
would benefit if this rather difficult implementation work is done in the
standard library.



# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Possible documentation of the two new methods of `Iterator` as well as
`[T]::is_sorted_by_key`:

> ```rust
> fn is_sorted(self) -> bool
> where
>     Self::Item: PartialOrd,
> ```
> Checks if the elements of this iterator are sorted.
>
> That is, for each element `a` and its following element `b`, `a <= b`
> must hold. If the iterator yields exactly zero or one element, `true`
> is returned.
>
> Note that if `Self::Item` is only `PartialOrd`, but not `Ord`, the above
> definition implies that this function returns `false` if any two
> consecutive items are not comparable.
>
> ## Example
>
> ```rust
> assert!([1, 2, 2, 9].iter().is_sorted());
> assert!(![1, 3, 2, 4).iter().is_sorted());
> assert!([0].iter().is_sorted());
> assert!(std::iter::empty::<i32>().is_sorted());
> assert!(![0.0, 1.0, std::f32::NAN].iter().is_sorted());
> ```
> ---
>
> ```rust
> fn is_sorted_by<F>(self, compare: F) -> bool
> where
>     F: FnMut(&Self::Item, &Self::Item) -> Option<Ordering>,
> ```
> Checks if the elements of this iterator are sorted using the given
> comparator function.
>
> Instead of using `PartialOrd::partial_cmp`, this function uses the given
> `compare` function to determine the ordering of two elements. Apart from
> that, it's equivalent to `is_sorted`; see its documentation for more
> information.
>
> ---
>
> (*for `[T]`*)
>
> ```rust
> fn is_sorted_by_key<F, K>(&self, f: F) -> bool
> where
>     F: FnMut(&T) -> K,
>     K: PartialOrd,
> ```
> Checks if the elements of this slice are sorted using the given
> key extraction function.
>
> Instead of comparing the slice's elements directly, this function
> compares the keys of the elements, as determined by `f`. Apart from
> that, it's equivalent to `is_sorted`; see its documentation for more
> information.
>
> ## Example
>
> ```rust
> assert!(["c", "bb", "aaa"].is_sorted_by_key(|s| s.len()));
> assert!(![-2i32, -1, 0, 3].is_sorted_by_key(|n| n.abs()));
> ```

The methods `[T]::is_sorted` and `[T]::is_sorted_by` will have analogous
documentations to the ones shown above.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes to add the following methods to `[T]` (slices) and
`Iterator`:

```rust
impl<T> [T] {
    fn is_sorted(&self) -> bool
    where
        T: PartialOrd,
    { ... }

    fn is_sorted_by<F>(&self, compare: F) -> bool
    where
        F: FnMut(&T, &T) -> Option<Ordering>,
    { ... }

    fn is_sorted_by_key<F, K>(&self, f: F) -> bool
    where
        F: FnMut(&T) -> K,
        K: PartialOrd,
    { ... }
}

trait Iterator {
    fn is_sorted(self) -> bool
    where
        Self::Item: PartialOrd,
    { ... }

    fn is_sorted_by<F>(mut self, compare: F) -> bool
    where
        F: FnMut(&Self::Item, &Self::Item) -> Option<Ordering>,
    { ... }
}
```

In addition to the changes shown above, the three methods added to `[T]` should
also be added to `core::slice::SliceExt` as they don't require heap
allocations.

To repeat the exact semantics from the prior section: the methods return
`true` if and only if for each element `a` and its following element `b`, the
condition `a <= b` holds. For slices/iterators with zero or one element,
`true` is returned. For elements which implement `PartialOrd`, but not `Ord`,
the function returns `false` if any two consecutive elements are not
comparable (this is an implication of the `a <= b` condition from above).

A sample implementation can be found
[here](https://play.rust-lang.org/?gist=431ff42fe8ba5980fcf9250c8bc4492b&version=stable).


# Drawbacks
[drawbacks]: #drawbacks

It increases the size of the standard library by a tiny bit.

# Rationale and alternatives
[alternatives]: #alternatives

### Only add the methods to `Iterator`, but not to `[T]`
Without `is_sorted()` defined for slices directly, one can still fairly easily
test if a slice is sorted by obtaining an iterator via `iter()`. So instead of
`v.is_sorted()`, one would need to write `v.iter().is_sorted()`.

This always works for `is_sorted()` because of the `PartialOrd` blanket impl
which implements `PartialOrd` for all references to an `PartialOrd` type. For
`is_sorted_by` it would introduce an additional reference to the closures'
arguments (i.e. `v.iter().is_sorted_by(|a, b| ...))` where `a` and `b` are
`&&T`).

While these two inconveniences are not deal-breakers, being able to call those
three methods on slices (and all `Deref<Target=[T]>` types) directly, could be
favourable for many programmers (especially given the popularity of slice-like
data structures, like `Vec<T>`). Additionally, the `sort` method and friends
are defined for slices, thus one might expect the `is_sorted()` method there,
too.


### Add the three methods to additional data structures (like `LinkedList`) as well
Adding these methods to every data structure in the standard library is a lot of
duplicate code. Optimally, we would have a trait that represents sequential
data structures and would only add `is_sorted` and friends to said trait. We
don't have such a trait as of now; so `Iterator` is the next best thing. Slices
deserve special treatment due to the reasons mentioned above (popularity and
`sort()`).


### `Iterator::while_sorted`, `is_sorted_until`, `sorted_prefix`, `num_sorted`, ...
[In the issue on the main repository](https://github.com/rust-lang/rust/issues/44370),
concerns about completely consuming the iterator were raised. Some alternatives,
such as [`while_sorted`](https://github.com/rust-lang/rust/issues/44370#issuecomment-327873139),
were suggested. However, consuming the iterator is neither uncommon nor a
problem. Methods like `count()`, `max()` and many more consume the iterator,
too. [One comment](https://github.com/rust-lang/rust/issues/44370#issuecomment-344516366) mentions:

> I am a bit skeptical of the equivalent on Iterator just because the return
> value does not seem actionable -- you aren't going to "sort" the iterator
> after you find out it is not already sorted. What are some use cases for this
> in real code that does not involve iterating over a slice?

As mentioned above, `Iterator` is the next best thing to a trait representing
sequential data structures. So to check if a `LinkedList`, `VecDeque` or
another sequential data structure is sorted, one would simply call
`collection.iter().is_sorted()`. It's likely that this is the main usage for
`Iterator`'s `is_sorted` methods. Additionally, code like
`if v.is_sorted() { v.sort(); }` is not very useful:  `sort()` already runs in
O(n) for already sorted arrays.

Suggestions like `is_sorted_until` are not really useful either: one can easily
get a subslice or a part of an iterator (via `.take()`) and call `is_sorted()`
on that part.


# Unresolved questions
[unresolved]: #unresolved-questions


### Should `Iterator::is_sorted_by_key` be added as well?

This RFC proposes to add `is_sorted_by_key` only to `[T]` but not to
`Iterator`. The latter addition wouldn't be too useful since once could easily
achieve the same effect as `.is_sorted_by_key(...)` by calling
`.map(...).is_sorted()`. It might still be favourable to include said function
for consistency and ease of use. The standard library already hosts a number of
sorting-related functions all of which come in three flavours: *raw*, `_by` and
`_by_key`. By now, programmers could expect there to be an `is_sorted_by_key`
as well.


### Add `std::cmp::is_sorted` instead

As suggested [here](https://github.com/rust-lang/rust/issues/44370#issuecomment-345495831),
one could also add this free function (plus the `_by` and `_by_key` versions)
to `std::cmp`:

```rust
fn is_sorted<C>(collection: C) -> bool
where
    C: IntoIterator,
    C::Item: Ord,
```

This can be seen as a better design as it avoids the question about which data
structure should get `is_sorted` methods. However, it might have the
disadvantage of being less discoverable and also less convenient (long path or
import).


### Require `Ord` instead of only `PartialOrd`

As proposed in this RFC, `is_sorted` only requires its elements to be
`PartialOrd`. If two non-comparable elements are encountered, `false` is
returned. This is probably the only useful way to define the function for
partially orderable elements.

While it's convenient to call `is_sorted()` on slices containing only
partially orderable elements (like floats), we might want to use the stronger
`Ord` bound:

- Firstly, for most programmers it's probably not *immediately* clear how the
  function is defined for partially ordered elements (the documentation should
  be sufficient as explanation, though).
- Secondly, being able to call `is_sorted` on something will probably make
  most programmers think, that calling `sort` on the same thing is possible,
  too. Having different bounds for `is_sorted` and `sort` thus might lead to
  confusion.
- Lastly, the `is_sorted_by` function currently uses a closure which returns
  `Option<Ordering>`. This differs from the closure for `sort_by` and looks a
  bit more complicated than necessary for most cases.
