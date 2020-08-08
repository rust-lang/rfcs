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

In order to mark a trait (`MyTrait`) as using a custom vtable layout, you implement the `CustomUnsized` trait for `dyn MyTrait`.

```rust
trait MyTrait: SomeSuperTrait {
    fn some_fn(&mut self, x: u32) -> i32;
}

impl CustomUnsized for dyn MyTrait {
    type WidePointer = MyWidePointer;
    // details later in this RFC
}

```

`MyWidePointer` is the backing type that is going to be used for wide pointers to the given trait, so

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
Since your `impl`'s `from` function is being interpreted in the target's environment, all target specific information will match up.
Now, you need some information about the `impl` in order to generate your metadata (and your vtable).
You get this information directly from the type (the `T` parameter) by adding trait bounds on it.

As an example, consider the function which is what normally generates your metadata.

```rust
#[repr(C)]
struct Pointer<T> {
    ptr: *mut T,
    vtable: &'static VTable<T>,
}

/// If the `owned` flag is `true`, this is an owned conversion like
/// in `Box<T> as Box<dyn Trait>`. This distinction is important, as
/// unsizing that creates a vtable in the same allocation as the object
/// (like C++ does), cannot work on non-owned conversions. You can't just
/// move away the owned object. The flag allows you to forbid such
/// unsizings by triggering a compile-time `panic` with an explanation
/// for the user.
unsafe impl<T: MyTrait> const CustomUnsize<Pointer> for T {
    fn unsize<const owned: bool>(ptr: *mut T) -> Pointer<T> {
        // We generate the metadata and put a pointer to the metadata into 
        // the field. This looks like it's passing a reference to a temporary
        // value, but this uses promotion
        // (https://doc.rust-lang.org/stable/reference/destructors.html?highlight=promotion#constant-promotion),
        // so the value lives long enough.
        Pointer {
            ptr,
            vtable: &default_vtable::<T>(),
        }
    }
}

/// DISCLAIMER: this uses a `Vtable` struct which is just a part of the
/// default trait objects. Your own trait objects can use any metadata and
/// thus "vtable" layout that they want.
unsafe impl Unsized for dyn MyTrait {
    fn size_of(self) -> usize {
        self.vtable.size
    }
    fn align_of(self) -> usize {
        self.vtable.align
    }
}

impl Drop for dyn MyTrait {
    fn drop(&mut self) {
        // using a dummy concrete type for `Pointer`
        let ptr = transmute::<&mut dyn Trait, Pointer<()>>(self);
        unsafe {
            let drop = ptr.vtable.drop;
            drop(&raw mut ptr.ptr)
        }
    }
}

const fn default_vtable<T: MyTrait>() -> VTable<T> {
    // `VTable` is a `#[repr(C)]` type with fields at the appropriate
    // places.
    VTable {
        size: std::mem::size_of::<T>(),
        align: std::mem::size_of::<T>(),
        drop: <T as Drop>::drop,
        some_fn: fn (&mut T, u32) -> i32,
    }
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
pub trait CStr {}


#[repr(C)]
struct CStrPtr {
    ptr: *mut u8,
}

impl CustomUnsized<CStrPtr> for dyn CStr {
    fn size_of(self) -> usize {
        unsafe { strlen(self.ptr) }
    }
    fn align_of(self) -> usize {
        1
    }
}
```

## `[T]` as sugar for a `Slice` trait

We could remove `[T]` (and even `str`) from the language and just make it desugar to
a `std::slice::Slice` (or `StrSlice`) trait.

```rust
pub trait Slice<T> {}

#[repr(C)]
struct SlicePtr<T> {
    ptr: *mut T,
    len: usize,
}

// This impl must be in the `vec` module, to give it access to the `vec`
// internals instead of going through `&mut Vec<T>` or `&Vec<T>`.
impl<T> CustomUnsize<SlicePtr<T>> for Vec<T> {
    fn unsize<const owned: bool>(ptr: *mut Vec<T>) -> SlicePtr<T> {
        SlicePtr {
            ptr: vec.data,
            len: vec.len,
        }
    }
}

impl<T> CustomUnsized for dyn Slice<T> {
    fn size_of(self) -> usize {
        self.len
    }
    fn align_of(self) -> usize {
        st::mem::align_of::<T>()
    }
}

