- Feature Name: `stable_wakers`
- Start Date: 2023-03-28
- RFC PR: [rust-lang/rfcs#3404](https://github.com/rust-lang/rfcs/pull/3404)
- Implementation PR: [rust-lang/rust#109706](https://github.com/rust-lang/rust/pull/109706)

# Summary
[summary]: #summary

Make `core::task::Waker` FFI-safe, i.e. ensure that its layout is stable, that `core::task::RawWakerVTable`'s layout is stable, and that the function pointers in the v-table may be used with a stable calling convention.

# Motivation
[motivation]: #motivation

Futures have now become a core feature of Rust, and returning boxed futures from trait-methods has become a very common pattern when working with `async`.

However, due to Rust's unstable default ABI, trait-objects are never FFI-safe (note that this doesn't stop users trying). Multiple crates (such as [`abi_stable`](https://crates.io/crates/abi_stable) and [`stabby`](https://crates.io/crates/stabby)) attempt to solve this problem by providing proc-macros that generate stable v-tables for the traits they annotate.

However, translating `Future` efficiently is made extremely complex and error-prone, as the waker must be transformed into an FFI-safe data structure (complete with `extern fn` adapters to allow calling the v-table's `extern "rust" fn`s), from which a new waker will be constructed on the other side of the FFI boundary. Note that in order to "clone" this new waker, it is necessary that either it has been allocated with a reference counter, or to make a new (possibly reference counted) allocation.

Note that should more efficient ways to convert wakers into and from stable-wakers exist, the complexity of the task raises the chance of misimplementation significantly.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC is entirely implementation-oriented, as the only visible change to the average user would be the ability to pass `core::task::Waker` over the FFI safely.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes two strategies to ABI-stabilize `core::task::Waker`.

In both cases, `core::task::RawWaker` and `core::task::RawWakerVTable` would be annotated with `#[repr(C)]` to ensure stable field ordering. Consideration may be taken as to which field order is more likely to help with the niche optimization of sum-types, but the likelihood of storing `Waker` or its fields in non-`Option` sum types is low, making this a low priority task.

The issue remains: the v-table being constructed from `extern "rust" fn`s, which are FFI-unsafe, where two approaches may be taken. A solution to this would be to add automatically constructed function call adapters to `RawWakerVTable`'s fields:
```rust
#[repr(C)]
pub struct RawWakerVTable {
    clone: unsafe fn(*const ()) -> RawWaker,
    wake: unsafe fn(*const ()),
    wake_by_ref: unsafe fn(*const ()),
    drop: unsafe fn(*const ()),
	clone_adapter: unsafe extern "C" fn(unsafe fn(*const ()) -> RawWaker, *const ()) -> RawWaker,
	other_adapter: unsafe extern "C" fn(unsafe fn(*const ()), *const ()),
}
impl RawWakerVTable {
    pub const fn new(
        clone: unsafe fn(*const ()) -> RawWaker,
        wake: unsafe fn(*const ()),
        wake_by_ref: unsafe fn(*const ()),
        drop: unsafe fn(*const ()),
    ) -> Self {
		unsafe extern "C" fn clone_adapter(clone: unsafe fn(*const ()) -> RawWaker, data: *const ()) -> RawWaker {
			clone(data)
		}
		unsafe extern "C" fn other_adapter(other: unsafe fn(*const ()), data: *const ()) {
			other(data)
		}
        Self { clone, wake, wake_by_ref, drop, clone_adapter, other_adapter }
    }
}
impl Waker {
	pub fn wake(self) {
        let other_adapter = self.waker.vtable.other_adapter;
        let wake = self.waker.vtable.wake;
        let data = self.waker.data;
        // Don't call `drop` -- the waker will be consumed by `wake`.
        crate::mem::forget(self);
        // SAFETY: This is safe because `Waker::from_raw` is the only way
        // to initialize `wake` and `data` requiring the user to acknowledge
        // that the contract of `RawWaker` is upheld.

        // This is also FFI-safe because `other_adapter` adapts the calling convention.
        unsafe { (other_adapter)(wake, data) };
    }
    // Similar implementation changes to `wake_by_ref` and `drop`
}
impl Clone for Waker {
    #[inline]
    fn clone(&self) -> Self {
        Waker {
            // SAFETY: This is safe because `Waker::from_raw` is the only way
            // to initialize `clone` and `data` requiring the user to acknowledge
            // that the contract of [`RawWaker`] is upheld.
            waker: unsafe { (self.waker.vtable.clone_adapter)(clone, self.waker.data) },
        }
    }
}
```

However, this adds a level of indirection to function calls. Providing a `RawWakerVTable::new_with_c_abi` constructor, and making it a stable niche optimized sum type like so would allow removing that indirection when the vtable is constructed from `extern "C" fn`s:
```rust
#[repr(C)]
struct OldRawWakerVTable {
    clone: unsafe fn(*const ()) -> RawWaker,
    wake: unsafe fn(*const ()),
    wake_by_ref: unsafe fn(*const ()),
    drop: unsafe fn(*const ()),
	clone_adapter: Option<unsafe extern "C" fn(unsafe fn(*const ()) -> RawWaker, *const ()) -> RawWaker>,
	other_adapter: Option<unsafe extern "C" fn(unsafe fn(*const ()), *const ())>,
}
#[repr(C)]
struct NewRawWakerVTable {
    clone: unsafe extern "C" fn(*const ()) -> RawWaker,
    wake: unsafe extern "C" fn(*const ()),
    wake_by_ref: unsafe extern "C" fn(*const ()),
    drop: unsafe extern "C" fn(*const ()),
    padding: [*const (); 2] // Must be null
}
#[repr(C)]
pub union RawWakerVTable {
    old: OldRawWakerVTable,
    new: NewRawWakerVTable,
}
impl RawWakerVTable {
    #[deprecated = "This constructor will go away in edition 2024"]
    pub const fn new(
        clone: unsafe fn(*const ()) -> RawWaker,
        wake: unsafe fn(*const ()),
        wake_by_ref: unsafe fn(*const ()),
        drop: unsafe fn(*const ()),
    ) -> Self {
		unsafe extern "C" fn clone_adapter(clone: unsafe fn(*const ()) -> RawWaker, data: *const ()) -> RawWaker {
			clone(data)
		}
		unsafe extern "C" fn other_adapter(other: unsafe fn(*const ()), data: *const ()) {
			other(data)
		}
        Self { old: OldRawWakerVTable { clone, wake, wake_by_ref, drop, clone_adapter, other_adapter }
    }
    pub const fn c_abi(
        clone: unsafe extern "C" fn(*const ()) -> RawWaker,
        wake: unsafe extern "C" fn(*const ()),
        wake_by_ref: unsafe extern "C" fn(*const ()),
        drop: unsafe extern "C" fn(*const ()),
    ) -> Self {
        Self { new: NewRawWakerVTable { clone, wake, wake_by_ref, drop, padding: [core::ptr::null(); 2] } }
    }
}
impl Waker {
	pub fn wake(self) {
        let vtable = self.waker.vtable;
        let data = self.waker.data;
        // Don't call `drop` -- the waker will be consumed by `wake`.
        crate::mem::forget(self);
        // SAFETY: This is safe because `Waker::from_raw` is the only way
        // to initialize `wake` and `data` requiring the user to acknowledge
        // that the contract of `RawWaker` is upheld.

        // This is also FFI-safe because `other_adapter` adapts the calling convention.
        unsafe { match vtable {
            RawWakerVTable { old: OldRawWakerVTable { wake, Some(other_adapter), .. } } => (other_adapter)(wake, data)
            RawWakerVTable { new: NewRawWakerVTable { wake, .. } } => {(new.wake)(data)}
        } };
    }
    // ...
}

```


# Drawbacks
[drawbacks]: #drawbacks

- This has runtime costs: all operations on wakers will go through one more function-call, or have to branch on the representation of the v-table. However, these runtime costs are likely negligible against the typical workload of a waker. In the case of the branching v-table, systems that only use one executor, or homogenous executors with regard to how they construct wakers, will likely have 100% correctness on branch-prediction.
- The proposed v-table layout requires a bit more `'static`ally borrowed memory, although the amount is negligible.
- Should the FFI-safe `Waker` imply a performance penalty, the whole ecosystem would be affected, including projects that do not care about the FFI-safety of futures.
- This introduces an ABI-stability constraint to a core feature of Rust.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- I don't see other means of making `Waker` FFI-safe than the ones proposed, but I'm open to suggestions for other implementations.
- `Waker` could remain FFI-unsafe, but this comes at a high cost for asynchronous projects that need/want a plugin system:
    - Either have to pay the performance penalty of allocation upon first cloning of a `Waker` in every `poll` attempt.
    - Use the `Wakers` _as if_ they were stable, with the associated risks of host and plugin possibly disagreeing about layout and/or calling conventions.
    - Or forego futures altogether: this is an especially high cost as this means plugin code would be forced to have its own executor to do be able to run futures, and communications between host and plugin would have to be piped through FFI-stable async channels to ensure that neither side's executor stalls by running blocking code coming from the other.

# Prior art
[prior-art]: #prior-art

The proposed technique is one of [`stabby`](https://github.com/ZettaScaleLabs/stabby)'s attempted techniques at providing `StableWaker`, discarded due to the difficulty of accessing `core::task::RawWakerVTable`'s original fields and the fact that it doesn't remove the need for allocating on `clone` for `StableWaker` to still be used by the newly-constructed waker.

[waker_getters](https://github.com/rust-lang/rust/issues/87021) states the difficulty of passing wakers across the FFI boundary as its motivation, but doesn't address the issue of calling convention. The current state of its implementation doesn't provide accessors all the way to the v-table, whose layout isn't `#[repr(C)]` either.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Which type of version bump of Rust would make breaking ABI-stability of such a core feature legal? Minor? Edition change? None at all?
- Should padding be added to allow future extension of `Waker`?

# Future possibilities
[future-possibilities]: #future-possibilities

None that I can think of.