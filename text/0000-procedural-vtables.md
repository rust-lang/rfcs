- Feature Name: `procedural-vtables`
- Start Date: 2020-08-01
- RFC PR: [rust-lang/rfcs#2967](https://github.com/rust-lang/rfcs/pull/2967)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

All vtable generation happens outside the compiler by invoking a `const fn` that generates said vtable from a generic description of a trait impl. By default, if no vtable generator function is specified for a specific trait, `std::vtable::default` is invoked.

# Motivation
[motivation]: #motivation

The only way we're going to satisfy all users' use cases is by allowing users complete freedom in how their wide pointers' metadata is built. Instead of hardcoding certain vtable layouts in the language (https://github.com/rust-lang/rfcs/pull/2955) we can give users the capability to invent their own layouts at their leisure. This should also help with the work on custom DSTs, as this scheme doesn't specify the size of the wide pointer metadata field.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In order to mark a trait as using a custom vtable layout, you apply the
`#[unsafe_custom_vtable = "foo"]` attribute to the trait declaration.
This is unsafe, because the `foo` function supplies functionality for accessing
`foo` denotes a `const fn` with the signature
`const fn<const IMPL: &'static std::vtable::ImplDescription, T>() -> std::vtable::DstInfo`.

All `impl`s of this trait will now use `foo` for generating the wide pointer
metadata (which contains the vtable). The `WidePointerMetadata` struct that
describes the metadata is a `#[nonexhaustive]` struct. You create instances of it by invoking its `new` method, which gives you a `WidePointerMetadata` that essentially is a `()`. This means you do not have any metadata, similar to `extern type`s. Since the `wide_ptr_metadata` field of the `DstInfo` struct is public, you can now modify it to whatever layout you desire.
You are actually generating the pointer metadata, not a description of it. Since your `const fn` is being interpreted in the target's environment, all target specific information will match up.
Now, you need some information about the `impl` in order to generate your metadata (and your vtable). You get this information partially from the type directly (the `T` parameter), and all the `impl` block specific information is encoded in `ImplDescription`, which you get as a const generic parameter.

As an example, consider the `std::vtable::default` function which is what normally generates your metadata:

```rust
/// DISCLAIMER: this uses a `Vtable` struct which is just a part of the
/// default trait objects. Your own trait objects can use any metadata and
/// thus "vtable" layout that they want.
pub const fn default<
    const IMPL: &'static std::vtable::ImplDescription,
    T,
>() -> std::vtable::DstInfo {
    let mut info = DstInfo::new();
    // We generate the metadata and put a pointer to the metadata into 
    // the field. This looks like it's passing a reference to a temporary
    // value, but this uses promotion
    // (https://doc.rust-lang.org/stable/reference/destructors.html?highlight=promotion#constant-promotion),
    // so the value lives long enough.
    info.unsize(|ptr| {
        (
            ptr,
            &default_meta::<IMPL>() as *const _ as *const (),
        )
    });
    // We supply a function for invoking trait methods
    info.method_id_to_fn_ptr(|idx, parents, meta| unsafe {
        
    });
    info.size_of(|meta| unsafe {
        let meta = *(meta as *&'static Vtable<0>);
        meta.size
    });
    info.align_of(|meta| unsafe {
        let meta = *(meta as *&'static Vtable<0>);
        meta.align
    });
    info.drop(|meta| unsafe {
        let meta = *(meta as *&'static Vtable<0>);
        meta.drop
    });
    info
}

// Compute the total number of methods, including super-traits
const fn num_methods<
    const IMPL: &'static std::vtable::ImplDescription,
>() -> usize {
    let mut n = IMPL.methods.len();
    let mut current = IMPL;
    while let Some(next) = current.parent {
        n += next.methods.len();
        current = next.parent;
    }
    n
}

const fn default_meta<
    const IMPL: &'static std::vtable::ImplDescription,
    T,
>() -> &'static VTable<{num_methods::<IMPL>()}> {
    // The metadata of a wide pointer for trait objects is a reference
    // to the vtable.
    &default_vtable::<IMPL>()
}

const fn default_vtable<
    const IMPL: &'static std::vtable::ImplDescription,
    T,
>() -> VTable<{num_methods::<IMPL>()}> {
    // `VTable` is a `#[repr(C)]` type with fields at the appropriate
    // places.
    let mut vtable = VTable {
        size: std::mem::size_of::<T>(),
        align: std::mem::size_of::<T>(),
        drop: transmute::<unsafe fn(*mut T), unsafe fn (*mut ())>(std::ptr::drop_in_place::<T>),
        methods: [std::ptr::null(); num_methods::<IMPL>()],
    };
    let mut i = 0;
    while i < num_methods::<IMPL>() {
        if let Some(method) = IMPL.methods[i] {
            // The `method` variable is a function pointer, but
            // cast to `*const ()`.
            vtable.methods[i] = method;
        }
    }
    vtable
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The two types `ImplDescription` and `DstInfo` are `#[nonexhaustive]` in order to allow arbitrary extension in the future.
Instances of the `ImplDescription` struct are created by rustc, basically replacing today's vtable generation, and then outsourcing the actual vtable generation to the const evaluator. The wide pointer metadata can be copied verbatim from the `DstInfo`'s `meta` field. When obtaining function pointers from vtables, instead of computing an offset, the `method_id_to_fn_ptr` function is invoked at runtime and computes a function pointer after being given a method index, the indices of all parents and a pointer to a metadata field. Through the use of MIR optimizations (e.g. inlining), the final LLVM assembly is tuned to be exactly the same as today.

These types' declarations are provided below:

```rust
#[nonexhaustive]
struct ImplDescription {
    pub methods: &'static [*const ()],
    pub parent: &'static ImplDescription,
}
#[nonexhaustive]
struct DstInfo {
    unsize: *const (),
    method_id_to_fn_ptr: fn(usize, &'static [usize], *const ()) -> *const (),
    size_of: fn(*const()) -> usize,
    align_of: fn(*const()) -> usize,
    drop: fn drop(*mut ()),
}
impl DstInfo {
    fn new() -> Self {
        Self {
            meta: &(),
            method_id_to_fn_ptr: |idx, parents, meta| {
                panic!("method called on trait object with custom vtable without method_id_to_fn_ptr")
            },
        }
    }
    unsafe fn unsize<T, WIDE_PTR>(f: fn(*const T)) -> WIDE_PTR) {
        self.drop = transmute(f);
    }
    /// The given function returns a function pointer to the method that
    /// is being requested.
    /// * The first argument is the method index,
    /// * the second argument is a list of indices used to traverse the
    ///   super-trait tree to find the trait whose method is being invoked, and
    /// * the thrid argument is a pointer to the metadata (so in case of trait objects, usually it would be `&'static &'static Vtable`).
    ///   This indirection is necessary, because we don't know the size of the metadata.
    unsafe fn method_id_to_fn_ptr(f: fn(usize, &'static [usize], *const ()) -> *const ()) {
        self.method_id_to_fn_ptr = f;
    }
    unsafe fn size_of(f: fn(*const ()) -> usize) {
        self.size_of = f;
    }
    unsafe fn align_of(f: fn(*const ()) -> usize) {
        self.align_of = f;
    }
    unsafe fn drop(f: fn(*const ()) -> fn(*mut ())) {
        self.drop = f;
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

* This may be a serious case of overengineering. We're basically taking vtables out of the language and making dynamic dispatch on trait objects a user definable thing.
* This may slow down compilation, likely entirely preventable by keeping a special case in the compiler for regular trait objects.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Don't use const generics

If we wait until we have a heap in const eval, we can use `Vec`s and `Box`es, which would allow us to avoid the const generics scheme, likely making all the code less roundabout.

We can still expose the current scheme and once we get heap in const eval, we can actually implement a convenience layer in user code. So this is basically like procedural macros, where a stringy API is exposed and user code (`syn`) gives a better API.

# Prior art
[prior-art]: #prior-art

I don't know of any prior art where a compile-time language has procedural vtable generation. You can see lots of similar tricks being employed in dynamic languages like ruby and python, where "types" are built by changing functions in objects at runtime. If this is just done at startup and not during actual program execution, it is essentially the same concept here, except that our "startup phase" is at compile-time.

## Other Custom DST RFCs

This list is shamelessly taken from [strega-nil's Custom DST RFC](https://github.com/rust-lang/rfcs/pull/2594):

- [mzabaluev's Version](https://github.com/rust-lang/rfcs/pull/709)
- [strega-nil's new version](https://github.com/rust-lang/rfcs/pull/2594)
- [strega-nil's Old Version](https://github.com/rust-lang/rfcs/pull/1524)
- [japaric's Pre-RFC](https://github.com/japaric/rfcs/blob/unsized2/text/0000-unsized-types.md)
- [mikeyhew's Pre-RFC](https://internals.rust-lang.org/t/pre-erfc-lets-fix-dsts/6663)
- [MicahChalmer's RFC](https://github.com/rust-lang/rfcs/pull/9)
- [nrc's Virtual Structs](https://github.com/rust-lang/rfcs/pull/5)
- [Pointer Metadata and VTable](https://github.com/rust-lang/rfcs/pull/2580)
- [Syntax of ?Sized](https://github.com/rust-lang/rfcs/pull/490)

This RFC differentiates itself from all the other RFCs in that it provides a procedural way to generate vtables, thus also permitting arbitrary user-defined compile-time conditions by aborting via `panic!`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Do we want this kind of flexibility? With power comes responsibility...
- I believe we can do multiple super traits, including downcasts with this scheme, make sure that's true.

# Future possibilities
[future-possibilities]: #future-possibilities

* This scheme can be used to generate C++-like vtables where the data and vtable are in the same allocation by making the `unsize` function create a heap allocation and write the value and the vtable into this new allocation. It's not clear yet how to handle this kind of "ownership takeover", since all unsizing in Rust currently happens either in a borrowed manner or in `Box`, which is special anyway.
* We can change slice (`[T]`) types to be backed by a `trait Slice` which uses this scheme to generate just a single `usize` for the metadata
* We can use this scheme and remove `extern type`s from the language, as they just become a trait with a custom metadata generator that uses `()` for the metadata. So `CStr` doesn't become an extern type, instead it becomes a trait.
* We can handle traits with parents which are created by a different metadata generator function, we just need to figure out how to communicate this to such a function so it can special case this situation.
* We can give the vtable pointer of the super traits to the constructor function, so it doesn't have to recompute anything and just grab the info off there. Basically `ImplDescription::parent` would not be a pointer to the parent, but a struct which contains at least the pointer to the parent and the pointer to the `DstInfo`
* Totally off-topic, but a similar scheme can be used to generate type declarations procedurally with const eval.