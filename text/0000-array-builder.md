- Feature Name: array_builder
- Start Date: 2021-05-26
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

A data structure to allow building a `[T; N]` dynamically. Safely handling drops and efficiently being convertable into the underlying `[T; N]`.

# Motivation
[motivation]: #motivation

Array initialisation is surprisingly unsafe. The safest way is to initialise with a default value, then replacing the values.
This is not always possible and requires moving to using MaybeUninit and unsafe. This is very easy to get wrong.

For example:
```rust
let mut array: [MaybeUninit<String>; 4] = MaybeUninit::uninit_array();
array[0].write("Hello".to_string());
panic!("some error");
```

Despite being completely safe, this will cause a memory leak. This is because `MaybeUninit` does not call `drop` for the string.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Definition

This RFC proposes a new struct, `ArrayBuilder`. It has a very basic API designed solely for building new `[T; N]` types without initialising it all before hand.
This is not a heapless replacement for Vec.

```rust
pub struct ArrayBuilder<T, const N: usize> {
    // hidden fields
}

// ArrayBuilder implements drop safely, and prevents any memory leaks
impl<T, const N: usize> Drop for ArrayBuilder<T, N> {}

impl<T, const N: usize> ArrayBuilder<T, N> {
    /// Create a new uninitialized ArrayBuilder
    pub fn new() -> Self;

    /// Adds the value onto the end of the ArrayBuilder
    ///
    /// Panics:
    /// If the ArrayBuilder is full
    pub fn push(&mut self, t: T);

    /// Complements push, added for consistency
    ///
    /// Panics:
    /// If the ArrayBuilder is empty
    pub fn pop(&mut self) -> T;

    /// Gets the current length of the ArrayBuilder
    pub fn len(&self) -> usize;

    /// Useful compliments to len()
    pub fn is_full(&self) -> bool;
    pub fn is_empty(&self) -> bool;

    /// If the ArrayBuilder is full, returns the successfully initialised array
    /// Otherwise, returns back self
    pub fn build(self) -> Result<[T; N], Self>;

    /// If the ArrayBuilder is full, returns the successfully initialised array
    /// and resets the owned data to uninitialised. If not full, returns None and does nothing.
    pub fn pop_array(&mut self) -> Option<[T; N]>;
}

// Implements AsRef/AsMut for slices. These will return references to
// any initialised data. Useful if extracting data when the ArrayBuilder is not yet full
impl<T, const N: usize> AsRef<[T]> for ArrayBuilder<T, N>;
impl<T, const N: usize> AsMut<[T]> for ArrayBuilder<T, N>;
```

## Example uses

A very simple demonstration:

```rust
let mut arr = ArrayBuilder::<String, 4>::new();

arr.push("a".to_string());
arr.push("b".to_string());
arr.push("c".to_string());
arr.push("d".to_string());

let arr: [String; 4] = arr.build().unwrap();
```

If you want the first 10 square numbers in an array:

```rust
let mut arr = ArrayBuilder::<usize, 10>::new();
for i in 1..=10 {
    arr.push(i*i);
}
arr.build().unwrap()
```

A simple iterator that can iterate over blocks of `N`:

```rust
struct ArrayIterator<I: Iterator, const N: usize> {
    builder: ArrayBuilder<I::Item, N>,
    iter: I,
}

impl<I: Iterator, const N: usize> Iterator for ArrayIterator<I, N> {
    type Item = [I::Item; N];

    fn next(&mut self) -> Option<Self::Item> {
        for _ in self.builder.len()..N {
            // If the underlying iterator returns None
            // then we won't have enough data to return a full array
            // so we can bail early and return None
            self.builder.push(self.iter.next()?);
        }
        // At this point, we must have N elements in the builder
        // So extract the array and reset the builder for the next call
        self.builder.pop_array()
    }
}

impl<I: Iterator, const N: usize> ArrayIterator<I, N> {
    pub fn remaining(&self) -> &[I::Item] {
        &self.builder
    }
}
```

## Possible mis-uses

```rust
let mut arr = ArrayBuilder::<String, 4>::new();

arr.push("a".to_string());
arr.push("b".to_string());
arr.push("c".to_string());
arr.push("d".to_string());
arr.push("e".to_string()); // panic. ArrayBuilder is already full
```

```rust
let mut arr = ArrayBuilder::<String, 4>::new();

arr.push("a".to_string());
arr.push("b".to_string());
arr.push("c".to_string());

let arr: [String; 4] = arr.build().unwrap(); // panic at unwrap. ArrayBuilder is not yet full.
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
