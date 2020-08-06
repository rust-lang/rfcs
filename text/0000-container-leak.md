- Feature Name: `container-leak`
- Start Date: 2020-08-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Describe a standard set of methods for converting container types like `Box<T>`, `Arc<T>`, `Vec<T>`, `String` to and from raw pointers.

For containers with a single value like `Box<T>`, `Arc<T>`, and `Rc<T>`, any subset of the following method pairs should be added to work with their raw representations:

- `leak`: leak the container and return an arbitrarily long-lived shared or mutable reference to its allocated content.
- `leak_raw`: leak the container and return a `NonNull<T>` pointer to its content.
- `unleak_raw`: take a previously leaked `NonNull<T>` pointer and restore the container from it.
- `into_raw`: leak the container and return a raw pointer to its content.
- `from_raw`: take a previously leaked raw pointer and restore the container from it.

For growable containers like `Vec<T>` and `String`, any subset of the following method pairs should be added to work with their raw representations:

- `leak`: shrink the container to its allocated length, leak it and return an arbitrarily long-lived shared or mutable reference to its allocated content.
- `leak_raw_parts`: leak the container and return a `NonNull<T>` pointer to its content along with any other state, like the allocated capacity, that would be needed to restore the container.
- `unleak_raw_parts`: take a previously leaked `NonNull<T>` pointer and additional state and restore the container from it.
- `into_raw_parts`: leak the container and return a raw pointer to its content along with any other state that would be needed to restore the container.
- `from_raw_parts`: take a previously leaked raw pointer and additional state and restore the container from it.

The `leak_raw`/`unleak_raw` methods are "modern" semantic alternatives to the existing `into_raw`/`from_raw` pair of methods on containers that use `NonNull<T>` as the pointer type instead of `*const T` or `*mut T`.
Users are encouraged to prefer the `leak_raw`/`unleak_raw` methods over `into_raw`/`from_raw` except for FFI or other niche cases.

# Motivation
[motivation]: #motivation

The `NonNull<T>` type is a non-nullable pointer type that's variant over `T`. `NonNull<T>` has stronger invariants than `*mut T`, but weaker than the internal `Unique<T>`. Since `Unique<T>` isn't planned to be stabilized, `NonNull<T>` is the most appropriate pointer type for containers like `Box<T>` and `Vec<T>` to use as pointers to their inner value.

Unfortunately, `NonNull<T>` was stabilized after methods like `Box::into_raw` and `Vec::from_raw_parts`, which are left working with `*mut T`. Now with the proposed API addition of `Vec::into_raw_parts` we're left with a conundrum. The options appear to be:

- break symmetry with `Vec::from_raw_parts` and diverge from `Box::into_raw` by producing a more semantically accurate `NonNull<T>`.
- not use a newer and more appropriate type for the purpose it exists for and leave it up to users to convert.

This RFC aims to answer this question by specifying any `into_raw`/`from_raw`-like APIs to stay consistent with the precedent set by `Box<T>` and `Vec<T>` of working with `*const T` and `*mut T`, and introduce a similar new API for `NonNull<T>` that is also more semantically typed with respect to `T`. Instead of `Vec::leak_raw` returning a `(*mut T, usize)` pair for its allocated storage, it returns a `NonNull<[T]>` instead.

Keeping the new `leak_raw`/`unleak_raw` API similar to the existing `into_raw`/`from_raw` API is to make them discoverable and avoid new cognitive load for those that are already familiar with `into_raw`/`from_raw`. The semantic names make it clear to a reader what happens to the contents of the container through the conversion into a pointer.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `leak_raw` method can be used to take manual control of the lifetime and access to the contents of a container like `Box<T>`.
The `unleak_raw` method can then be used to later restore the container from its leaked pointer.
It's a fundamental pattern used by specialty data-structures like linked lists to manage non-trivial access and ownership models.
Take the example of `LinkedList<T>`. Internally, it stores `NonNull<T>` pointers to its nodes:

```rust
pub struct LinkedList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
    marker: PhantomData<Box<Node<T>>>,
}
```

The nodes are allocated using `Box<T>`, where they're then leaked into the linked list, then later unleaked back out:

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

The `String::leak_raw` method is a nice representative of the new API for multi-value containers because it produces a more semantic fat-pointer to the string's contents.
Instead of a `(*mut u8, usize)` pair, it returns a `NonNull<str>`, which encodes its length and retains the UTF8 invariant together.
Working with the underlying string is just a matter of dereferencing it, instead of having to reconstruct it through `slice::from_raw_parts` and then `str::from_utf8_unchecked`.

The `leak_raw` and `unleak_raw` methods are recommended over `into_raw` and `from_raw` except in special cases like FFI where `*const T` or `*mut T` might be explicitly wanted. With these new methods, the following existing code:

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

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes the following API for single-value containers (some of these methods are already stable or implemented but unstable):

