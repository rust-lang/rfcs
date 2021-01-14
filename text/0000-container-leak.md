- Feature Name: `container-leak`
- Start Date: 2020-08-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Describe a standard set of methods for converting container types like `Box<T>`, `Arc<T>`, `Vec<T>`, `String` to and from raw pointers. This RFC doesn't suggest all of these methods actually exist, only that if they do they follow the standard laid out.

For containers with a single value like `Box<T>`, `Arc<T>`, and `Rc<T>`, any subset of the following method pairs should be added to work with their raw representations:

- `leak`: leak the container and return an arbitrarily long-lived shared or mutable reference to its allocated content.
- `leak_raw`: leak the container and return a `NonNull<T>` pointer to its content. The type `T` is the same as `Deref::Target`, so `Self::leak_raw(value)` is equivalent to `NonNull::from(&*self)` and `NonNull::from(Self::leak(value))`.
- `unleak_raw`: take a previously leaked `NonNull<T>` pointer and restore the container from it.
- `into_raw`: leak the container and return a raw pointer to its content.
- `from_raw`: take a previously leaked raw pointer and restore the container from it.

For multi-value containers like `Vec<T>` and `String`, any subset of the following method pairs should be added to work with their raw representations:

- `leak`: leak the container and return an arbitrarily long-lived shared or mutable reference to its allocated content. The contents may or may not be shrinked as an implementation detail of the container.
- `leak_raw_parts`: leak the container and return a `NonNull<T>` pointer to its content along with any other state, like the allocated capacity, that would be needed to restore the container. The type `T` is the same as `Deref::Target`, so `NonNull::from(&*self)` is equivalent to `NonNull::from(self.leak())` and `NonNull::from(self.leak_raw_parts().0)`.
- `unleak_raw_parts`: take a previously leaked `NonNull<T>` pointer and additional state and restore the container from it.
- `into_raw_parts`: leak the container and return a raw pointer to its content along with any other state that would be needed to restore the container.
- `from_raw_parts`: take a previously leaked raw pointer and additional state and restore the container from it.

The `leak_raw`/`unleak_raw` methods are "modern" semantic alternatives to the existing `into_raw`/`from_raw` pair of methods on containers that use `NonNull<T>` as the pointer type instead of `*const T` or `*mut T`.
Users are encouraged to prefer the `leak_raw`/`unleak_raw` methods over `into_raw`/`from_raw` except for the important case where they need FFI-safety.

# Motivation
[motivation]: #motivation

The `NonNull<T>` type is a non-nullable pointer type that's variant over `T`. `NonNull<T>` has stronger invariants than `*mut T`, but weaker than the internal `Unique<T>`.
Since `Unique<T>` isn't planned to be stabilized, `NonNull<T>` is the most appropriate pointer type for containers like `Box<T>` and `Vec<T>` to use as pointers to their inner value.

Unfortunately, `NonNull<T>` was stabilized after methods like `Box::into_raw` and `Vec::from_raw_parts`, which are left working with `*mut T`.
Now with the proposed API addition of `Vec::into_raw_parts` we're left with a conundrum. The options appear to be:

- break symmetry with `Vec::from_raw_parts` and diverge from `Box::into_raw` by producing a more semantically accurate `NonNull<T>`.
- not use a newer and more appropriate type for the purpose it exists for and leave it up to users to convert.

This RFC aims to answer this question by specifying any `into_raw`/`from_raw`-like APIs to stay consistent with the precedent set by `Box<T>` and `Vec<T>` of working with `*const T` and `*mut T`, and introduce a similar new API for `NonNull<T>` that is also more semantically typed with respect to `T`.
Instead of `Vec::leak_raw` returning a `(*mut T, usize)` pair for its allocated storage, it returns a `NonNull<[T]>` instead.

Keeping the new `leak_raw`/`unleak_raw` API similar to the existing `into_raw`/`from_raw` API is to make them discoverable and avoid new cognitive load for those that are already familiar with `into_raw`/`from_raw`.
The semantic names make it clear to a reader what happens to the contents of the container through the conversion into a pointer.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## When do I use `leak_raw`/`unleak_raw`?

The `leak_raw`/`unleak_raw` and `leak_raw_parts`/`unleak_raw_parts` methods are good for pure Rust datastructures that would probably use references if it was possible to describe their non-trivial access and ownership requirements through them.

