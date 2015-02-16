- Feature Name: Zip tuples of iterators
- Start Date: 2015-01-30
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Implement IntoIterator (since [RFC 235][1] has [landed][2]) for the common tuple types and remove the zip function from Iterator.

# Motivation

The zip function is convenient for iterating over two iterators at the same time. But when iterating over more iterators simultaneously causes unreadable code. A user might expect the following for loop to work:
```rust
    for (x, y, z) in (1..10, 2..11, 3..13) {
        println!("{}", (x, y, z));
    }
```
but instead is required to write
```rust
    for ((x, y), z) in (1..10).zip(2..11).zip(3..12) {
        println!("{}, {}, {}", x, y, z);
    }
```

# Detailed design

1. Remove IteratorExt::zip

2. replace std::iter::Zip by a struct and some macro-tuple-magic

[Bluss' implementation in the itertools crate][4] or mostly the same, but not quite finished implementation. WIP-implementation to be found at [my repository][3]. Will shout loudly at you because I haven't figured out IntoIterator for references yet.

```rust
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct TupleIterStruct<T> {
    inner : T
}

macro_rules! head {
    ($head:ident, $($tail:ident,)*) => {
        $head
    }
}

macro_rules! impl_ii_tuple {
    ( $($name:ident,)+) => (
        impl<$($name,)*> Iterator for TupleIterStruct<($($name,)*)>
            where $($name: Iterator,)*{
            type Item = ($(<$name as Iterator>::Item,)*);

            #[allow(non_snake_case)]
            #[inline]
            fn next(&mut self) -> Option<<Self as Iterator>::Item> {
                let ($(ref mut $name,)*) = self.inner;
                // lots of confusing brackets
                // Some -> tuple -> macro argument expansion -> if/else block
                Some(($(
                    if let Some(x) = $name.next() {
                        x
                    } else {
                        // WARNING: partial consume possible
                        // Zip worked the same.
                        return None;
                    }
                ,)*))
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                let ($(ref mut $name,)*) = self.inner;
                $(let $name = $name.size_hint();)*

                let lower = head!($($name,)*).0;
                $(let lower = cmp::min($name.0, lower);)*

                let upper = head!($($name,)*).1;
                $(
                    let upper = match ($name.1, upper) {
                        (Some(x), Some(y)) => Some(cmp::min(x,y)),
                        (Some(x), None) => Some(x),
                        (None, Some(y)) => Some(y),
                        (None, None) => None
                    };
                )*

                (lower, upper)
            }
        }

        impl<$($name,)*> IntoIterator for ($($name,)*)
            where $($name : IntoIterator,)* {
            type Iter = TupleIterStruct<($(<$name as IntoIterator>::Iter,)*)>;
            #[allow(non_snake_case)]
            fn into_iter(self) -> <Self as IntoIterator>::Iter {
                let ($($name,)*) = self;
                TupleIterStruct {
                    inner : ($($name.into_iter(),)*)
                }
            }
        }

        impl<$($name,)*> ExactSizeIterator for TupleIterStruct<($($name,)*)>
            where $($name : ExactSizeIterator,)* {}

        impl<$($name,)*> DoubleEndedIterator for TupleIterStruct<($($name,)*)> where
            $($name: DoubleEndedIterator + ExactSizeIterator,)*
        {
            #[inline]
            fn next_back(&mut self) -> Option<<Self as Iterator>::Item> {
                let ($(ref mut $name,)*) = self.inner;
                let len = head!($($name,)*).len();
                $(let len = cmp::min($name.len(), len);)*
                $(
                    for _ in 0..$name.len() - len {$name.next_back(); }
                )*
                // lots of confusing brackets
                // Some -> tuple -> macro argument expansion -> if/else block
                Some(($(
                    if let Some(x) = $name.next_back() {
                        x
                    } else {
                        // WARNING: partial consume not possible here
                        // but code does not reflect that
                        return None;
                    }
                ,)*))
            }
        }
        impl<$($name,)*> RandomAccessIterator for TupleIterStruct<($($name,)*)> where
            $($name: RandomAccessIterator,)*
        {
            #[inline]
            fn indexable(&self) -> usize {
                let ($(ref $name,)*) = self.inner;
                $(let $name = $name.indexable();)*

                let lower = head!($($name,)*);
                $(let lower = cmp::min($name, lower);)*
                lower
            }

            #[inline]
            fn idx(&mut self, index: usize) -> Option<<Self as RandomAccessIterator>::Item> {
                let ($(ref mut $name,)*) = self.inner;
                // lots of confusing brackets
                // Some -> tuple -> macro argument expansion -> if/else block
                Some(($(
                    if let Some(x) = $name.idx(index) {
                        x
                    } else {
                        // WARNING: partial consume possible here
                        return None;
                    }
                ,)*))
            }
        }
    );
}

macro_rules! peel_ii_tuple {
    () => ();
    ($name:ident, $($other:ident,)*) => (
        impl_ii_tuple! { $name, $($other,)* }
        peel_ii_tuple! { $($other,)* }
    )
}

peel_ii_tuple! { T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, }
```

# Drawbacks

Shamelessly quoting @japaric:

variadics generics would make it possible to do generic
programming over tuples and would
let us implement this in a cleaner way (for any arity)
which would modify/deprecate the TupleIterStruct and
that would be backwards-incompatible

# Alternatives
## Keep zip
don't change anything :(

## Extend zip
Extend zip to allow more than two items.

## impl Iterator for tuples
simple to implement (tested, works)
Still requires .iter() and similar calls for the tuple elements.

```rust
for (x, y, z) in (a.iter(), b.iter(), 1..20) {
    // something
}
```

# Unresolved questions
I have not thought about mixed move, ref and mut ref tuples


  [1]: https://github.com/rust-lang/rfcs/blob/master/text/0235-collections-conventions.md#intoiterator-and-iterable
  [2]: https://github.com/rust-lang/rust/pull/20790
  [3]: https://github.com/oli-obk/rust/tree/tuple_into_iter
  [4]: https://github.com/bluss/rust-itertools/blob/master/src/ziptuple.rs
