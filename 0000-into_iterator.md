- Start Date: 2014-03-20
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add an `IntoIterator` trait that allows a function to consume both an iterator
and a value that can be converted into an iterator.

# Motivation

We have some unnecessary redundancy in some of our collection APIs:

 * `Vec<T>::push_all` is designed to take one `Vec<T>` and merge it into another.
 * `Extendable::extend` takes an Iterator and merges it into a value.
 * `vec::append` copies and merges one `Vec<T>` with anther `Vec<T>`.

# Detailed design

This redundancy can be eliminated with a simple trait:

```
trait IntoIterator<T, Iter: Iterator<T>> {
    fn into_iterator(self) -> Iter;
}
```

Here is how it would be used with `Vec<T>`:

```
use std::vec_ng::{Vec, MoveItems};

impl<T> IntoIterator<T, MoveItems<T>> for MoveItems<T> {
    fn into_iterator(self) -> MoveItems<T> { self }
}

impl<T> IntoIterator<T, MoveItems<T>> for Vec<T> {
    fn into_iterator(self) -> MoveItems<T> { self.move_iter() }
}
```

Almost every value and Iterator will have the same implementation so this could
be a good use case for a macro.

Here is a demonstration on how they would be used:

```
trait MyExtendable<T> {
    fn my_extend<Iter: Iterator<T>, IntoIter: IntoIterator<T, Iter>>(&mut self, x: IntoIter);
}

impl<T> MyExtendable<T> for Vec<T> {
    fn my_extend<Iter: Iterator<T>, IntoIter: IntoIterator<T, Iter>>(&mut self, x: IntoIter) {
        let mut iter = x.into_iterator();
        let (size, _) = iter.size_hint();
        self.reserve_additional(size);

        for x in iter {
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