The `leak_raw` method can be used to take manual control of the lifetime and access to the contents of a container like `Box<T>`.
The `unleak_raw` method can then be used to later restore the container from its leaked pointer.

Take the example of `LinkedList<T>` from the standard library. Internally, it stores `NonNull<T>` pointers to its nodes:

```rust
pub struct LinkedList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
    marker: PhantomData<Box<Node<T>>>,
}
```

The nodes are allocated using `Box<T>`, where they're then leaked into the linked list, then later unleaked back out.
This can be done using `leak_raw`/`unleak_raw`:

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

The `leak_raw_parts` method is the equivalent of `leak_raw` for multi-value containers like `String` that return extra data beyond the pointer needed to reconstruct the container later.
The `unleak_raw_parts` method is the equivalent of `unleak_raw`.

The `String::leak_raw_parts` method is a nice example of the new `leak_raw` API because it returns the most accurate pointer type possible to represent the raw string data.
Instead of a `(*mut u8, usize)` pair for the pointer and length, it returns a `NonNull<str>`, which encodes its length and retains the UTF8 invariant together.
The following example shows how `leak_raw_parts` makes it easier to work with the leaked string than `into_raw_parts`:

```diff
let string = String::from("🗻∈🌏");

+ let (ptr, cap): (NonNull<str>, usize) = string.leak_raw_parts();
- let (ptr, len, cap): (*mut u8, usize, usize) = string.into_raw_parts();

+ assert_eq!(Some("🗻"), unsafe { ptr.as_ref().get(0..4) });
- assert_eq!(Some("🗻"), unsafe { str::from_utf8_unchecked(slice::from_raw_parts(ptr, len)).get(0..4) });

+ let string = String::unleak_raw_parts(ptr, cap);
- let string = String::from_raw_parts(ptr, len, cap);
```

## When do I use `into_raw`/`from_raw`?

The `into_raw`/`from_raw` and `into_raw_parts`/`from_raw_parts` methods are good for FFI where a Rust type needs to be used by non-Rust code.

The `*mut T`, `*const T`, and `usize` types returned by these methods typically have a direct counterpart in the target language, so they don't require learning new concepts for users that are familiar with raw pointers.

As an example, it's common to share complex Rust values opaquely by boxing them and passing raw pointers to-and-fro.
Take this example [from The Rust FFI Guide][ffi-guide] that wraps a web request:

```rust
#[no_mangle]
pub unsafe extern "C" fn request_create(url: *const c_char) -> *mut Request {
    if url.is_null() {
        return ptr::null_mut();
    }

    let raw = CStr::from_ptr(url);

    let url_as_str = match raw.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let parsed_url = match Url::parse(url_as_str) {
        Ok(u) => u,
        Err(_) => return ptr::null_mut(),
    };

    let req = Request::new(parsed_url, Method::Get);
    
    // Get a stable address for the request
    Box::into_raw(Box::new(req))
}

#[no_mangle]
pub unsafe extern "C" fn request_destroy(req: *mut Request) {
    if !req.is_null() {
        // Reinterpret the stable address as a previously allocated box
        drop(Box::from_raw(req));
    }
}
```

In this example, a reader only needs to consider one kind of pointer type (technically `*const T` and `*mut T` are different types, but one could read them like `T*` from other languages with a sharing annotation).
This API could use `Option<NonNull<Request>>` instead of `*mut Request` to force null checking in `request_destroy`, but that requires the author to juggle more concepts to write.
They'd need to understand that while `NonNull<T>` has the same representation as `*const T`, it has the same semantics as `Option<NonNull<T>>`.

The `into_raw_parts` method is the equivalent of `into_raw` for multi-value containers like `Vec<T>` that split the fat pointer into its FFI-safe parts.
The `from_raw_parts` method is the equivalent of `from_raw`.

An FFI over `Vec<u8>` is a nice example of when `into_raw_parts` can be helpful over `leak_raw_parts`.
An FFI should only be built from FFI-safe types that have a well-known representation, but the fat `NonNull<[u8]>` pointer returned by `leak_raw_parts` (and consequently `*const [u8]`) is not considered FFI-safe.
That's not a problem for `into_raw_parts` though because it only returns FFI-safe `*mut u8` and `usize` types.

