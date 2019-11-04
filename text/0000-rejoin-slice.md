- Feature Name: `rejoin_slice`
- Start Date: 2019-11-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add method `fn rejoin(&self, other: &[T]) -> &[T]` for `slice`, with other related methods
(`*_mut`, `try_*`, and `rejoin`, `try_rejoin` for `&str`).
This API allows joining two slices that are adjacent in the memory, into one.

# Motivation
[motivation]: #motivation

The standard library has multiple APIs for splitting slices into subslices,
including range indexing and the `split_at` methods. However, once split,
it doesn't provide any methods of joining slices back together.

Sometimes the need for this arises when using parsers or utility APIs that
split a parsed slice or string into subslices. As the logic might not be
customizable, it's hard for the user to retrieve longer subslices from
specific shorter ones after getting them from the library.

As a remedy, it's often possible to use indices and/or pointers, and retrieve
a longer subslice from the original slice. However this might be fiddly and bug-prone.
The API proposed by this RFC provides an obvious and easy-to-use way to
rejoin slices. 

The said indexing workaround becomes even harder when mutable slices are used -
as long as the derived subslices exist, the original slice stays borrowed
and it's hard for the user to retrieve a longer slice in the case they
would need it.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Sometimes we get, as a result of parsing, a bunch of substrings that we know to be
split from the original string:

```
let mut values: Vec<&str> = util_lib::split_by_streak("aaaaaaabbbbbbbcccccccddddeeeeeeefffggggggggh");
assert_eq!(&"gggggggg", values[values.len()-1]);
assert_eq!(&"h", values[values.len()-2]);
```

In the case we want to join adjacent substrings back together, we can use the `rejoin` API:

```
let last_two = &values[values.len()-2].rejoin(&values[values.len()-1]);
assert_eq!(&"ggggggggh", last_two);
```

Slices have a similar API too. Joining subslices is especially useful when a parser returns mutable slices:

```
let mut values: Vec<&mut [u32]> = util_lib::split_by_streak_mut(&mut [5, 5, 5, 5, 2, 2, 7, 7, 7, 7, 7, 3, 3, 3][..]);
assert_eq!(&mut [7, 7, 7, 7, 7][..], values[values.len()-2]);
assert_eq!(&mut [3, 3, 3][..], values[values.len()-1]);
```

Modifying the other subslices as-is, but joining the last two into a single subslice
would be cumbersome to do without the `rejoin` operation - we would have to get rid of the
mutable slices to be able to index the original slice, which defeats the purpose
of the operation we just did with `split_by_streak_mut`!

```
let last_two = &values[values.len()-2].rejoin_mut(&values[values.len()-1]);
assert_eq!(&mut [7, 7, 7, 7, 7, 3, 3, 3][..], last_two);
```

Fortunately `rejoin_mut` allows us to easily combine the last two subslices back together.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes adding the following APIs to the standard library:

```
impl<T> [T] {
    fn rejoin<'r>(&'r self, other: &'r [T]) -> &'r [T] {
        self.try_rejoin(other).expect("the input slices must be adjacent in memory")
    }

    fn rejoin_mut<'r>(&'r mut self, other: &'r mut [T]) -> &'r mut [T] {
        self.try_rejoin_mut(other).expect("the input slices must be adjacent in memory")
    }

    fn try_rejoin<'r>(&'r self, other: &'r [T]) -> Option<&'r [T]> {
        let self_len = self.len();
        let self_end = self[self_len..].as_ptr();
        if core::ptr::eq(self_end, other.as_ptr()) {
            Some(unsafe { core::slice::from_raw_parts(self.as_ptr(), self.len() + other.len()) })
        } else {
            None
        }
    }

    fn try_rejoin_mut<'r>(&'r mut self, other: &'r mut [T]) -> Option<&'r mut [T]> {
        let self_len = self.len();
        let self_end = self[self_len..].as_mut_ptr();
        if core::ptr::eq(self_end, other.as_mut_ptr()) {
            Some(unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len() + other.len()) })
        } else {
            None
        }
    }
}

impl str {
    fn rejoin<'r>(&'r self, other: &'r str) -> &'r str {
        self.try_rejoin(other).expect("the input string slices must be adjacent in memory")
    }

    fn try_rejoin<'r>(&'r self, other: &'r str) -> Option<&'r str> {
        self.as_bytes().try_rejoin(other.as_bytes()).map(|s| unsafe { core::str::from_utf8_unchecked(s) })
    }
}
```

The APIs are implemented for testing purposes in crate https://crates.io/crates/rejoin_slice.

## Notes about safety

These APIs internally use `unsafe` to achieve their functionality.
However, they provides a safe interface.
The following precautions are taken for safety:
1. Pointer arithmetic is never explicitly performed. A pointer pointing to
the end of the first slice is calculated using safe APIs.
2. Equality comparisons between pointers, although undefined behaviour in C in
cases where the pointers originate from different objects, can be considered
to be safe in Rust. This is ensured by the fact that the standard library
provides a safe function `core::ptr::eq` to compare pointers.
3. `unsafe` is only used to call `core::slice::from_raw_parts` to create a new
slice after the check that the input slices are adjacent in memory.

# Drawbacks
[drawbacks]: #drawbacks

There is a single outstanding question related to the soundness of the APIs:
although intended for rejoining subslices that originate from the same slice,
they necessarily also allow joining together unreleated slices that are adjacent
in memory *by chance*. From the perspective of the type system this shouldn't
be a problem, as the APIs require the both the input slices and the output slice
to be of the same type, and the resulting slice is always outlived by the subslices
it is joined from; it shouldn't enable any funny interactions.

However from the viewpoint of the memory model and LLVM optimizations this
*might* be a problem. It should be noted that according to the above reasoning
in *Notes about safety*, calculating the pointers shouldn't be UB as we are able
to do that in purely safe Rust. However, the author would like to have a
confirmation from the Unsafe Code Guidelines working group whether indexing and
doing pointer arithmetic with the resulting slice is sound or not, and
if there is complications, is there anything we can do to resolve them.

The author believes that joining slices is a useful primitive, and deserves
a place in the standard library if the soundness can be guaranteed.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design seems fairly obvious choice in the design space.
The main alternative to this proposal is not to implement it,
and let users to calculate joined subslices from indexes or pointers.

# Prior art
[prior-art]: #prior-art

There exists a crate that implements the API described by this RFC: https://crates.io/crates/rejoin_slice

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is the API sound from memory model perspective?
- Do string slices need a mutable version of the API?
- Anything else?

# Future possibilities
[future-possibilities]: #future-possibilities

The author hasn't got anything special in mind.
