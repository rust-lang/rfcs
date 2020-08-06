- Feature Name: `procedural-vtables`
- Start Date: 2020-08-01
- RFC PR: [rust-lang/rfcs#2967](https://github.com/rust-lang/rfcs/pull/2967)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Building a wide pointer from a concrete pointer (and thus vtable generation)
can be controlled by choosing a custom wide pointer type for the trait.
The custom wide pointer must implement a trait that generates said wide pointer
by taking a concrete pointer and a generic description of a trait impl.
By default, if no vtable generator function is specified for a specific trait,
the unspecified scheme used today keeps getting used.

# Motivation
[motivation]: #motivation

The only way we're going to satisfy all users' use cases is by allowing users
complete freedom in how their wide pointers' metadata is built.
Instead of hardcoding certain vtable layouts in the language
(https://github.com/rust-lang/rfcs/pull/2955) we can give users the capability
to invent their own wide pointers (and thus custom dynamically sized types).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In order to mark a trait as using a custom vtable layout, you apply an attribute to
your trait declaration.

```rust
#[custom_wide_pointer = "MyWidePointer"]
trait MyTrait {}

```

`MyWidePointer` is a type which implements the `CustomUnsized` trait and optionally the `CustomUnsize` trait.
`MyWidePointer` is the type that is going to be used for wide pointers to the given trait, so

* `&dyn MyTrait`
* `Box<dyn MyTrait>`
* `Arc<dyn MyTrait>`

and any other container types that can use wide pointers. Normally when unsizing from a concrete
pointer like `&MyStruct` to `&dyn MyTrait` a wide pointer (that essentially is
`(&MyStruct, &'static Vtable)`) is produced. This is currently done via compiler magic, but
further down in this section you can see how it theoretically could be done in user code.
Likely it will stay compiler magic out of compile-time performance reasons.

All `impl`s of `MyTrait` will now use `MyWidePointer` for generating the wide pointer.
The `TraitDescription` struct describes the metadata like the list of methods and a tree
of super traits.

You are actually generating the wide pointer, not a description of it.
Since your `const impl`'s `from` function is being interpreted in the target's environment, all target specific information will match up.
Now, you need some information about the `impl` in order to generate your metadata (and your vtable).
You get this information partially from the type directly (the `T` parameter),
and all the `impl` block specific information is encoded in `TraitDescription`, which you get as a const generic parameter.

As an example, consider the function which is what normally generates your metadata. Note that if you have a generic trait,
the `CustomUnsize` and `CustomUnsized` impls need additional generic parameters, one for each parameter of the trait.
See the `[T]` demo further down for an example.

```rust
#[repr(C)]
struct Pointer<const IMPL: &'static std::vtable::TraitDescription> {
    ptr: *const (),
    vtable: &'static VTable<{num_methods::<IMPL>()}>,
}

/// If the `owned` flag is `true`, this is an owned conversion like
/// in `Box<T> as Box<dyn Trait>`. This distinction is important, as
/// unsizing that creates a vtable in the same allocation as the object
/// (like C++ does), cannot work on non-owned conversions. You can't just
/// move away the owned object. The flag allows you to forbid such
/// unsizings by triggering a compile-time `panic` with an explanation
/// for the user.
unsafe impl<const IMPL: &'static std::vtable::TraitDescription> const CustomUnsize for Pointer<IMPL> {
    fn from<T, const owned: bool>(ptr: *const T) -> Self {
        // We generate the metadata and put a pointer to the metadata into 
        // the field. This looks like it's passing a reference to a temporary
        // value, but this uses promotion
        // (https://doc.rust-lang.org/stable/reference/destructors.html?highlight=promotion#constant-promotion),
        // so the value lives long enough.
        Pointer {
            ptr,
            vtable: &default_vtable::<T, IMPL>(),
        }
    }
}

/// DISCLAIMER: this uses a `Vtable` struct which is just a part of the
/// default trait objects. Your own trait objects can use any metadata and
/// thus "vtable" layout that they want.
unsafe impl<const IMPL: &'static std::vtable::TraitDescription> CustomUnsized for Pointer<IMPL> {
    fn method_id_to_fn_ptr(
        self,
        mut idx: usize,
        parents: &[usize],
    ) -> *const () {
        let mut table = IMPL;
        for parent in parents {
            // we don't support multi-parents yet
            assert_eq!(parent, 0);
            idx += table.methods.len();
            // Never panics, there are always fewer or equal number of
            // parents given as the argument as there are in reality.
            table = table.parent.unwrap();
        }
        self.vtable.methods[idx]
    }
    fn size_of(self) -> usize {
        self.vtable.size
    }
    fn align_of(self) -> usize {
        self.vtable.align
    }
    fn drop(self) {
        unsafe {
            let drop = self.vtable.drop;
            drop(&raw mut self.ptr)
        }
    }
    fn self_ptr(self) -> *const () {
        self.ptr
    }
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
                // cast to `*const ()` in order to support null pointers.
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

Now, if you want to implement a fancier vtable, this RFC enables you to do that.

## Null terminated strings (std::ffi::CStr)

This is how I see all extern types being handled.
There can be no impls of `CStr` for any type, because the `Unsize`
trait impl is missing. See the future extension section at the end of this RFC for
ideas that could support `CString` -> `CStr` unsizing by allowing `CString` to implement
`CStr` instead of having a `Deref` impl that converts.

```rust
#[custom_wide_pointer = "CStrPtr"]
pub trait CStr {}


#[repr(C)]
struct CStrPtr {
    ptr: *const u8,
}

impl<
    T,
    const IMPL: &'static std::vtable::TraitDescription,
> CustomUnsized for CStrPtr {
    fn method_id_to_fn_ptr(
        self,
        method_index: usize,
        super_tree_path: usize,
    ) -> *const () {
        panic!("CStr has no trait methods, it's all inherent methods acting on the pointer")
    }
    fn size_of(self) -> usize {
        unsafe { strlen(self.ptr) }
    }
    fn align_of(self) -> usize {
        1
    }
    fn drop(self) {
        // Nothing to drop (just `u8`s) and we are not in charge of dealloc
    }
    fn self_ptr(self) -> *const () {
        self.ptr
    }
}
```

## `[T]` as sugar for a `Slice` trait

We could remove `[T]` (and even `str`) from the language and just make it desugar to
a `std::slice::Slice` (or `StrSlice`) trait.

```rust
#[custom_wide_pointer = "SlicePtr"]
pub trait Slice<T> {}

#[repr(C)]
struct SlicePtr<T> {
    ptr: *const T,
    len: usize,
}

impl<
    T,
    const IMPL: &'static std::vtable::TraitDescription,
> CustomUnsized for Slice<T> {
    fn method_id_to_fn_ptr(
        self,
        method_index: usize,
        super_tree_path: usize,
    ) -> *const () {
        panic!("CStr has no trait methods, it's all inherent methods acting on the pointer")
    }
    fn size_of(self) -> usize {
        self.len
    }
    fn align_of(self) -> usize {
        st::mem::align_of::<T>()
    }
    fn drop(self) {
        let mut data_ptr = self.ptr;
        for i in 0..self.len {
            std::ptr::drop_in_place(data_ptr);
            data_ptr = data_ptr.offset(1);
        }
    }
    fn self_ptr(self) -> *const () {
        // Not quite sure what to do here, need to think on this.
        // Technically I want to return `Self`, but other schemes do not.
        self.ptr
    }
}
```

## C++ like vtables

Most of the boilerplate is the same as with regular vtables.

```rust
#[repr(C)]
struct Pointer<const IMPL: &'static std::vtable::TraitDescription> {
    ptr: *const (VTable<{num_methods::<IMPL>()}>, ())
}

unsafe impl<const IMPL: &'static std::vtable::TraitDescription> const CustomUnsize for Pointer<IMPL> {
    fn from<T, const owned: bool>(ptr: *const T) -> Self {
        unsafe {
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
            Self { ptr: new_ptr }
        }
    }
}