The following example shows how `into_raw_parts` makes it easier to work with FFI-safe values than `leak_raw_parts`:

```diff
#[repr(C)]
pub struct RawVec {
    ptr: *mut u8,
    len: usize,
    cap: usize
}

#[no_mangle]
pub unsafe extern "C" fn vec_create() -> RawVec {
    let v = vec![0u8; 512];
    
+    let (ptr, len, cap) = v.into_raw_parts();
-    let (ptr, cap) = v.leak_raw_parts();
-    let (ptr, len) = (ptr.cast::<u8>().as_ptr(), ptr.len());

    RawVec { ptr, len, cap }
}

#[no_mangle]
pub unsafe extern "C" fn vec_destroy(vec: RawVec) {
    if !vec.ptr.is_null() {
+        drop(Vec::from_raw_parts(vec.ptr, vec.len, vec.cap));
-        drop(Vec::unleak_raw_parts(NonNull::slice_from_raw_parts(NonNull::new_unchecked(vec.ptr), vec.len), vec.cap));
    }
}
```

[ffi-guide]: https://michael-f-bryan.github.io/rust-ffi-guide/basic_request.html#creating-the-c-interface

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

and the following API for multi-value containers (some of these methods are already stable or implemented but unstable):

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

These conversion methods follow the existing semantics of static functions for containers that dereference to their inner value like `Box<T>`, and inherent methods for others.

The docs for the `into_raw`/`from_raw` methods will point users to `leak_raw`/`unleak_raw` unless they need FFI-safety.

The `NonNull<[T]>` and `NonNull<str>` methods are expected to eventually offer a way to get their length without needing to go through a reference first, but the exact mechanism is left as out-of-scope for this RFC.

# Drawbacks
[drawbacks]: #drawbacks

A drawback of this approach is that it creates a standard that any future containers are expected to adhere to.
It creates more API surface area that needs to be rationalized with future idioms, just like this RFC is attempting to do for `into_raw`/`from_raw` with `NonNull<T>`.
As an example, if a future Rust stabilizes another even more appropriate pointer type then it would need to be fit into this scheme.

It introduces more APIs so users have to choose the right one for their usecase instead of just trying to make the only option available work for them.
With clear guidance in the documentation for these methods and similarities in their design this shouldn't be an issue in practice.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An alternative is to just start using `NonNull<T>` going forward and accept the inconsistency with existing methods.
This isn't preferable to keeping new `into_raw`/`from_raw` pairs consistent with the ones that already exist because it forces users to learn the return values for all of these methods by rote instead of being able to rely on simple conventions.

Another is to just use `leak` methods and the conversion from `&T` and `&mut T` into `NonNull<T>` to work with.
This isn't preferable to method pairs that return a `NonNull<T>` and look similar to `into_raw`/`from_raw` because they're less discoverable while still being preferable for common usecases, and require more steps to leak and unleak than would otherwise be needed.

Another is to deprecate `into_raw`/`from_raw` in favor of `leak_raw().as_ptr()` and `NonNull::new_unchecked(ptr)`.
This makes it easier to discover the preferred API for working with raw container contents and the expense of more machinery in FFI use-cases.
This isn't preferable to guidance in docs on both sets of methods because it puts more burden on FFI code and deprecates APIs that are already perfectly suited to their needs.
This could possibly be worked around by making it easier to convert types like `NonNull<[T]>` into a `(*mut T, usize)` pair.

# Prior art
[prior-art]: #prior-art

The prior art is `Box<T>`, which already has the `leak`, `into_raw` and `from_raw` methods.
It also has the unstable `into_raw_non_null`, but is deprecated in favor of `NonNull::from(Box::leak(b))`.
This current workaround is the second alternative listed above, that isn't considered preferable to `Box::leak_raw(b)`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

This RFC doesn't propose a `leak` method for `Rc<T>` or `Arc<T>` but they could be added after working through the motivations.

Do we expect `Box::unleak_raw(NonNull::from(Box::leak(b)))` to work?

# Future possibilities
[future-possibilities]: #future-possibilities

There are other types that should probably be included, like `OsString` and `PathBuf`.
Using `NonNull<[T]` and `NonNull<str>` sets an expectation that `NonNull<T>` will have some APIs for working with these fat-pointer types.