```rust
impl<T> Box<T> {
    // Already stable
    pub fn leak<'a>(this: Box<T>) -> &'a mut T where T: 'a;

    pub fn leak_raw(this: Box<T>) -> NonNull<T>;
    pub unsafe fn unleak_raw(ptr: NonNull<T>) -> Box<T>;

    // Already stable
    pub fn into_raw(this: Box<T>) -> *mut T;
    // Already stable
    pub unsafe fn from_raw(ptr: *mut T) -> Box<T>;
}

impl<T> Rc<T> {
    pub fn leak_raw(this: Rc<T>) -> NonNull<T>;
    pub unsafe fn unleak_raw(ptr: NonNull<T>);

    // Already stable
    pub fn into_raw(this: Rc<T>) -> *const T;
    // Already stable
    pub unsafe fn from_raw(ptr: *const T) -> Rc<T>;
}

impl<T> Arc<T> {
    pub fn leak_raw(this: Arc<T>) -> NonNull<T>;
    pub unsafe fn unleak_raw(ptr: NonNull<T>) -> Arc<T>;

    // Already stable
    pub fn into_raw(this: Arc<T>) -> *const T;
    // Already stable
    pub unsafe fn from_raw(ptr: *const T) -> Arc<T>;
}
```

and the following API for growable containers (some of these methods are already stable or implemented but unstable):

```rust
impl<T> Vec<T> {
    pub fn leak<'a>(self) -> &'a mut [T] where T: 'a;

    pub fn leak_raw_parts(self) -> (NonNull<[T]>, usize);
    pub fn unleak_raw_parts(ptr: NonNull<[T]>, capacity: usize) -> Vec<T>;

    // Unstable, tracked by: https://github.com/rust-lang/rust/issues/65816
    pub fn into_raw_parts(self) -> (*mut T, usize, usize);
    // Already stable
    pub fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize) -> Vec<T>;
}

impl String {
    pub fn leak<'a>(self) -> &'a mut str;

    pub fn leak_raw_parts(self) -> (NonNull<str>, usize);
    pub fn unleak_raw_parts(ptr: NonNull<str>, capacity: usize) -> String;

    // Unstable, tracked by: https://github.com/rust-lang/rust/issues/65816
    pub fn into_raw_parts(self) -> (*mut u8, usize, usize);
    // Already stable
    pub fn from_raw_parts(ptr: *mut u8, length: usize, capacity: usize) -> String;
}
```

These conversion methods follow the existing semantics of static functions for containers that dereference to their inner value like `Box<T>`, and inherent methods for other containers like `Vec<T>`.

The `NonNull<[T]>` and `NonNull<str>` methods are expected to eventually offer a way to get their length without needing to go through a reference first, but the exact mechanism is left as out-of-scope for this RFC.

# Drawbacks
[drawbacks]: #drawbacks

A drawback of this approach is that it creates a standard that any future containers are expected to adhere to.
It creates more API surface area that needs to be rationalized with future idioms, just like this RFC is attempting to do for `into_raw`/`from_raw` with `NonNull<T>`.
As an example, if a future Rust stabilizes another even more appropriate pointer type then it would need to be fit into this scheme.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An alternative is to just start using `NonNull<T>` going forward and accept the inconsistency with existing methods.
This isn't preferable to keeping new `into_raw`/`from_raw` pairs consistent with the ones that already exist because it forces users to learn the return values for all of these methods by rote instead of being able to rely on simple conventions.

Another is to just use `leak` methods and the conversion from `&T` and `&mut T` into `NonNull<T>` to work with.
This isn't preferable to method pairs that return a `NonNull<T>` and look similar to `into_raw`/`from_raw` because they're less discoverable while still being preferable, and require more steps to leak and unleak than would otherwise be needed.

# Prior art
[prior-art]: #prior-art

The prior art is `Box`, which already has the `leak`, `into_raw` and `from_raw` methods.
It also has unstable `into_raw_non_null`, but is deprecated in favor of `NonNull::from(Box::leak(b))`.
This current workaround is the second alternative listed above, that isn't considered preferable to `Box::leak_raw(b)`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

This RFC doesn't propose a `leak` method for `Rc<T>` or `Arc<T>` but they could be added after working through the motivations.

Do we expect `Box::unleak_raw(NonNull::from(Box::leak(b)))` to work?

# Future possibilities
[future-possibilities]: #future-possibilities

There are other types that should probably be included, like `OsString` and `PathBuf`.
Using `NonNull<[T]` and `NonNull<str>` sets an expectation that `NonNull<T>` will have some APIs for working with these fat-pointer types.
