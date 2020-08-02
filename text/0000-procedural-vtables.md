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
This is unsafe, because the `foo` function supplies functionality for accessing the `self` pointers.
`foo` denotes a `const fn` with the signature
`const fn<const IMPL: &'static std::vtable::TraitDescription>() -> std::vtable::DstInfo`.

Additionally, if your trait supports automatically unsizing from the types it's implemented for (unlike `CStr`, `str` and `[T]`, which require type-specific logic), you can supply your trait with an `unsizing` function by
specifying `#[unsafe_custom_unsize = "bar"]`. `bar` denotes a `const fn`
with the signature
`const fn<const IMPL: &'static std::vtable::TraitDescription, T>() -> std::vtable::UnsizeInfo`, where `T` is your concrete type.

All `impl`s of this trait will now use `foo` for generating the wide pointer
metadata (which contains the vtable). The `WidePointerMetadata` struct that
describes the metadata is a `#[nonexhaustive]` struct. You create instances of it by invoking its `new` method, which gives you a `WidePointerMetadata` that essentially is a `()`. This means you do not have any metadata, similar to `extern type`s. Since the `wide_ptr_metadata` field of the `DstInfo` struct is public, you can now modify it to whatever layout you desire.
You are actually generating the pointer metadata, not a description of it. Since your `const fn` is being interpreted in the target's environment, all target specific information will match up.
Now, you need some information about the `impl` in order to generate your metadata (and your vtable). You get this information partially from the type directly (the `T` parameter), and all the `impl` block specific information is encoded in `TraitDescription`, which you get as a const generic parameter.

As an example, consider the function which is what normally generates your metadata. Note that if these methods are used for generic traits, the method needs additional generic parameters, one for each parameter of the trait.
See the `[T]` demo further down for an example.

```rust
/// If the `owned` flag is `true`, this is an owned conversion like
/// in `Box<T> as Box<dyn Trait>`. This distinction is important, as
/// unsizing that creates a vtable in the same allocation as the object
/// (like C++ does), cannot work on non-owned conversions. You can't just
/// move away the owned object. The flag allows you to forbid such
/// unsizings by triggering a compile-time `panic` with an explanation
/// for the user.
pub const fn custom_unsize<
    T,
    const IMPL: &'static std::vtable::TraitDescription,
    const _owned: bool,
>(*const T) -> (*const T, &'static VTable<{num_methods::<IMPL>()}>) {
    // We generate the metadata and put a pointer to the metadata into 
    // the field. This looks like it's passing a reference to a temporary
    // value, but this uses promotion
    // (https://doc.rust-lang.org/stable/reference/destructors.html?highlight=promotion#constant-promotion),
    // so the value lives long enough.
    (
        ptr,
        &default_vtable::<T, IMPL>() as *const _ as *const (),
    )
}

/// DISCLAIMER: this uses a `Vtable` struct which is just a part of the
/// default trait objects. Your own trait objects can use any metadata and
/// thus "vtable" layout that they want.
pub const fn custom_vtable<
    const IMPL: &'static std::vtable::TraitDescription,
>() -> std::vtable::DstInfo {
    let mut info = DstInfo::new();
    unsafe {
        // We supply a function for invoking trait methods.
        // This is always inlined and will thus get optimized to a single
        // deref and offset (strong handwaving happening here).
        info.method_id_to_fn_ptr(|mut idx, parents, meta| unsafe {
            let meta = *(meta as *const (*const(), &'static Vtable<{num_methods::<IMPL>()}>));
            let mut table = IMPL;
            for parent in parents {
                // we don't support multi-parents yet
                assert_eq!(parent, 0);
                idx += table.methods.len();
                // Never panics, there are always fewer or equal number of
                // parents given as the argument as there are in reality.
                table = table.parent.unwrap();
            }
            meta.1.methods[idx];
        });
        info.size_of(|meta| unsafe {
            let meta = *(meta as *const (*const(), &'static Vtable<{num_methods::<IMPL>()}>));
            meta.1.size
        });
        info.align_of(|meta| unsafe {
            let meta = *(meta as *const (*const(), &'static Vtable<{num_methods::<IMPL>()}>));
            meta.1.align
        });
        info.drop(|meta| unsafe {
            let meta = *(meta as *const (*const(), &'static Vtable<{num_methods::<IMPL>()}>));
            meta.1.drop
        });
        info.self_ptr(|meta| unsafe {
            let meta = *(meta as *const (*const(), &'static Vtable<{num_methods::<IMPL>()}>));
            meta.0
        });
    }
    info
}

// Compute the total number of methods, including super-traits
const fn num_methods<
    const IMPL: &'static std::vtable::TraitDescription,
>() -> usize {
    let mut n = IMPL.methods.len();
    let mut current = IMPL;
    while let Some(next) = current.parent {
        n += next.methods.len();
        current = next.parent;
    }
    n
}

const fn default_vtable<
    T,
    const IMPL: &'static std::vtable::TraitDescription,
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
    let mut current = IMPL;
    loop {
        match IMPL.methods.get(i) {
            Some(Some(method)) => {
                // The `method` variable is a function pointer, but
                // cast to `*const ()`.
                vtable.methods[i] = method;
                i += 1;
            },
            Some(None) => {
                // Method that cannot be called on this vtable
                i += 1;
            }
            None => match current.parent {
                Some(next) => {
                    current = next;
                    i = 0;
                },
                None => break,
            }
        }
    }
    vtable
}
```

