- Feature Name: `arc_into_unique`
- Start Date: 2025-06-19
- RFC PR: [rust-lang/rfcs#3835](https://github.com/rust-lang/rfcs/pull/3835)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Ads a `into_unique` method to `Rc` and `Arc`, returns the `Some(this)`, if the has exactly one strong reference.

# Motivation
[motivation]: #motivation

I created a UniqArc wrapper that allows unique Arc to be DerefMut like a Box,
But I can't stably convert some shared Arc into a UniqArc, similar `Arc::try_unwrap(this).ok()`

For the sized type, `Arc::new(Arc::into_inner(this)?)` can be used, but this requires reallocating an Arc,
And if it's an unsized type, I have no way at all

```rust
// If there is such a method:
// pub fn into_unique(this: Arc<T>) -> Option<Arc<T>> { ... }
// I can create:
impl<T: ?Sized> UniqArc<T> {
    pub fn consume_new(arc: Arc<T>) -> Option<Self> {
        let unique = Arc::into_unique(arc)?;
        Some(UniqArc(unique))
    }
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Example similar to `Arc::into_inner`

```rust
use std::sync::Arc;

let x = Arc::new(3);
let y = Arc::clone(&x);

// Two threads calling `Arc::into_unique` on both clones of an `Arc`:
let x_thread = std::thread::spawn(|| Arc::into_unique(x));
let y_thread = std::thread::spawn(|| Arc::into_unique(y));

let unique_x = x_thread.join().unwrap();
let unique_y = y_thread.join().unwrap();

// One of the threads is guaranteed to receive the inner value:
assert!(matches!(
    (unique_x.as_deref(), unique_y.as_deref()),
    (None, Some(&3)) | (Some(&3), None)
));
// The result could also be `(None, None)` if the threads called
// `Arc::get_mut(&mut x).is_some().then_some(x)` and `Arc::get_mut(&mut x).is_some().then_some(x)` instead.
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

- Return `None` when there are other strong references
- Return `Some(this)` when there are no other strong references and no other weak references
- Dissociate weak references and return `Some(this)` when there are no other strong references but weak references

Partial implementation:

```rust
impl<T: ?Sized> Arc<T> {
    #[inline]
    pub fn into_unique(this: Self) -> Option<Self> {
        if this.inner().strong.fetch_sub(1, Release) != 1 {
            return;
        }

        // If there are outstanding weak references, it will be dissociated like make_mut
        todo!();

        this.inner().strong.fetch_add(1, Relaxed);

        Some(this)
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

Perhaps this method is not commonly used

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- What other designs have been considered and what is the rationale for not choosing them?
  - If `Option<Weak<T>>` is returned, additional upgrade costs will be required
  - If `Option<*const T>` is returned, it can easily lead to unnecessary unsafe code

- What is the impact of not doing this?
  - Unable to mutable on the last Arc instance stably without copying

- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?
  - The public methods of Rc/Arc seems insufficient to accomplish this

# Prior art
[prior-art]: #prior-art

I don't know

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
  - I'm not sure how to stably implement the dissociated weak

- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
  - Create a unique [`UniqArc`] from some shared `Arc`

[`UniqArc`]: https://crates.io/crates/unique-rc

# Future possibilities
[future-possibilities]: #future-possibilities

cannot think of anything.
