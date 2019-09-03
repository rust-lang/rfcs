- Feature Name: collection-transmute
- Start Date: 2019-09-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a `transmute(self)` method to `Vec`, `VecDeque` and `BinaryHeap`.

# Motivation
[motivation]: #motivation

The mentioned types cannot safely be transmuted by `mem::transmute`. Adding a 
method for that purpose will hopefully discourage users to try anyway.

E.g. the following code is UB:

```rust
let x = vec![0u32; 2];
let y = unsafe { std::mem::transmute::<_, Vec<[u8; 4]>>(x) };
```

This is explained in the docs for [`Vec`], but with an unsound solution. The 
way to do this correctly turns out surprisingly tricky:

```rust
let x = vec![0u32; 2];
let y = unsafe {
    let y: &mut Vec<_> = &mut *ManuallyDrop::new(x);
    Vec::from_raw_parts(y.as_mut_ptr() as *mut [u8; 4],
       		        y.len(),
		        y.capacity())
};
```

Though the code is not too large, there are a good number of things to get 
wrong – this solution was iteratively created by soundness-knowledgeable 
Rustaceans, with multiple wrong attempts. So this method seems like a good 
candidate for inclusion into `std`.

This also applies to `VecDeque` and `BinaryHeap`, which are implemented in 
terms of `Vec`.

[`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The types `std::vec::Vec`, `std::collections::VecDeque` and 
`std::collections::BinaryHeap` all get a new unsafe `transmute` method that 
takes `self` by value and returns a new `Vec`/`VecDeque`/`BinaryHeap` with a 
caller-chosen item type.

The API might look like this (exemplified for `Vec`):

```rust
impl<T> Vec<T> {
    /// Transmute this `Vec` to a different item type
    ///
    /// # Safety
    ///
    /// Calling this function requires the target item type be compatible with
    /// `Self::Item` (see [`mem::transmute`]).
    ///
    /// # Examples
    ///
    /// transmute a `Vec` of 32-bit integers to their byte representations:
    ///
    /// ```
    /// let x = vec![0u32; 5];
    /// let y = unsafe { x.transmute::<[u8; 4]>() };
    /// assert_eq!(5, y.len());
    /// assert_eq!([0, 0, 0, 0], y[0]);
    /// ```
    ///
    /// [`mem::transmute`]: ../../std/mem/fn.transmute.html
    unsafe fn transmute<I>(self) -> Vec<I> {
        ..
    }
```

This would mean our example above would become:

```rust
let x = vec![0i32; 2];
let y = x.transmute::<[u8; 4]>();
```

The documentation of `mem::transmute` should link to the new methods.

A clippy lint can catch offending calls to `mem::transmute` and suggest using 
the inherent `transmute` method where applicable.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation for `Vec` copies the above solution. The methods for 
`VecDeque` and `BinaryHeap` use the `Vec` method on their data.

# Drawbacks
[drawbacks]: #drawbacks

Adding a new method to `std` increases code size and needs to be maintained. 
However, this seems to be a minor inconvenience when compared to the safety 
benefit.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

As explained above, the method is useful, yet non-obvious. There were multiple 
wrong implementation attempts before that were not chosen for being unsound.

We could do nothing, but this means users wanting to transmute the items of a 
`Vec` will be left without an obvious solution which will lead to unsound code 
if they get it wrong.

We could document the correct solution instead of putting it into `std`. This 
would lead to worse ergonomics.

We could create a trait to hold the `transmute` method. This would allow more 
generic usage, but might lead to worse ergonomics due to type inference 
uncertainty.

It would even be possible to provide a default implementation using 
`mem::transmute`, but having a default implementation that might be unsound for 
some types is a footgun waiting to happen.

# Prior art
[prior-art]: #prior-art

`mem::transmute` offers the same functionality for many other types. We have 
added similar methods to different types where useful, see the various 
iterator-like methods in `std`. @Shnatsel and @danielhenrymantilla came up 
with the solution together in a 
[clippy issue](https://github.com/rust-lang/rust-clippy/issues/4484).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- are there other types that would benefit from such a method? What about 
`HashSet`, `HashMap` and `LinkedList`?
- would it make sense to add implementations to types where `mem::transmute` is 
acceptable, to steer people away from the latter?
- this RFC does not deal with collection types in crates such as 
[`SmallVec`](https://docs.rs/smallvec), though it is likely an implementation 
in `std` might motivate the maintainers to include similar methods.

# Future possibilities
[future-possibilities]: #future-possibilities

The author cannot think of anything not already outlined above.
