- Feature Name: `trait_stable_vtable`
- Start Date: 2020-07-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

To allow for traits to have a stable fat-pointer layout and a stable vtable layout to allow FFI Compatibility, 
 a new `#[stable_vtable]` attribute is provided to opt-in to the layout as described within this rfc. 

# Motivation
[motivation]: #motivation

Presently in rust, FFI is strictly based on C concepts. Free functions, pointers, nothing fancy. However, sometimes, it is desired to offer a higher level FFI.
In particular, the ability to define a trait that encapsilates runtime behaviour, then obtain a pointer of some kind to a foreign implementation of the trait. 
This implementation could then cross FFI bounderies, or even DSO Bounderies. A use case is in the development of plugin driven applications, where a higher level plugin api is desired.
DSO Bounderies are also as important as different languages, as even when compiled in rust, they may be compiled with different rust versions, or even different compilers. 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A declared trait may have the `#[stable_vtable]` attribute, to allow its use in FFI and across dynamicly linked libraries/shared objects.  

```rust
#[stable_vtable]
pub trait StableVtable{
    pub fn example_fn(&self)->();
    pub fn example_fn_returning_i32(&self)->i32;
}
```

This declares a trait with a stable vtable layout, safe for use across FFI. 

A `stable_vtable` trait may have supertraits, provided all supertraits are either also `stable_vtable` traits, or `auto` traits (IE. Send, Sync). 

```
// OK
#[stable_vtable]
pub trait StableWithSuperTraits: StableVTable{
    pub fn example_taking_f32(&self,i32)->();
};

// OK
#[stable_vtable]
pub trait StableRequiresSync : Sync{
    pub fn do_thread_thing(&self)->();
};


pub trait NotStable{
};

// Error, Traits with stable virtual table cannot have non-stable supertraits
#[stable_vtable]
pub trait BadStable: NotStable{

};
```

By default, all required and provided functions in a `stable_vtable` trait are `extern"C"`. This is because the, otherwise default, rust calling convention is not stable, 
 and unsuited for any of the purposes which this rfc is intended to fufil. Exceptions to this rule are functions that are not available on the trait object (for example, because they require `Self: Sized`). 
A required or provided function may have an explicit abi specification, which applies over the default `extern"C"`.

```rust

#[stable_vtable]
pub trait Stable{
   pub fn default_abi(&self) ->();
   pub fn default_self(self)->() where Self: Sized;
};

#[stable_vtable]
pub trait StableExternC{
   pub extern"C" fn c_abi(&self)->();
   pub extern"C" fn c_self(self)->() where Self: Sized;
};

```
Both `default_abi` and `c_abi` are `extern"C"`. However, while `c_self` is still `extern"C"`, `default_self` is `extern"Rust"` (since it requires `Self: Sized` and cannot be called from a trait object).


All traits declared `#[stable_vtable]` must be *Object-safe*. `#[stable_vtable]` does not have any affect on regular implementors of the trait (asside from the above default abi), and therefore is useless on traits that can't be used as a trait object. 

It is a minor change to add `#[stable_vtable]` to a trait, but a major change to remove it once added, reorder the declared functions, add or remove new required *or* provided functions (even for Sealed traits), or change the signature of functions in such a trait (even to a source-compatible signature).  


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

_Note - For additional clarity, C++ style standardeese is adopted by this specification so as to make clear where absolute requirements are defined, and where looser implementation requirements are suggested. - End Note_

In this section, the term shall is to be interpreted as an absolute requirement of the program or implementation. The term implementation shall mean `rustc` or any alternative implementation of rust which adopts the features described by this rfc.  

