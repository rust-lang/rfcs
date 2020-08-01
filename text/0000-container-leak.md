- Feature Name: `container-leak`
- Start Date: 2020-08-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Describe a standard set of methods for converting container types like `Box<T>`, `Arc<T>`, `Vec<T>`, `String` between their raw representations.

For containers with a single value like `Box`, `Arc`, and `Rc`, the following methods should be added to work with their raw representations:

- `leak`: leak the container and return an arbitrarily long-lived shared or mutable reference to its allocated content.
- `leak_raw`: leak the container and return a `NonNull` pointer to its content.
- `unleak_raw`: take a previously leaked `NonNull` pointer and restore the container from it.
- `into_raw`: leak the container and return a raw pointer to its content.
- `from_raw`: take a previously leaked raw pointer and restore the container from it.

For growable containers like `Vec` and `String`, the following methods should be added to work with their raw representations:

- `leak`: shrink the container to its allocated length, leak it and return an arbitrarily long-lived shared or mutable reference to its allocated content.
- `leak_raw_parts`: leak the container and return a `NonNull` pointer to its content along with any other state, like the allocated capacity, that would be needed to restore the container.
- `unleak_raw_parts`: take a previously leaked `NonNull` pointer and additional state and restore the container from it.
- `into_raw_parts`: leak the container and return a raw pointer to its content along with any other state that would be needed to restore the container.
- `from_raw_parts`: take a previously leaked raw pointer and additional state and restore the container from it.

The `leak_raw` and `unleak_raw` methods are "modern" semantic alternatives to the existing `from_raw`/`into_raw` pair of methods on containers that use `NonNull` instead of `*const` or `*mut`.
Users are encouraged to use the `leak_raw`/`unleak_raw` pair over the `from_raw`/`into_raw` pair except for FFI.
With these new methods, the following code:

```rust
let b: Box<T> = Box::new(t);

let ptr: *mut T = Box::into_raw(b);

..

let b: Box<T> = unsafe { Box::from_raw(ptr) };
```

can be replaced with:

```rust
let b: Box<T> = Box::new(t);

let ptr: NonNull<T> = Box::leak_raw(b);

..

let b: Box<T> = unsafe { Box::unleak_raw(ptr) };
```

# Motivation
[motivation]: #motivation

Why are we doing this? What use cases does it support? What is the expected outcome?

The `NonNull<T>` type is a non-nullable pointer type that's variant over `T`. `NonNull<T>` has stronger invariants than `*mut T`, but weaker than the internal `Unique<T>`. Since `Unique<T>` isn't planned to be stabilized, `NonNull<T>` is the most appropriate pointer type for containers like `Box<T>` and `Vec<T>` to use as pointers to their inner value.

Unfortunately, `NonNull<T>` was stabilized after methods like `Box::into_raw` and `Vec::from_raw_parts`, which are left working with `*mut T`. Now with proposed API addition of `Vec::into_raw_parts` we're left with the conundrum. The options appear to be:

- break symmetry with `Vec::from_raw_parts` and diverge from `Box::into_raw` by producing a more semantically correct `NonNull<T>`.
- not use a newer and more appropriate type for the purpose it exists for.

This RFC aims to solve this by specifying any `from_raw`/`into_raw`-like APIs to stay consistent with the precedent set by `Box` and `Vec` of working with raw pointers, and introduce a similar new API for `NonNull<T>` that is also more semantically typed with respect to `T`. Instead of `Vec::leak_raw` returning a `(*mut T, usize)` pair for its allocated storage, it returns a `NonNull<[T]>` instead.

Keeping the new `leak_raw`/`unleak_raw` API similar to the existing `into_raw`/`from_raw` API is to make them discoverable and avoid new cognitive load for those that are already familiar with `into_raw`/`from_raw`. The semantic names make it clear to a reader what happens to the contents of the container through the conversion into a pointer.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `leak_raw` and `unleak_raw` methods can be used to take manual control of the contents of a container like `Box<T>` and restore it later.
It's a fundamental pattern used by specialty datastructures like linked lists to manage non-trivial access and ownership models.
Take the example of `LinkedList<T>`. Internally, it stores `NonNull` pointers to its nodes:

