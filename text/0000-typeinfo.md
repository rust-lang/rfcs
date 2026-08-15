- Feature Name: typeinfo
- Start Date: 2019-08-01
- RFC PR: [rust-lang/rfcs#2738](https://github.com/rust-lang/rfcs/pull/2738)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a new unsafe trait `TypeInfo` to `core::any`, and implement it for all types. This has a method `type_id` with the signature `fn type_id(&self) -> TypeId where Self: 'static`, as well as a method `type_name` with the signature `fn type_name(&self) -> &'static str`. Traits wanting to support downcasting can add it as a supertrait, without breaking backwards compatibility.

# Motivation
[motivation]: #motivation

This enables traits other than `core::any::Any`, such as `std::error::Error`, to soundly support downcasting in safe code. The initial implementation of this relied on simply adding a method to `Error`, but this was found to be unsound to override.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The new `core::any::TypeInfo` unsafe trait provides a similar functionality to `core::any::Any`, providing a `type_id` method for types where `Self: 'static`. However, the trait itself does not impose any requirements on its implementors. This is unlike `core::any::Any`, which requires `Self: 'static` for the trait to be implemented. It is also implemented for every type. These features enable it to be added as a supertrait without breaking backwards compatibility. The trait's definition follows:

```rust
pub unsafe trait TypeInfo {
    fn type_id(&self) -> TypeId where Self: 'static {
        TypeId::of::<Self>()
    }
    
    fn type_name(&self) -> &'static str {
        core::any::type_name::<Self>()
    }
}

unsafe impl<T: ?Sized> TypeInfo for T {} // enables assuming all types implement TypeInfo, without breaking backwards compatibility
```

While not a part of this RFC, making `core::any::TypeInfo` a supertrait of `std::error::Error` would solve the soundness issues with `Error::type_id` while preserving backwards compatbility with stable code and preserving the same functionality.

As an example of the primary functionality of this trait, let's say we have a trait CanDoThing, and we want to add the ability to convert an `&dyn CanDoThing + 'static` to `&DoesThing`. Before the update, we have this code:

```rust
pub trait CanDoThing {
    fn do_it();
}

pub struct DoesThing;

impl CanDoThing for DoesThing {
    fn do_it() {
        println!("Did the thing!");
    }
}
```

After the update, we have this:


```rust
pub trait CanDoThing: TypeInfo {
    fn do_it(&self);
}

pub struct DoesThing;

impl CanDoThing for DoesThing {
    fn do_it(&self) {
        println!("Did the thing!");
    }
}

impl dyn CanDoThing + 'static {
    pub fn is<T: CanDoThing + 'static>(&self) -> bool {
        // `TypeId::of::<Self>()` would be `dyn CanDoThing`, not the "actual" type of Self
        TypeId::of::<T>() == self.type_id()
    }
    
    pub fn downcast_ref<T: CanDoThing + 'static>(&self) -> Option<&T> {
        if self.is::<T>() {
            unsafe {
                Some(&*(self as *const dyn CanDoThing as *const T))
            }
        } else {
            None
        }
    }
}
```

[(playground link)](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2018&gist=a03eda57fa9199c0d7ef06768ca26b33)

In this case, we could use Any instead of TypeInfo. However, what if another crate had this code?

```rust
struct MyThingDoer<'a> {
    pub thing_text: &'a str,
}

impl CanDoThing for MyThingDoer<'_> { ... }
```

Before we added the Any bound, it worked fine. After we added it, that crate would get an error! If CanDoThing uses TypeInfo instead, there isn't a problem.

[(playground link)](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2018&gist=6ead12d4e5c525451a6bf477f81afcc1)

The trait also has a secondary functionality, being that you can call `type_name` on any type to get its name for debugging.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`Error::type_id` was originally unsound, as it was used by unsafe code in the standard library to get the TypeId of any `Error + 'static` type. If a crate overrode it, then the standard library would effectively transmute an `Error + 'static` to any type chosen by the user in safe code.

Making the method unsafe does not have the correct semantics, since it implies *using* the method is unsafe. A method being unsafe does not necessarily affect the safety of implementing it, unlike an unsafe trait. Thus, an unsafe trait is necessary for the proper semantics.

The implementation on all types was *not* intended to prevent overrides on user types. It was added primarily so that adding it as a supertrait would not break backwards compatibility, and secondarily for convenience when calling `type_id` or `type_name` directly.

Note that simply doing `TypeId::of::<Self>()` in an `impl dyn Trait` gives the TypeId of `dyn Trait`, and not the "real" type [(playground link)](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=da9811b06256cd4b00e156aaf035be28).

# Drawbacks
[drawbacks]: #drawbacks

This trait may be too similar to `core::any::Any`, and is in the same module. However, I believe that the subtle differences provide a useful motivation to add it to the standard library.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs? This design provides the proper semantics, allowing unsafe code to use it for downcasting without worry of a user overriding `type_id`.
- What other designs have been considered and what is the rationale for not choosing them? Adding a safe `type_id` method on every trait needing downcasting has been attempted. However, if it is possible to override it without an `unsafe impl`, then it will allow unsoundness in safe code. Another method would be to have a private type used as an argument in traits that want the type_id. However, this is less idiomatic than an unsafe trait, and may be an error in future versions of Rust.
- What is the impact of not doing this? Currently, `std::error::Error` is using an unstable method as a naïve method to prevent user code from implementing the method. However, this does not help on nightly.

# Prior art
[prior-art]: #prior-art

In Rust itself, this trait is very similar to `core::any::Any`. However, that trait requires all of its implementors to have `Self: 'static`, preventing it from being added as a supertrait on existing traits without breaking backwards compatibility. Some other languages such as C# solve this problem by having *all* types provide a `GetType` method that cannot be overriden. This would be a far-reaching and unnecessary change for Rust, however.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged? The name of the TypeInfo trait is somewhat confusing, and may need bikeshedding.
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization? There may be issues in crates that `use std::any::*;`. However, I am not sure if any crates do this, or if it could be worked around.
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC? This RFC provides an elegant solution for the unsoundness of `Error::type_id`. However, there are other possible solutions as well.

# Future possibilities
[future-possibilities]: #future-possibilities

This was originally written to enable the functionality of `Error::type_id` to be soundly utilized in stable Rust. However, this is a more impactful change, since it would possibly remove an existing, albeit unstable, method on a widely-used trait.

# Credits
[credits]: #credits

This solution was originally identified by @programmerjake in https://github.com/rust-lang/rust/issues/60784#issuecomment-511039223. This RFC slightly differs in that it is not intended to be specific to the Error trait.