A trait declared with the `#[stable_vtable]` shall have the following properties:
* A *stable-layout-pointer* to a trait object for which the primary (non-auto) trait is `#[stable_vtable]` shall be layed out as though the same as the following *exposition-only* repr(C) struct:
```rust
#[repr(C)]
struct TraitObject{
    data: *mut (),
    vtable: *const ()
}
```
* The vtable pointed to by the above vtable member of such a trait object shall be layed out as though the same as the following *expositon-only* struct:
```rust
#[repr(C)]
struct VTable{
    size: usize,
    align: usize,
    drop_in_place: Option<unsafe extern"C" fn(*mut ())->()>,
    dealloc:Option<unsafe extern"C" fn(*mut ())->()>,
    virtual_fns: [unsafe extern "C" fn(*mut ())->()]
}
```
* The order of the virtual functions in the vtable shall be the declaration order of the functions in the trait. 
There shall be exactly one virtual function entry for each function which can be called on a trait object. For example, the StableVtable trait above will have this VTable
```rust
#[repr(C)]
struct VTable_StableVtable{
    size: usize,
    align: usize,
    drop_in_place: Option<unsafe extern"C" fn(*mut ())->()>,
    dealloc: Option<unsafe extern"C" fn(*mut ())->()>,
    _vdispatch_example_fn: unsafe extern"C" fn(*const ())->(),
    _vdispatch_example_fn_returning_i32: unsafe extern"C" fn(*const ())->i32
}
```

(_Note - in the above, none such structs are actually defined by the rust language or standard library, and are provided for *exposition-only*. - End Note_)

The fields of the vtable shall be initialized as follows:

* `size` and `align` entries shall be initialized to the size and ABI required alignment of the implementing type. The size entry shall be a multiple of `align`. 
* The `drop_in_place` entry shall be initialized to a function which performs the drop operation of the implementing type. If the drop operation is a no-op,
 the entry may be initialized to a null pointer (`None`) _Note - It is unspecified if types with trivial (no-op) destruction have the entry initialized to None,
 or to a function that performs no operation - End Note_ 
* The `dealloc` entry shall be initialized to a function which is suitable for deallocating the pointer if it was produced by the in-use global-allocator, (including potentially the intrinsic global-allocator provided by the `std` library). If no global-allocator is available, the entry shall be initialized to a null pointer, or a pointer to a function which performs no operation.
* Each `virtual_fn` shall be initialized to the appropriate function provided by the implementation. If the trait has any supertraits, the `virutal_fn` entries from those supertraits appear first, from Left to Right. Trait functions which are not valid to call on a reciever with a trait-object type are omitted from the vtable. _Note - In particular, entries which require Self: Sized are omitted - End Note_

The `#[stable_vtable]` attribute shall not be applied to a trait which is not object safe, or which has any supertraits that are not `#[stable_vtable]` or `auto` traits.


The following types shall be *stable-layout-pointers*:
* Both mutable and non-mutable references to trait objects for which the primary trait is declared `#[stable_vtable]` 
* A NonNull pointer to such a trait object
* A `Box<T>` of such a trait object, with no specified allocator, the Global allocator specified, or a 1-ZST allocator.
* A repr(transparent) type of any of the above (except where that `repr(transparent)` type is `core::mem::MaybeUninit`, `core::mem::ManuallyDrop`, or `core::cell::UnsafeCell`) _Note - It is unspecified whether `ManuallyDrop<T>` of such a type above, when inside an `Option` is considered to be a stable-layout-pointer - End Note_
* An `Option<T>` of any of the above
* A raw pointer to a trait object as described above
* A `MaybeUninit`, `ManuallyDrop`, or `UnsafeCell` of any of these types, or a repr(transparent) wrapper arround such a type

A call to the function `core::mem::size_of_val` when applied to a reference to which is a *stable-layout-pointer* shall return the value of the `size` entry in the VTable.
A call to the function `core::mem::align_of_val` when applied to such a reference shall return the value of the `align` entry.

The behaviour is undefined if any of the following is violated for any *stable-layout-pointer*. The implementation shall not cause any of these constraints to be violated:
* `size` shall be a multiple of `align`.
* `align` shall be a power of two.

The behaviour is undefined if any of the following is violated for references and instantiations of the type `Box` that are *stable-layout-pointers*. The implementation shall not cause any of these contrainsts to be violated:
* `data` shall be valid for reading for a number of bytes which is at least `size`
* `data` shall be have at least `align` alignment

# Drawbacks
[drawbacks]: #drawbacks

Implementing this rfc would require a specific layout for trait objects in a subset of all traits.
 It is possible that crate authors may add `#[stable_vtable]` to even traits not necessarily intended to be used for FFI, which would significantly impact the ability for layout optimizations to be applied. 