unsafe impl<const IMPL: &'static std::vtable::TraitDescription> CustomUnsized for Pointer<IMPL> {
    fn method_id_to_fn_ptr(
        self,
        method_index: usize,
        super_tree_path: usize,
    ) -> *const () {
        let meta = unsafe { &*self.ptr };
        // The rest of the function body is the same as with regular
        // vtables.
    }
    fn size_of(self) -> usize {
        unsafe {
            (*self.ptr).0.size
        }
    }
    fn align_of(self) -> usize {
        unsafe {
            (*self.ptr).0.align
        }
    }
    fn drop(self) {
        unsafe {
            let drop = (*self.ptr).0.drop;
            drop(&raw mut (*self.ptr).1)
        }
    }
    fn self_ptr(self) -> *const () {
        unsafe {
            &raw const (*self.ptr).1
        }
    }
}
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The type `TraitDescription` is `#[nonexhaustive]` in order to allow arbitrary extension in the future.
Instances of the `TraitDescription` struct are created by rustc, basically replacing today's vtable generation, and then outsourcing the actual vtable generation to the const evaluator.

When unsizing, the `const fn` specified
via `<WidePtrType as CustomUnsize>::from` is invoked.
The `TraitDescription` parameter of `CustomUnsize` contains function pointers to the
concrete functions of the `impl`. The `TraitDescription` parameter of `CustomUnsized`
contains function pointers to default impls (if any), or mostly just null pointers.
The same datastructure is shared here to make implementations easier and because they
contain mostly the same data anyway.
`WidePtrType` refers to the argument of the
`#[custom_wide_ptr = "WidePtrType"]` attribute. The only reason that trait must be
`impl const WidePtrType` is to restrict what kind of things you can do in there, it's not
strictly necessary.

