- Feature Name: `exhaustive_traits`
- Start Date: 2025-11-24
- RFC PR: [rust-lang/rfcs#3885](https://github.com/rust-lang/rfcs/pull/3885)
- Rust Issue: [rust-lang/rust#3885](https://github.com/rust-lang/rust/issues/3885)

# Summary
[summary]: #summary
For any concrete type T, the set of #[exhaustive] traits it implements is finite and discoverable at runtime, enabling cross-trait casts.

Given:
```rust
#[exhaustive]
trait Behavior { ... }
```


If I have dyn Trait, I want to be able to attempt:
```rust
let any : &dyn Any = &MyStruct::new();
let casted: Option<&dyn Behavior> = any.cross_trait_cast_ref::<dyn Behavior>()
```
# Motivation
[motivation]: #motivation

It will enable dyn trait pattern matching, which will also enable many other ways of making programs.

Say you are making a game, and your bullet collides with an entity. If you want to damage it, you would want to check if the object has a `Damageable` trait so you can call its damage method,  assuming not everything in the game can be damaged. This method can be seen as another way of composition. A different pattern from having a collection of components (`Vec<Box<dyn Component>>`), or the ECS pattern.

`bevy_reflect`, which is used in the game engine “bevy”, has functionality that enables you to cast between unrelated traits.

But it involves macros and you have to manually register the trait to enable cross trait casting, and tell the struct “yeah you can be casted to this trait even if the compiler does not know your concrete type”

GUI/widget capabilities: `Clickable`, `Draggable`, `Focusable`, `Scrollable`, etc. In GUI frameworks, you may want “if this widget supports X, call X” at runtime without a giant enum or manual registration.

Making casting between unrelated traits a natural part of the language would make this much easier
# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation


If a trait is marked `#[exhaustive]`, then the compiler enforces certain rules that make cross trait casting work soundly.

### Rule 1: A crate may only implement an exhaustive trait for types it owns.

Equivalently:

* `impl SomeTrait for LocalType {}` is allowed.
* `impl<T> SomeTrait for Vec<T> {}` is rejected (type not owned).
* Blanket impls for the trait are also not allowed

The core problem with cross-trait casting is that two unrelated crates could each see the “same” concrete type but a different set of trait impls, due to downstream impls and blanket impls. With separate compilation, that makes any global “type → trait set” table incoherent unless you delay codegen or centralize it, both of which are hostile to Rust’s model.

`#[exhaustive]` sidesteps this by making the implementation set for a given type deterministic and crate-local:

* Every impl of an exhaustive trait for `T` must live in `T`’s defining crate.
* Therefore no other crate can add impls later.
* Therefore all crates see the same exhaustive-trait set for a type.

This makes a *restricted* form of cross-trait casting feasible.


This rule applies **even to the crate that defines the exhaustive trait**. Ownership of the **type** is what matters, not ownership of the trait.

### Rule 2: An impl is only allowed if the trait’s generic arguments are fully determined by the implementing type.

Concretely: in
`impl<...> ExhaustiveTrait<TraitArgs...> for SelfTy<TypeArgs...>`,
every generic parameter that appears in `TraitArgs...` must also be a generic parameter of `SelfTy`, or be a concrete argument (eg i32).

Examples
```rust
#[exhaustive]
trait MyTrait<T> {}

// ERR: creates infinite implementations for the exhaustive trait.
impl<U> MyTrait<U> for MyType {}

// OK: trait args are concrete → finite
impl MyTrait<i32> for MyType {}

// OK: trait arg is tied to Self’s generic parameter → 
// each concrete MyType<T> has exactly one matching impl
impl<T> MyTrait<T> for MyType<T> {}

// also OK: still determined by Self
impl<T> MyTrait<Vec<T>> for MyType<T> {}
```

This makes it impossible for type "T" to implement an infinite amount of `#[exhaustive]` traits, which is what we do not want, since the implementation set of #[exhaustive] traits should be deterministic.

Because the exhaustive-trait implementation set for the concrete type is deterministic, the compiler/runtime can safely use per-type metadata to answer “does this type implement `Behavior`?” in different crates without coherence surprises

### Rule 3: Exhaustive traits and all their implementors must be `'static`.

This gives us the ability to map traits to vtables. (TypeId -> dyn VTable) where `TypeId` represents the `TypeId` of the trait.



### Rule 4: Exhaustive traits must be object safe

This is self-explanatory. To be able to store the VTable of an `#[exhaustive]` trait implementation, the `#[exhaustive]` trait would need to be able to have a dyn vtable in the first place. 

if all the rules are satisfied, code that is similar to the code below will be possible

```rust
#[exhaustive]
trait A { fn a(&self) -> i32; }

#[exhaustive]
trait B { fn b(&self) -> i32; }

struct T(i32);

impl A for T { fn a(&self) -> i32 { self.0 } }
impl B for T { fn b(&self) -> i32 { self.0 * 2 } }

fn main() {
    let t = T(7);

    let a: &dyn A = &t;
    let b: &dyn B = a.cross_trait_cast_ref::<dyn B>().unwrap(); // cross-trait cast

    assert_eq!(a.a(), 7);
    assert_eq!(b.b(), 14);
}
```



# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Where are the VTable mappings stored?

Each type will have an array (`[(TypeId, TraitVTable)]`), where `TypeId` is the `TypeId` of the `dyn Trait`. this is possible because of the `'static only` restriction, and this is similar to how C# does it.

Essentially, an iteration would be done, until it finds the relevant vtable. If it cannot be found, `None` would be returned. Ofc, this makes it O(n), but C# has a fast path which we could be able to emulate, which I have yet to fully understand. Something we could discuss.

Inside the vtable of every trait object, a ptr that represents the array can be found. Types that do not have any exhaustive traits implemented, and non static types could simply have a ptr that represents an empty array

A quick sketch

```
struct VTable {
    *drop_in_place
    usize size;
    usize align;
    // method fns for TraitX, in trait order
    *method1
    *method2
    ...

    // NEW: pointer to exhaustive trait map for concrete T. points to the first implementation map if theres any
    ExhaustiveEntry* exhaustive_ptr;
    usize exhaustive_len;
};
```

### Intrinsics

We would have compiler intrinsics that would enable us to get the VTable for a trait object

```rust
// Auto implemented by traits that are exhaustive. Cannot be manually implemented.
pub trait Exhaustive{
    
}

#[rustc_intrinsic]
pub const unsafe fn exhaustive_vtable_of<T: ?Sized + Exhaustive + 'static, U: ptr::Pointee<Metadata = ptr::DynMetadata<U>> + ?Sized>(
    obj: *const U
) -> Option<core::ptr::DynMetadata<T>>;
```

where `U` is the trait object, `T` is the trait object we want to cast to. 
This intrinsic would be used by a non intrinsic functions to enable safe cross trait casting. Namely `cross_trait_cast_ref` and `cross_trait_cast_mut`

# Drawbacks
[drawbacks]: #drawbacks

Some drawbacks would be a slight increase in binary size when using trait objects. 

Even if no type in the program implements an `#[exhaustive]`  trait, each vtable of a trait object would still be forced to have a ptr that represents an array of trait implementations, even if it is empty.

Checking if an underlying type implements a trait would have a time complexity of O(n) in worst case.

At first, I thought of proposing something similar to a hashmap, but it would be slower than the array version in most cases, and would probably result to even bigger binary sizes

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why this design is best

Given the rules above, the compiler can build a finite, per-type trait→vtable table that is deterministic across crates. That directly enables cross-trait casting with predictable behavior and no manual bookkeeping.

It keeps extensibility where it matters: anyone can define an exhaustive trait, and the type’s crate opts in by implementing it once.

### Other designs considered (and why not)

Before this, I had another design in mind: The compiler looks at every used type and every used trait object throughout the entire program, and does a many-to-many relationship between the types and traits to figure out whether the types implement the traits and store an `Option<VTable>` for each relationship somewhere. 

After further thought, this design  is either impossible or would require a significant shift in the way the rust compiler works, since each crate is compiled separately and different crates would see a different set of trait implementations. This RFC design works a lot better with the current rust compiler.


### Impact of not doing this

People who need runtime capability checks will keep rebuilding partial solutions (registries, ad-hoc reflection), leading to more boilerplate, more bugs, and less interoperable patterns.

### Could this be a library/macro instead?

A library can be used to make this feature possible, like `bevy_reflect` has done, but only by adding extra registration steps. It requires a lot of boilerplate and there could be instances of casts failing despite the type implementing the trait, simply because the dev forgot to register the relationship between the trait and the type

The proposal reduces maintenance burden by making the relationship “type implements exhaustive trait” automatically discoverable at runtime, without extra code paths to keep in sync.
# Prior art
[prior-art]: #prior-art

C#, Java, Go, Swift all support “interface/protocol assertions” at runtime.
You can take an erased interface value and ask whether it also implements another interface/protocol, getting either a new interface view or failure. Their runtimes do a conformance lookup and return the right dispatch table (or cached equivalent).

### Good
Very ergonomic for capability-based code; enables “if it supports X, use X” patterns.

### Bad 
There would be some binary size costs


### Rust prior art
Rust stabilized dyn upcasting for subtrait→supertrait only, explicitly not for unrelated traits.

### Trait registry crates (Rust ecosystem).
Crates like `bevy_reflect` exist to allow this, but they rely on manual/derive registration and can’t be compiler-verified as complete—matching the gap this RFC targets. 

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is `#[exhaustive]` really a good name for these kinds of traits?
- Could `#[exhaustive]` be a `keyword` rather than an attribute?
- Is there a more efficient way to map traits to vtables other than using trait TypeIds?
- Would it be possible to make the `#[exhaustive]` trait implementation rules more flexible while preserving soundness?

# Future possibilities
[future-possibilities]: #future-possibilities

No additional future possibilities are identified at this time.