Now, if you want to implement a fancier vtable, this RFC enables that.

## Null terminated strings (std::ffi::CStr)

This is how I see all extern types being handled.
There can be no impls of `CStr` for any type, because the `unsize`
function is missing. The `CStr`

```rust

// Not setting the `unsafe_custom_unsize` function, there's no sized
// equivalent like with normal traits. See the future
// extensions section for more details on unsizing.
#[unsafe_custom_vtable = "c_str"]
pub trait CStr {}

pub const fn c_str<
    const IMPL: &'static std::vtable::TraitDescription,
>() -> std::vtable::DstInfo {
    let mut info = DstInfo::new();
    unsafe {
        info.method_id_to_fn_ptr(|idx, parents, meta| {
            panic!("CStr has no trait methods, it's all inherent methods acting on the pointer")
        });
        info.size_of(|meta| unsafe {
            let ptr = *(meta as *const *const u8);
            strlen(ptr)
        });
        info.align_of(|meta| 1);
        // Nothing to drop (just `u8`s) and we are not in charge of dealloc
        info.drop(|meta| None);
        info.self_ptr(|ptr| {
            let ptr = *(meta as *const *const u8);
            ptr
        });
    }
    info
}
```

## `[T]` as sugar for a `Slice` trait

We could remove `[T]` from the language and just make it desugar to
a `std::slice::Slice` trait.

```rust
#[unsafe_custom_vtable = "slice"]
pub trait Slice<T> {}

pub const fn slice<
    T,
    const IMPL: &'static std::vtable::TraitDescription,
>() -> std::vtable::DstInfo {
    let mut info = DstInfo::new();
    unsafe {
        info.method_id_to_fn_ptr(|idx, parents, meta| {
            panic!("CStr has no trait methods, it's all inherent methods acting on the pointer")
        });
        info.size_of(|meta| unsafe {
            let ptr = *(meta as *const (*const T, usize);
            ptr.1
        });
        info.align_of(|meta| std::mem::align_of::<T>());
        info.drop(|meta| {
            let ptr = *(meta as *const (*const T, usize);
            let mut data_ptr = ptr.0;
            for i in 0..ptr.1 {
                std::ptr::drop_in_place(data_ptr);
                data_ptr = data_ptr.offset(1);
            }
        });
        info.self_ptr(|ptr| ptr);
    }
    info
}
```