For all other operations, the methods on `<WidePtrType as CustomUnsized>` is invoked.

When obtaining function pointers from vtables, instead of computing an offset, the `method_id_to_fn_ptr` function is invoked at runtime and computes a function pointer after being given a method index, the indices of all parents, and a pointer to a metadata field.
Through the use of MIR optimizations (e.g. inlining), the final LLVM assembly is tuned to be exactly the same as today.

These types' and trait's declarations are provided below:

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

unsafe trait CustomUnsized: Copy {
    /// Returns a function pointer to the method that
    /// is being requested.
    /// * The first argument is the method index,
    /// * the second argument is a list of indices used to traverse the
    ///   super-trait tree to find the trait whose method is being invoked, and
    /// * the third argument is a pointer to the wide pointer (so in case of trait objects, usually it would be `*const (*const T, &'static Vtable)`).
    ///   This indirection is necessary, because we don't know the size of the wide pointer.
    fn method_id_to_fn_ptr(
        self,
        method_index: usize,
        super_tree_path: usize,
    ) -> *const ();
    fn size_of(self) -> usize;
    fn align_of(self) -> usize;
    fn drop(self);
    /// Extracts the `&self` pointer
    /// from the wide pointer
    /// for calling trait methods. This needs a method as
    /// wide pointer layouts may place their `self` pointer
    /// anywhere they desire.
    fn self_ptr(self) -> *const ();
}

unsafe trait CustomUnsize: CustomUnsized {
    fn from<
        T
        const owned: bool,
    >(t: *const T) -> Self;
}
```

# Drawbacks
[drawbacks]: #drawbacks

* This may be a serious case of overengineering. We're basically taking vtables out of the language and making dynamic dispatch on trait objects a user definable thing.
* This may slow down compilation, likely entirely preventable by keeping a special case in the compiler for regular trait objects.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is a frequently requested feature and as a side effect obsoletes `extern type`s which have all kinds of problems (like the inability to invoke `size_of_val` on pointers to them).

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

This RFC differs from all the other RFCs in that it provides a procedural way to generate vtables,
thus also permitting arbitrary user-defined compile-time conditions by aborting via `panic!`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Do we want this kind of flexibility? With power comes responsibility...
- I believe we can do multiple super traits, including downcasts with this scheme and no additional extensions, but I need to make sure that's true.
- This scheme could support downcasting `dyn A` to `dyn B` if `trait A: B` if we make `T: ?Sized` (`T` is the `impl` block type). But that will not allow the sized use-cases anymore (since `size_of::<T>` will fail). If we have something like `MaybeSized` that has `size_of` and `align_of` methods returning `Option`, then maybe we could do this.
* Need to be generic over the allocator, too, so that reallocs are actually sound.
* how does this RFC (especially the `owned` flag) interact with `Pin`?

# Future possibilities
[future-possibilities]: #future-possibilities

* Add a scheme that allows upcasting to super traits that have different vtable generators.
  So `trait A: B + C`, where `B` and `C` have different vtable generators and `A` unites them in some manner.
  This requires the information about the vtable generators to be part of the `TraitDescription` type.
  We can likely even put a function pointer to the vtable generator into the `TraitDescription`.
* Add a scheme allowing `dyn A + B`. I have no idea how, but maybe we just need to add a method to `CustomUnsize`
  that allows chaining vtable generators. So we first generate `dyn A`, then out of that we generate `dyn A + B`
* We can change string slice (`str`) types to be backed by a `trait StrSlice` which uses this scheme
  to generate just a single `usize` for the metadata (see also the `[T]` demo).
* We can handle traits with parents which are created by a different metadata generator function,
  we just need to figure out how to communicate this to such a function so it can special case this situation.
* We can give the vtable pointer of the super traits to the constructor function,
  so it doesn't have to recompute anything and just grab the info off there.
  Basically `TraitDescription::parent` would not be a pointer to the parent,
  but a struct which contains at least the pointer to the parent and the pointer to the `CustomUnsize::from` function.
* Totally off-topic, but a similar scheme (via const eval) can be used to procedurally generate type declarations.
* We can likely access associated consts and types of the trait directly without causing cycle errors, this should be investigated
* This scheme is forward compatible to adding associated fields later.
* If/once we expose the `Unsize` (do not confuse with `CustomUnsize`) traits on stable, we could consider adding a method to the `Unsize` trait that performs the conversion. This way more complex unsizings like `String` -> `str` could be performed without going through the `Deref` trait which does the conversion. This would allow us to essentially write `impl str for String` if we make `str` a `trait`. We could also move the `CustomUnsize` trait's `from` method's `T` parameter onto the trait, thus allowing users to manually `impl CustomUnsize<CString> for CStrPtr`.