The stablization of trait object layout, even for a subset of traits, may impact future implementations which allow for dynamically sized types which require multiple metadata (such a structures containing multiple slices. 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The original proposal was to overload `repr(C)`. However in discussions on that thread seems to point me in the direction that `repr(C)` has been overloaded enough. 
At the same time, an `abi` argument was proposed, to allow customization of the vtable abi used. This was not chosen here for simplicitly. As this gets support, 
 and custom abis/reprs are also added, it could follow that this or the abi rfc be updated appropriately. 

A language level alternative would be to use the `Itanium`-like vtable. The Itanium C++ ABI is a widely adopted ABI Specification for implementations of the C++ Standard. 
However, using it directly would likely be a non-starter in Rust, as the vtables are inline with the struct, rather than separated from the type. 
Additionally, something similar to COM could be used as the stablized vtable layout, which would require swapping the vtable and data pointers in the Object declaration. However I have not yet researched the use of the first four entries in a COM struct Pointer table, in relation to the pointers entered here. 

# Prior art
[prior-art]: #prior-art

As mentioned, both the C++ Itanium ABI and COM are examples of a "stablized" virtual dispatch. Both specify the layout of virtual dispatch tables, and how they interact with virtual calls. COM-like structs have specifically been used for cross-language "virtual" dispatch, and have been used in a variety of applications, including the Java Native Interface.

The [abi\_stable](https://crates.io/crates/abi_stable) crate provides a semi-stablized vtable for Rust-Rust ABI Compatibility. The VTable used by it is known to be incompatible with this RFC, however it serves as an example of the usefulness of allowing cross-module compatibility in Rust code.  

Beyond prior art, some in-progress work from which this rfc is based:
The vtable layout here is the one used in the in-progress Mod-It-All framework, which is designed to allow modules written in foreign languages to nicely interact with each other. See <https://github.com/ModItAll/Framework/blob/726957eda4f02cc6c9d2cb3033d438cc2c1115cf/include/Framework.h#L7..17>. This layout is also being used in a concurrent proposed technical specification for the in-development Laser Language <https://github.com/ComLangDevelopment>.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Composing multiple traits in a trait object is currently not supported, but would it be useful to specify how that feature should interact with `stable_vtable` traits?
- Is it reasonable to impose `extern"C"` by default on the traits, which may have some penalty in static usage, or should it be exclusively the vtable items which are `extern"C"`.
- Should trait objects entirely composed of `auto` traits be subject to these rules, or should it remain unspecified?
- Should this rfc consider a COM-like layout, rather than the above vtable? If so, would it primarily be a matter of reordering the data and vtable items in the trait object, or would further changes be necessary? It should be noted that this would be contrary to the (albeit unstable) `core::raw::TraitObject` type, which has data before vtable (as in this rfc). 

# Future possibilities
[future-possibilities]: #future-possibilities

As mentioned above, this rfc is submitted concurrently to a proposed technical specification for the in-development Laser Language. It is intended that comments related to the actual semantics of the proposal (beyond the rust-specific syntax) be relayed to the comments on that proposed technical specification, and the reverse. As both proposals evolve, more specific changes may occur (including a resolution of the question reguarding the use of a COM-like layout). 

The `dealloc` item is used as a deallocation function for allocated pointers of the type. In the initial discussion, `Box<dyn Trait>` was used to indicate a (potentially foreign) smart pointer, 
 which would need to be deallocated using that vtable entry. However, this would cause unsoundness in theoretically sound code.  
While the entry was reintroduced, it presently has no use in this rfc A future extension to this could be to introduce a standard smart pointer similar to box that allocates using either the system allocator, 
 or potentially a type-specific allocator, and deallocates the pointer using that entry. (Or alternatively, with the `allocator_api` implementation, a "type-aware" allocator that when used in a Box, calls the deallocation entry if present, otherwise the `System` allocator).

Under this rfc, the `dealloc` item *could* be used with a user-provided smart pointer type to provide that functionality, however the safety/soundness may be predicated on either a user-provided trait implementation, or on a standard library trait for TraitObjects, and specifically trait objects with stable vtable layout. 