## C++ like vtables

Most of the boilerplate is the same as with regular vtables.

```rust

pub const fn cpp_unsize<
    T,
    const IMPL: &'static std::vtable::TraitDescription,
    const owned: bool,
>(
    ptr: *const T,
) -> *const (VTable<{num_methods::<IMPL>()}>, T)
where {
    assert!(owned, "cannot unsize borrowed object for C++ like trait")
},
{
    let size = std::mem::size_of::<T>();
    let vtable_size = std::mem::size_of::<Vtable<{num_methods::<IMPL>()}>>();
    let layout = Layout::from_size_align(size + vtable_size, std::mem::align_of::<T>()).unwrap();
    let new_ptr = std::alloc::alloc(layout);
    // Move the value to the new allocation
    std::ptr::copy_nonoverlapping(ptr, new_ptr.offset(vtable_size), 1);
    std::alloc::dealloc(ptr);

    // Copy the vtable into the shared allocation.
    // Note that we are reusing the same vtable generation as in
    // the regular Rust case.
    std::ptr::write(new_ptr, default_vtable::<T, IMPL>());
    new_ptr
}

pub const fn cpp<
    const IMPL: &'static std::vtable::TraitDescription,
>() -> std::vtable::DstInfo {
    let mut info = DstInfo::new();
    unsafe {
        info.method_id_to_fn_ptr(|idx, parents, meta| unsafe {
            let meta = *(meta as *const *const Vtable<{num_methods::<IMPL>()}>);
            // The rest of the function body is the same as with regular
            // vtables.
        });
        info.size_of(|meta| unsafe {
            let meta = *(meta as *const *const Vtable<{num_methods::<IMPL>()}>);
            (*meta).size
        });
        info.align_of(|meta| unsafe {
            let meta = *(meta as *const *const Vtable<{num_methods::<IMPL>()}>);
            (*meta).align
        });
        info.drop(|meta| unsafe {
            let meta = *(meta as *const *const Vtable<{num_methods::<IMPL>()}>);
            (*meta).drop
        });
        info.self_ptr(|meta| unsafe {
            let ptr = *(meta as *const *const (Vtable<{num_methods::<IMPL>()}>, ()));
            &raw const (*ptr).1;
        });
    }
    info
}
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The two types `TraitDescription` and `DstInfo` are `#[nonexhaustive]` in order to allow arbitrary extension in the future.
Instances of the `TraitDescription` struct are created by rustc, basically replacing today's vtable generation, and then outsourcing the actual vtable generation to the const evaluator.

When unsizing, the `const fn` specified
via `unsafe_custom_unsize` is invoked. The only reason that function is
`const fn` is to restrict what kind of things you can do in there. We
can lift this restriction in the future.

For all other operations, the `unsafe_custom_vtable` function is invoked.
This one must be `const fn`, as it is evaluated at compile-time and the
compiler then inspects the resulting `DstInfo` at compile-time.

When obtaining function pointers from vtables, instead of computing an offset, the `method_id_to_fn_ptr` function is invoked at runtime and computes a function pointer after being given a method index, the indices of all parents and a pointer to a metadata field. Through the use of MIR optimizations (e.g. inlining), the final LLVM assembly is tuned to be exactly the same as today.

These types' declarations are provided below:

```rust
#[nonexhaustive]
struct TraitMethod {
    fn_ptr: *const (),
    // may get more fields like `name` or even `body` (the latter being a string of the body to be used with `syn`).
}

#[nonexhaustive]
struct TraitDescription {
    pub methods: &'static [TraitMethod],
    pub parent: &'static TraitDescription,
}

#[nonexhaustive]
struct DstInfo {
    method_id_to_fn_ptr: fn(usize, &'static [usize], *const ()) -> *const (),
    size_of: fn(*const()) -> usize,
    align_of: fn(*const()) -> usize,
    drop: *const (),
    self_ptr: fn(*const()) -> *const (),
}

impl DstInfo {
    const fn new() -> Self {
        Self {
            method_id_to_fn_ptr: None,
            size_of: None,
            align_of: None,
            drop: None,
            self_ptr: None,
        }
    }
    /// The given function returns a function pointer to the method that
    /// is being requested.
    /// * The first argument is the method index,
    /// * the second argument is a list of indices used to traverse the
    ///   super-trait tree to find the trait whose method is being invoked, and
    /// * the third argument is a pointer to the wide pointer (so in case of trait objects, usually it would be `*const (*const T, &'static Vtable)`).
    ///   This indirection is necessary, because we don't know the size of the wide pointer.
    unsafe fn method_id_to_fn_ptr(f: fn(usize, &'static [usize], *const ()) -> *const ()) {
        self.method_id_to_fn_ptr = Some(f);
    }
    /// Set the function that extracts the dynamic size
    unsafe fn size_of(f: fn(*const ()) -> usize) {
        self.size_of = Some(f);
    }
    /// Set the function that extracts the dynamic alignment
    unsafe fn align_of(f: fn(*const ()) -> usize) {
        self.align_of = Some(f);
    }
    /// Set the function that extracts the drop code.
    unsafe fn drop<T>(f: fn(*const ()) -> Option<fn(*mut T)>) {
        self.drop = Some(f);
    }
    /// Set the function that extracts the `&self` pointer
    /// from the wide pointer
    /// for calling trait methods. This needs a method as
    /// wide pointer layouts may place their `self` pointer
    /// anywhere they desire.
    unsafe fn self_ptr(f: fn(*const ()) -> *const ()) {
        self.self_ptr = Some(f);
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
- This scheme could support downcasting `dyn A` to `dyn B` if `trait A: B` if we make `T: ?Sized` (`T` is the `impl` block type). But that will not allow the sized use-cases anymore (since `size_of::<T>` will fail). If we have something like `MaybeSized` that has `size_of` and `align_of` methods returning `Option`, then maybe we could do this.
* Need to be generic over the allocator, too, so that reallocs are actually sound.
* how does this interact with `Pin`?

# Future possibilities
[future-possibilities]: #future-possibilities

* Add a scheme that allows super traits to have different vtable generators and permit a vtable generator to process them. So `trait A: B + C`, where `B` and `C` have different vtable generators and `A` unites them in some manner. This requires the information about the vtable generators to be part of the `TraitDescription` type. We can likely even put a function pointer to the vtable generator into the `TraitDescription`.
* Add a scheme allowing `dyn A + B`. I have no idea how, but maybe we just need to add a method to `DstInfo` that allows chaining vtable generators. So we first generate `dyn A`, then out of that we generate `dyn A + B`
* We can change string slice (`str`) types to be backed by a `trait StrSlice` which uses this scheme to generate just a single `usize` for the metadata (see also the `[T]` demo).
* We can handle traits with parents which are created by a different metadata generator function, we just need to figure out how to communicate this to such a function so it can special case this situation.
* We can give the vtable pointer of the super traits to the constructor function, so it doesn't have to recompute anything and just grab the info off there. Basically `TraitDescription::parent` would not be a pointer to the parent, but a struct which contains at least the pointer to the parent and the pointer to the `DstInfo`
* Totally off-topic, but a similar scheme (via const eval) can be used to procedurally generate type declarations.
* We can likely access associated consts and types of the trait directly without causing cycle errors, this should be investigated
* This scheme is forward compatible to adding associated fields later.
* If/once we expose the `Unsize` traits on stable, we could consider adding a method to the `Unsize` trait that performs the conversion. This way more complex unsizings like `String` -> `str` could be performed without going through the `Deref` trait which does the conversion. This would allow us to essentially write `impl str for String` if we make `str` a `trait`.