impl Drop for dyn Slice<T> {
    fn drop(&mut self) {
        let wide_ptr = transmute::<&mut dyn Slice<T>, SlicePtr<T>>(self);
        let mut data_ptr = wide_ptr.ptr;
        for i in 0..wide_ptr.len {
            std::ptr::drop_in_place(data_ptr);
            data_ptr = data_ptr.offset(1);
        }
    }
}
```

## C++ like vtables

Most of the boilerplate is the same as with regular vtables.

```rust
#[repr(C)]
struct Pointer<T> {
    ptr: *mut (VTable, T),
}

unsafe impl<T: MyTrait> const CustomUnsize<Pointer<T>> for T {
    fn unsize<const owned: bool>(ptr: *mut T) -> Pointer<T> {
        unsafe {
            let new = Box::new((default_vtable::<T>(), std::ptr::read(ptr)));
            std::alloc::dealloc(ptr);

            Pointer { ptr: Box::into_ptr(new) }
        }
    }
}


unsafe impl CustomUnsized for dyn MyTrait {
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
}
```


## zero sized references to MMIO

Instead of having one type per MMIO register bank, we could have one
trait per bank and use a zero sized wide pointer format.

```rust
struct NoPointer;

trait MyRegisterBank {
    fn flip_important_bit(&mut self);
}

unsafe impl CustomUnsized for dyn MyRegisterBank {
    fn size_of(self) -> usize {
        4
    }
    fn align_of(self) -> usize {
        4
    }
}

impl MyRegisterBank for dyn MyRegisterBank {
    fn flip_important_bit(&mut self) {
        std::mem::volatile_write::<bool>(42, !std::mem::volatile_read::<bool>(42))
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When unsizing, the `<dyn MyTrait as CustomUnsize>::unsize` is invoked.
The only reason that trait must be
`impl const CustomUnsize` is to restrict what kind of things you can do in there, it's not
strictly necessary. This restriction may be lifted in the future.

For all other operations, the methods on `<dyn MyTrait as CustomUnsized>` is invoked.

When obtaining function pointers from vtables, instead of computing an offset, the `MyTrait for dyn MyTrait` impl's
methods are invoked, allowing users to insert their own logic for obtaining the runtime function pointer.
Through the use of MIR optimizations (e.g. inlining), the final LLVM assembly is tuned to be exactly the same as today.
The above statement contains significant hand-waving, but I propose we block the stabilization of this RFC on the
relevant optimizations existing, which will then allow users to reproduce the performance of the built-in unsizing.

These types' and trait's declarations are provided below:

```rust
unsafe trait CustomUnsized {
    fn size_of(self) -> usize;
    fn align_of(self) -> usize;
}

unsafe trait CustomUnsize<WidePtrType> where WidePtrType: CustomUnsized {
    fn unsize<const owned: bool>(t: *mut Self) -> WidePtrType;
}
```

# Drawbacks
[drawbacks]: #drawbacks

* This may be a serious case of overengineering. We're basically taking vtables out of the language and making dynamic dispatch on trait objects a user definable thing.
* This may slow down compilation, likely entirely preventable by keeping a special case in the compiler for regular trait objects.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is a frequently requested feature and as a side effect obsoletes `extern type`s which have all kinds of problems (like the inability to invoke `size_of_val` on pointers to them).

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

This RFC differs from all the other RFCs in that it focusses on a procedural way to generate vtables,
thus also permitting arbitrary user-defined compile-time conditions by aborting via `panic!`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Do we want this kind of flexibility? With power comes responsibility...
- I believe we can do multiple super traits, including downcasts with this scheme and no additional extensions, but I need to make sure that's true.
- This scheme support downcasting `dyn A` to `dyn B` if `trait A: B` if you `impl CustomUnsize<TraitAWidePtrType> for dyn B`
* this scheme allows `dyn A + B`. By implemting `CustomUnsized` for `dyn A + B`
* Need to be generic over the allocator, too, so that reallocs are actually sound.
* how does this RFC (especially the `owned` flag) interact with `Pin`?

# Future possibilities
[future-possibilities]: #future-possibilities

* We can change string slice (`str`) types to be backed by a `trait StrSlice` which uses this scheme
  to generate just a single `usize` for the metadata (see also the `[T]` demo).
* This scheme is forward compatible to adding associated fields later, but it is a breaking change to add such fields to an existing trait.