```rust
pub struct LinkedList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
    marker: PhantomData<Box<Node<T>>>,
}
```

The nodes are allocated using `Box`, where they're then leaked into the linked list, then later unleaked back out:

```rust
impl<T> LinkedList<T> {
    fn push_front_node(&mut self, mut node: Box<Node<T>>) {
        unsafe {
            node.next = self.head;
            node.prev = None;

            // Leak the contents of `node` and return a `NonNull<Node<T>>`.
            // It's now the responsibility of `LinkedList<T>` to manage.
            let node = Some(Box::leak_raw(node));

            match self.head {
                None => self.tail = node,
                Some(head) => (*head.as_ptr()).prev = node,
            }

            self.head = node;
            self.len += 1;
        }
    }

    fn pop_front_node(&mut self) -> Option<Box<Node<T>>> {
        self.head.map(|node| unsafe {

            // Unleak the contents of `node` and return a `Box<Node<T>>`.
            // It's now the responsibility of `Box<T>` to manage.
            let node = Box::unleak_raw(node.as_ptr());
            self.head = node.next;

            match self.head {
                None => self.tail = None,
                Some(head) => (*head.as_ptr()).prev = None,
            }

            self.len -= 1;
            node
        })
    }
}
```

The `leak_raw` and `unleak_raw` methods are recommended over `into_raw` and `from_raw` except in special cases like FFI where raw pointers might be explicitly wanted.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes the following API for single-value containers:

```rust
impl<T> Box<T> {
    pub fn leak<'a>(this: Box<T>) -> &'a mut T where T: 'a;

    pub fn leak_raw(this: Box<T>) -> NonNull<T>;
    pub unsafe fn unleak_raw(ptr: NonNull<T>) -> Box<T>;

    pub fn into_raw(this: Box<T>) -> *mut T;
    pub unsafe fn from_raw(ptr: *mut T) -> Box<T>;
}

impl<T> Rc<T> {
    pub fn leak<'a>(this: Rc<T>) -> &'a T where T: 'a;

    pub fn leak_raw(this: Rc<T>) -> NonNull<T>;
    pub unsafe fn unleak_raw(ptr: NonNull<T>);

    pub fn into_raw(this: Rc<T>) -> *const T;
    pub unsafe fn from_raw(ptr: *const T) -> Rc<T>;
}

impl<T> Arc<T> {
    pub fn leak<'a>(this: Arc<T>) -> &'a T where T: 'a;

    pub fn leak_raw(this: Arc<T>) -> NonNull<T>;
    pub unsafe fn unleak_raw(ptr: NonNull<T>) -> Arc<T>;

    pub fn into_raw(this: Arc<T>) -> *const T;
    pub unsafe fn from_raw(ptr: *const T) -> Arc<T>;
}
```

and the following API for growable containers:

```rust
impl<T> Vec<T> {
    pub fn leak<'a>(self) -> &'a mut [T] where T: 'a;

    pub fn leak_raw_parts(self) -> (NonNull<[T]>, usize);
    pub fn unleak_raw_parts(ptr: NonNull<[T]>, capacity: usize) -> Vec<T>;

    pub fn into_raw_parts(self) -> (*mut T, usize, usize);
    pub fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize) -> Vec<T>;
}

impl String {
    pub fn leak<'a>(self) -> &'a mut str;

    pub fn leak_raw_parts(self) -> (NonNull<str>, usize);
    pub fn unleak_raw_parts(ptr: NonNull<str>, capacity: usize) -> String;

    pub fn into_raw_parts(self) -> (*mut u8, usize, usize);
    pub fn from_raw_parts(ptr: *mut u8, length: usize, capacity: usize) -> String;
}
```

These conversion methods follow the existing semantics of static functions for containers that auto-deref to their inner value like `Box`, and inherent methods for other containers like `Vec`.

The `NonNull<[T]>` and `NonNull<str>` methods are expected to eventually offer a way to get their length without needing to go through a reference first, but the exact mechanism is left as out-of-scope for this RFC.

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
Also consider how the this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
