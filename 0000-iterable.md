- Start Date: 2014-03-20
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add an `Iterable` family of traits that will allow a function to consume both
an iterator or a value that can be converted into an iterator.

# Motivation

We have some unnecessary redundancy in some of our collection APIs:

 * `Vec<T>::push_all` is designed to take one `Vec<T>` and merge it into another.
 * `Extendable::extend` takes an Iterator and merges it into a value.
 * `vec::append` copies and merges one `Vec<T>` with anther `Vec<T>`.

# Detailed design

This redundancy can be eliminated with a simple trait:

```
/// Any type that implements `Iterable<T, I>` can be iterated over with the
/// iterator `I` to yield values of type `T`. Because of Rusts move semantics,
/// this means that `self` needs to be taken by value, which for moving types
/// means turning `self` into a iterator.
trait Iterable<T, I: Iterator<T>> {
    fn into_iter(self) -> I;

    fn map<'r, B>(self, f: 'r |A| -> B) -> iter::Map<'r, A, B, I> {
        self.into_iter().map(f)
    }
 
    fn collect<B: iter::FromIterator<A>>(self) -> B {
        iter::FromIterator::from_iterator(&mut self.into_iter())
    }
 
    // ... all old Iterator adapters that take `self` and make sense
    // to provide on a data structure directly
}
```

This trait is implementable by both values and iterators. Values would opt-in
to implementing `Iterable`, where we would move all `.move_iter()`-style
methods to impls of `Iterable`:

```
use std::vec_ng::{Vec, Items, MutItems, MoveItems};

impl<T> Iterable<T, MoveItems<T>> for Vec<T> {
    fn into_iter(self) -> MoveItems<T> {
        self.move_iter()
    }
}
```

Iterators would now be required to implement `Iterable`:

```
trait Iterator<A>: Iterable<A, Self> {
    ...
}

impl<T> Iteratable<T, MoveItems<T>> for MoveItems<T> {
    fn into_iter(self) -> MoveItems<T> { self }
}
```

Since every Iterator will have the same implementation so this could be a good
use case for a macro.

Additionally, we would add two similar traits to handle returning references:

```
trait RefIterable<'a, A, I: iter::Iterator<A>> {
    fn refs(&'a self) -> I;
}
 
trait MutIterable<'a, A, I: iter::Iterator<A>> {
    fn mut_refs(&'a mut self) -> I;
}
```

We also could support more optimal iterators for collections of `Pod`s

```
/// Automatically implemented for all `RefIterable<&A>` with `A: Pod`
trait PodIterable<'a, A: Pod, I: iter::Iterator<A>> {
    fn values(&'a self) -> Values<I>;
}

// Automatically implement for all `RefIterable<&A>` with `A: Pod`
impl<'a, T: Pod, RefIter: iter::Iterator<&'a T>, Iter: RefIterable<'a, &'a T, RefIter>>
        PodIterable<'a, &'a T, RefIter> for Iter {
    fn values(&'a self) -> Values<RefIter> { Values { iter: self.refs() } }
}

/////////////////////////////////////////////////////////////////////////////

struct Values<I> { iter: I }
impl<'a, T: Pod, I: iter::Iterator<&'a T>> iter::Iterator<T> for Values<I> {
    fn next(&mut self) -> Option<T> { self.iter.next().map(|x| *x) }
    fn size_hint(&self) -> (uint, Option<uint>) { self.iter.size_hint() }
}
impl<'a, T: Pod, I: iter::DoubleEndedIterator<&'a T>> iter::DoubleEndedIterator<T> for Values<I> {
    fn next_back(&mut self) -> Option<T> { self.iter.next_back().map(|x| *x) }
}
impl<'a, T: Pod, I: iter::ExactSize<&'a T>> iter::ExactSize<T> for Values<I> {}
```

Finally, here is a demonstration of using this trait to reimplement `Extendable`:

```
trait Extendable<T> {
    fn extend<I: Iterator<T>, Iter: Iterable<T, I>>(&mut self, x: Iter);
}

impl<T> Extendable<T> for Vec<T> {
    fn extend<I: Iterator<T>, Iter: Iterable<T, I>>(&mut self, iter: Iter) {
        let mut iter = iter.into_iter();
        let (size, _) = iter.size_hint();
        self.reserve_additional(size);

        for x in &mut iter {
            self.push(x);
        }
    }
}

fn main() {
    let mut a = Vec::new();
    a.my_extend(vec!(4, 5, 6));
    a.my_extend(vec!(7, 8, 9).move_iter());
    println!("extend: {}", a);
}
```

# Alternatives

 * We just implement everything to accept iterators. It does lead to a simpler,
   but less powerful, interface.
 * Felix Klock (pnkfelix) suggested an alternative name of `Pushable` instead of
   `Extendable`. This trait would also provide a `.push()` method for adding a
   single element.

# Unresolved questions

 * This RFC should be revisited if/when we gain associated items. It could
   reduce some of the function declaration syntactic overhead.
 * This RFC should be revisited if we modify the coherence rules to allow impls like
	 `impl<I: Iterator> Iterable for I { ... }` while still allowing for other
   impls of `Iterable`.
