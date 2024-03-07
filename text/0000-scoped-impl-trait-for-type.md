- Feature Name: `scoped_impl_trait_for_type`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This proposal adds scoped `impl Trait for Type` items into the core language, as coherent but orphan-rule-free alternative to implementing traits globally. It also extends the syntax of `use`-declarations to allow importing these scoped implementations into other scopes (including other crates), and differentiates type identity of most generics by which scoped trait implementations are available to each specified generic type parameter.

This (along with some details specified below) enables any crate to

- locally, in item scopes, implement nearly any trait for any expressible type,
- publish these trivially composable implementations to other crates,
- import and use such implementations safely and seamlessly and
- completely ignore this feature when it's not needed\*.

\* aside from one hopefully very obscure `TypeId` edge case that's easy to accurately lint for.

This document uses "scoped implementation" and "scoped `impl Trait for Type`" interchangeably. As such, the former should always be interpreted to mean the latter below.

# Motivation
[motivation]: #motivation

While orphan rules regarding trait implementations are necessary to allow crates to add features freely without fear of breaking dependent crates, they limit the composability of third party types and traits, especially in the context of derive macros.

For example, while many crates support `serde::{Deserialize, Serialize}` directly, implementations of the similarly-derived `bevy_reflect::{FromReflect, Reflect}` traits are less common. Sometimes, a `Debug`, `Clone` or (maybe only contextually sensible) `Default` implementation for a field is missing to derive those traits. While crates like Serde often do provide ways to supply custom implementations for fields, this usually has to be restated on each such field. Additionally, the syntax for doing so tends to differ between crates.

Wrapper types, commonly used as workaround, add clutter to call sites or field types, and introduce mental overhead for developers as they have to manage distinct types without associated state transitions in order to work around the issues laid out in this section. They also require a distinct implementation for each combination of traits and lack discoverability through tools like rust-analyzer.

Another pain point are sometimes missing `Into<>`-conversions when propagating errors with `?`, even though one external residual (payload) type may (sometimes *contextually*) be cleanly convertible into another. As-is, this usually requires a custom intermediary type, or explicit conversion using `.map_err(|e| …)` (or an equivalent function/extension trait). If an appropriate `From<>`-conversion can be provided *in scope*, then just `?` can be used.

This RFC aims to address these pain points by creating a new path of least resistance that is easy to use and very easy to teach, intuitive to existing Rust-developers, readable without prior specific knowledge, discoverable as needed, has opportunity for rich tooling support in e.g. rust-analyzer and helpful error messages, is quasi-perfectly composable including decent re-use of composition, improves maintainability and (slightly) robustness to major-version dependency changes compared to newtype wrappers, and does not restrict crate API evolution, compromise existing coherence rules or interfere with future developments like specialisation. Additionally, it allows the implementation of more expressive (but no less explicit) extension APIs using syntax traits like in the `PartialEq<>`-example below, without complications should these traits be later implemented in the type-defining crate.

For realistic examples of the difference this makes, please check the [rationale-and-alternatives] section.

# (Pending changes to this draft)

It should be possible to specify differences in the implementation environment directly where it is captured, e.g. as `BTreeSet<usize: PartialOrd in reverse + Ord in reverse>`, without bringing these implementations into scope.

As this requires additional grammar changes and overall more adjustments to this document, I plan to tackle that a bit later.

For now, see [explicit-binding] in *Future possibilities* below for more, but less rigorous, text about one possibility.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Scoped `impl Trait for Type` can be introduced in The Book alongside global trait implementations and mentioned in the standard library documentation examples.

For example, the following changes could be made:

## **10.2.** Traits: Defining Shared Behavior

The following sections are added after [Implementing a Trait on a Type]:

[Implementing a Trait on a Type]: https://doc.rust-lang.org/book/ch10-02-traits.html#implementing-a-trait-on-a-type

### Scoped Implementation of a Trait on a Type

Independently of implementing a trait on a type or set of types *globally*, it's possible to do so only for the current scope, by adding the `use` keyword:

```rust
use impl Trait for Type {
    // ...
}
```

With the exception of very few traits related to language features, you can implement any visible trait on any visible type this way, even if both are defined in other crates.

In other words: The *orphan rule* does not apply to scoped implementations. Instead, item shadowing is used to determine which implementation to use.

*Scoped implementations are intended mainly as compatibility feature*, to let third party crates provide glue code for other crate combinations. To change the behaviour of an instance or a set of instances from their default, consider using [the newtype pattern] instead.

[`Hash`]: https://doc.rust-lang.org/stable/std/hash/trait.Hash.html
[`PartialEq`]: https://doc.rust-lang.org/stable/std/cmp/trait.PartialEq.html
[`Eq`]: https://doc.rust-lang.org/stable/std/cmp/trait.Eq.html
[`PartialOrd`]: https://doc.rust-lang.org/stable/std/cmp/trait.PartialOrd.html
[`Ord`]: https://doc.rust-lang.org/stable/std/cmp/trait.Ord.html

[`Deserialize`]: https://docs.rs/serde/1/serde/trait.Deserialize.html
[`Serialize`]: https://docs.rs/serde/1/serde/trait.Serialize.html

[the newtype pattern]: https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#using-the-newtype-pattern-to-implement-external-traits-on-external-types

### Publishing and Importing Scoped Implementations

You can also publish a scoped implementation further by adding a visibility before `use` ...:

```rust
pub use impl Trait for Type {
    // ...
}

pub use unsafe impl UnsafeTrait for Type {
    // ...
}
```

... and import it into other scopes:

```rust
use other_module::{
    impl Trait for Type,
    impl UnsafeTrait for Type,
};
```

Note that the scoped implementation of `UnsafeTrait` is imported without the `unsafe` keyword. **It is the implementing crate's responsibility to ensure the exported `unsafe` implementation is sound everywhere it is visible!**

Generic parameters, bounds and `where`-clauses can be used as normal in each of these locations, though you usually have to brace `impl</*...*/> Trait for Type where /*...*/` individually in `use`-declarations.

You can import a subset of a generic implementation, by narrowing bounds or replacing type parameters with concrete types in the `use`-declaration.

Global implementations can be imported from the root namespace, for example to shadow a scoped implementation:

```rust
use ::{impl Trait for Type};
```

### Scoped implementations and generics
[scoped-implementations-and-generics]: #scoped-implementations-and-generics

Scoped implementations are resolved on most generics' type parameters where those are specified, and become part of the (now less generic) host type's identity:

```rust
struct Type<T>(T);

trait Trait {
    fn trait_fn();
}

impl<T: Trait> Type<T> {
    fn type_fn() {
        T::trait_fn();
    }
}

mod nested {
    use impl Trait for () {
        fn trait_fn() {
            println!("nested");
        }
    }

    pub type Alias = Type<()>;
}
use nested::Alias;

Alias::type_fn(); // "nested"

// Type::<()>::type_fn();
//             ^^^^^^^ error[E0599]: the function or associated item `type_fn` exists for struct `Type<()>`, but its trait bounds were not satisfied

// let t: Type<()> = Alias(());
//                   ^^^^^^^^^ error[E0308]: mismatched types
```

This works equally not just for type aliases but also fields, `let`-bindings and also where generic type parameters are inferred automatically from expressions (for example to call a constructor).

Note that some utility types, like references, tuples, `Option`, `Result` and closure traits, do not bind implementations eagerly but only when used to specify another generic. You can find a list of these types in the reference. (← i.e. "insert link here".)

## **19.2.** Advanced Traits

The section [Using the Newtype Pattern to Implement External Traits on External Types] is updated to mention scoped implementations, to make them more discoverable when someone arrives from an existing community platform answer regarding orphan rule workarounds. It should also mention that newtypes are preferred over scoped implementations when use of the type is semantically different, to let the type checker distinguish it from others.

[Using the Newtype Pattern to Implement External Traits on External Types]: https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#using-the-newtype-pattern-to-implement-external-traits-on-external-types

A new section is added:

### Using Scoped Implementations to Implement External Traits on External Types
[using-scoped-implementations-to-implement-external-traits-on-external-types]: #using-scoped-implementations-to-implement-external-traits-on-external-types

Since scoped implementations allow crates to reusably implement external traits on external types, they can be used to provide API extensions that make use of syntactic sugar. For example:

Filename: fruit-comparer/src/lib.rs

```rust
use apples::Apple;
use oranges::Orange;

pub use impl PartialEq<Orange> for Apple {
    fn eq(&self, other: &Orange) -> bool {
        todo!("Figure out how to compare apples and oranges.")
    }
}

pub use impl PartialEq<Apple> for Orange {
    fn eq(&self, other: &Orange) -> bool {
        todo!("Figure out how to compare oranges and apples.")
    }
}
```

Filename: src/main.rs

```rust
use apples::Apple;
use oranges::Orange;

use fruit_comparer::{
    impl PartialEq<Orange> for Apple,
    impl PartialEq<Apple> for Orange,
};

fn main() {
    let apple = Apple::new();
    let orange = Orange::new();

    // Compiles:
    dbg!(apple == orange);
    dbg!(orange == apple);
}
```

If the type whose API was extended this way later gains the same trait inherently, that is not a problem as the consuming code continues to use `fruit_comparer`'s scoped implementation. However, a warning ([global-trait-implementation-available]) is shown by default to alert the maintainers of each crate of the covering global implementation.

Be careful about literal coercion when using generic traits this way! For example, if a scoped implementation of `Index<isize>` is used and a global `Index<usize>` implementation is added later on the same type, the compiler will *not* automatically decide which to use for integer literal indices between these two.

## Rustdoc documentation changes

### `use` and `impl` keywords

The documentation pages [for the `use` keyword] and [for the `impl` keyword] are adjusted to (very) briefly demonstrate the respective scoped use of `impl Trait for Type`.

[for the `use` keyword]: https://doc.rust-lang.org/stable/std/keyword.use.html
[for the `impl` keyword]: https://doc.rust-lang.org/stable/std/keyword.impl.html

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Grammar changes
[grammar-changes]: #grammar-changes

The core Rust language grammar is extended as follows:

- [*TraitImpl*]'s definition is prepended with (*Visibility*<sup>?</sup> `use`)<sup>?</sup> and refactored for partial reuse to arrive at

  > *TraitImpl*:  
  > &emsp; **(*Visibility*<sup>?</sup> `use`)<sup>?</sup>** `unsafe`<sup>?</sup> ***TraitCoverage***  
  > &emsp; `{`  
  > &emsp;&emsp; *InnerAttribute*<sup>\*</sup>  
  > &emsp;&emsp; *AssociatedItem*<sup>\*</sup>  
  > &emsp; `}`  
  >
  > **TraitCoverage**:  
  > &emsp; ***TraitCoverageNoWhereClause***  
  > &emsp; *WhereClause*<sup>?</sup>  
  >
  > **TraitCoverageNoWhereClause**:  
  > &emsp; `impl` *GenericParams*<sup>?</sup> `!`<sup>?</sup> *TypePath* `for` *Type*  

  where a trait implementation with that `use`-prefix provides the implementation *only* as item in the containing item scope.

  (This can be distinguished from `use`-declarations with a lookahead up to and including `impl` or `unsafe`, meaning at most four shallowly tested token trees with I believe no groups. No other lookaheads are introduced into the grammar by this RFC.)

  **The scoped implementation defined by this item is implicitly always in scope for its own definition.** This means that it's not possible to refer to any shadowed implementation inside of it (including generic parameters and where clauses), except by re-importing specific scoped implementations inside nested associated functions. Calls to generic functions cannot be used as backdoor either (see [type-parameters-capture-their-implementation-environment]).

  [*TraitImpl*]: https://doc.rust-lang.org/reference/items/implementations.html?highlight=TraitImpl#implementations

- [*UseTree*]'s definition is extended for importing scoped implementations by inserting the extracted *TraitCoverage* and *TraitCoverageNoWhereClause* rules as follows:

  > *UseTree*:  
  > &emsp; (*SimplePath*<sup>?</sup> `::`)<sup>?</sup> `*`  
  > &ensp;| (*SimplePath*<sup>?</sup> `::`)<sup>?</sup> `{`  
  > &emsp; (  
  > &emsp;&emsp; (**(**&zwj;*UseTree* **| *TraitCoverageNoWhereClause*)** (`,` **(**&zwj;*UseTree* **| *TraitCoverageNoWhereClause*)**)\* **(**`,` ***TraitCoverage*<sup>?</sup>)**<sup>?</sup>)<sup>?</sup>  
  > &emsp;&ensp;**| *TraitCoverage***  
  > &emsp; )  
  > &emsp; `}`  
  > &ensp;| *SimplePath* (`as` (IDENTIFIER | `_`))<sup>?</sup>  

  Allowing a trailing *TraitCoverage* with *WhereClause* in a braced list is intended for ergonomics, but rustfmt should brace it individually by default, then append a trailing comma where applicable as usual. A '`,`' in the *WhereClause* here is not truly ambiguous because *WhereClauseItem*s contain '`:`', but allowing that ahead of others would likely be visually confusing and tricky to implement (requiring an arbitrarily long look-ahead). Alternatively to allowing a trailing *TraitCoverage* in mixed lists, an error similar to [E0178] could be emitted.

  [E0178]: https://doc.rust-lang.org/error_codes/E0178.html

  > Allowing unbraced imports like `use some_crate::impl<A, B> Trait<A> for Type<B> where A: Debug, B: Debug;` would break the source code's visual hierarchy quite badly, so I won't suggest it here, but it is possible without ambiguity too. If that is added for convenience, then I'm strongly in favour of rustfmt bracing the *TraitCoverage* by default and rust-analyzer suggesting it only braced.

  Here, *TraitCoverage* imports the specified scoped `impl Trait for Type` for binding and conflict checks as if defined in the scope containing the `use`-declaration. The resulting visibility is taken from *UseDeclaration*, like with *SimplePath*-imported items.

  *TraitCoverage* must be fully covered by the scoped implementation visible in the source module. Otherwise, a compile-error occurs explaining the uncovered case (similarly to the current error(s) for missing trait implementations).

  ***TraitCoverage* may subset the source module's implementation** by having narrower bounds or using concrete types in place of one or more generic type parameters. This causes only the specified subset of the scoped implementation to be imported.

  Note that scoped implementations of `unsafe` traits are imported without `unsafe`. It is the exporting crate's responsibility to ensure a scoped implementation is sound everywhere it is visible.

  Other elements of the coverage must match the source module's implementation exactly, unless specified otherwise.

  [*UseTree*]: https://doc.rust-lang.org/reference/items/use-declarations.html?highlight=UseTree#use-declarations

## No scoped `impl Trait for Type` of auto traits, `Copy` and `Drop`

Implementations of auto traits state guarantees about private implementation details of the covered type(s), which an external implementation can almost never do soundly.

`Copy` is not an auto trait, but implementing it on a smart pointer like `Box<T>` would immediately be unsound. As such, this trait must be excluded from all external implementations.

Shadowing `Drop` for types that are `!Unpin` is similarly unsound without cooperation of the original crate (in addition to likely causing memory leaks in this and more cases).

## No scoped `impl !Trait for Type`

Any negative scoped implementation like for example

```rust
use impl !Sync for Type {}
```

is syntactically valid, but rejected by the compiler with a specific error. (See [negative-scoped-implementation].)

This also applies to `impl Trait`s in `use`-declarations (even though the items they would import cannot be defined anyway. Having a specific error saying that this *isn't possible* would be much clearer than one saying that the imported item doesn't exist).

## No external scoped implementations of sealed traits
[no-external-scoped-implementations-of-sealed-traits]: #no-external-scoped-implementations-of-sealed-traits

Consider this library crate:

```rust
pub struct Generic<T>(T);

mod private {
    // Implemented only on traits that are also `Sealed`.
    pub trait Sealing {}
}
use private::Sealing;

pub trait Sealed: Sealing {
    fn assumed {
        // (2)
    }
}

impl<T: Sealed> Generic {
    fn assuming {
        // (1)
    }
}
```

In this crate, any code at (1) is currently allowed to make safety-critical assumptions about code at (2) and other implementations of `assumed`.

To ensure this stays sound, scoped `impl Trait for Type` where `Trait` is external requires that all supertraits of `Trait` are visible to the crate defining the scoped implementation or are defined not in `Trait`'s definition crate (meaning they must still be exported from a crate somewhere in the dependency tree).

See also [scoped-implementation-of-external-sealed-trait].

## Type parameters capture their *implementation environment*
[type-parameters-capture-their-implementation-environment]: #type-parameters-capture-their-implementation-environment

When a type parameter is specified, either explicitly or inferred from an expression, it captures a view of *all* implementations that are applicable to its type there. This is called the type parameter's *implementation environment*.

(For trait objects, associated types are treated as type parameters for the purposes of this proposal.)

When implementations are resolved on the host type, bounds on the type parameter can only be satisfied according to this captured view. This means that implementations on generic type parameters are 'baked' into discretised generics and can be used even in other modules or crates where this discretised type is accessible (possibly because a value of this type is accessible). Conversely, additional or changed implementations on a generic type parameter in an already-discretised type *cannot* be provided anywhere other than where the type parameter is specified.

When a generic type parameter is used to discretise another generic, the captured environment is the one captured in the former but overlaid with modifications applicable to that generic type parameter's opaque type.

Note that type parameter defaults too capture their *implementation environment* where they are specified, so at the initial definition site of the generic. This environment is used whenever the type parameter default is used.

## Type identity of discrete types
[type-identity-of-discrete-types]: #type-identity-of-discrete-types

The type identity and `TypeId::of::<…>()` of discrete types, including discretised generics, are not affected by scoped implementations *on* them.

## Type identity of generic types
[type-identity-of-generic-types]: #type-identity-of-generic-types

### Implementation-aware generics
[implementation-aware-generics]: #implementation-aware-generics

Generics that are not [implementation-invariant-generics] are implementation-aware generics.

The type identity of implementation-aware generic types is derived from the types specified for their type parameters as well as the *full* *implementation environment* of each of their type parameters and their associated types:

```rust
#[derive(Default)]
struct Type;
#[derive(Default)]
struct Generic<T>(T);
trait Trait {}

impl<T> Generic<T> {
    fn identical(_: Self) {}
    fn nested_convertible<U: Into<T>>(_: Generic<U>) {}
}

mod mod1 {
    use crate::{Generic, Trait, Type};
    use impl Trait for Type {} // Private implementation, but indirectly published through `Alias1`.
    pub type Alias1 = Generic<Type>;
}

mod mod2 {
    use crate::{Generic, Trait, Type};
    pub use impl Trait for Type {} // Public implementation.
    pub type Alias2 = Generic<Type>;
}

mod mod3 {
    use crate::{Generic, Trait, Type};
    use crate::mod2::{impl Trait for Type}; // Reused implementation.
    pub type Alias3 = Generic<Type>;
}

mod mod4 {
    use crate::{Generic, Trait, Type};
    use impl<T> Trait for Generic<T> {} // Irrelevant top-level implementation.
    pub type Alias4 = Generic<Type>;
}

mod mod5 {
    use crate::{Generic, Type};
    // No implementation.
    pub type Alias5 = Generic<Type>;
}

use mod1::Alias1;
use mod2::Alias2;
use mod3::Alias3;
use mod4::Alias4;
use mod5::Alias5;

fn main() {
    use std::any::TypeId;

    use tap::Conv;

    // Distinct implementations produce distinct types.
    assert_ne!(TypeId::of::<Alias1>(), TypeId::of::<Alias2>());
    assert_ne!(TypeId::of::<Alias1>(), TypeId::of::<Alias3>());

    // Types with identical captured implementation environments are still the same type.
    assert_eq!(TypeId::of::<Alias2>(), TypeId::of::<Alias3>());

    // Top-level implementations are not part of type identity.
    assert_eq!(TypeId::of::<Alias4>(), TypeId::of::<Alias5>());

    // If the type is distinct, then values aren't assignable.
    // Alias1::identical(Alias2::default());
    //                   ^^^^^^^^^^^^^^^^^ error[E0308]: mismatched types

    // Fulfilled using the global reflexive `impl<T> Into<T> for T` on `Type`,
    // as from its perspective, the binding is stripped due to being top-level.
    Alias1::nested_convertible(Alias2::default());

    // The reflexive `impl<T> Into<T> for T` does not to the generic here,
    // as the distinct capture in the type parameter affects its inherent identity.
    // (It's unfortunately not possible to generically implement this conversion without specialisation.)
    // Alias1::default().conv::<Alias2>();
    //                   ^^^^ error[E0277]: the trait bound `[…]¹` is not satisfied

    // Identical types are interchangeable.
    Alias2::identical(Alias3::default());
    Alias4::identical(Alias5::default());
}
```

As mentioned in [type-identity-of-discrete-types], implementations on the generic type *itself* do *not* affect its type identity, as can be seen with `Alias4` above.

The `TypeId` of these generics varies alongside their identity. Note that due to the transmutation permission defined in [layout-compatibility], consumer code is effectively allowed to change the `TypeId` of instances of generics between calls to generic implementations in most cases. Due to this, implementations of generics that manage types at runtime should usually rely on the [typeid-of-generic-type-parameters-opaque-types] or `(…,)`-tuple-types combining them instead.

¹ With the current implementation, this would likely say `Generic<_>: From<Generic<_>>>`, which isn't helpful. With [explicit-binding], it could say `Generic<Type: Trait in mod2>: From<Generic<Type: Trait in mod1>>>`.

(For a practical example, see [logical-consistency] [of-generic-collections].)

### Implementation-invariant generics
[implementation-invariant-generics]: #implementation-invariant-generics

The following generics that never rely in the consistency of implementation of their type parameters are implementation-invariant:

- `&T`, `&mut T` (references),
- `*const T`, `*mut T` (pointers),
- `[T; N]`, `[T]` (arrays and slices),
- `(T,)`, `(T, U, ..)` (tuples),
- *superficially*\* `fn(T) -> U` and similar (function pointers),
- *superficially*\* `Fn(T) -> U`, `FnMut(T) -> U`, `FnOnce(T) -> U`, `Future<Output = T>`, `Iterator<Item = T>`, `std::ops::Coroutine` and similar (closures),
- `Pin<P>`, `NonNull<T>`, `Box<T>`, `Rc<T>`, `Arc<T>`, `Weak<T>`, `Option<T>`, `Result<T, E>`\*\*.

Implementation-invariant generics never capture *implementation environments* on their own. Instead, their effective *implementation environments* follow that of their host, acting as if they were captured in the same scope.

The type identity of implementation-invariant generics seen on their own does not depend on the implementation environment.

\* superficially: The underlying instance may well use a captured implementation internally, but this isn't surfaced in signatures. For example, a closure defined where `usize: PartialOrd in reverse + Ord in reverse` is just `FnOnce(usize)` but will use `usize: PartialOrd in reverse + Ord in reverse` privately when called.

\*\* but see [which-structs-should-be-implementation-invariant].

See also [why-specific-implementation-invariant-generics].

## `TypeId` of generic type parameters' opaque types
[typeid-of-generic-type-parameters-opaque-types]: #typeid-of-generic-type-parameters-opaque-types

In addition to the type identity of the specified type, the `TypeId` of opaque generic type parameter types varies according to the captured *implementation environment*, but *only according to implementations that are relevant to their bounds (including implicit bounds)*, so that the following program runs without panic:

```rust
use std::any::TypeId;

#[derive(Default)]
struct Type;
trait Trait {}
impl Trait for Type {}

#[derive(Default)]
struct Generic<T>(T);

mod nested {
    use super::{Trait, Type, Generic};
    use impl Trait for Type {};
    pub type B = Generic<Type>;
}

// `A` and `B` are distinct due to different captured implementation environments.
type A = Generic<Type>;
use nested::B;

fn no_bound<T: 'static, U: 'static>(_: (T,), _: (U,)) {
    assert_eq!(TypeId::of::<T>(), TypeId::of::<U>());
    assert_ne!(TypeId::of::<Generic<T>>(), TypeId::of::<Generic<U>>());

    assert_eq!(TypeId::of::<T>(), TypeId::of::<Type>());
    assert_eq!(TypeId::of::<U>(), TypeId::of::<Type>());
}

fn yes_bound<T: Trait + 'static, U: Trait + 'static>(_: (T,), _: (U,)) {
    assert_ne!(TypeId::of::<T>(), TypeId::of::<U>());
    assert_ne!(TypeId::of::<Generic<T>>(), TypeId::of::<Generic<U>>());

    assert_eq!(TypeId::of::<T>(), TypeId::of::<Type>());
    assert_ne!(TypeId::of::<U>(), TypeId::of::<Type>());
}

fn main() {
    no_bound(A::default(), B::default());
    yes_bound(A::default(), B::default());
}
```

In particular:

- If no bound-relevant scoped implementations are captured in a type parameter, then the `TypeId` of the opaque type of that type parameter is identical to that of the discrete type specified for that type parameter.
- Distinct sets of bound-relevant captured scoped implementations lead to distinct `TypeId`s of the opaque type of a type parameter.
- If the set of bound-relevant captured scoped implementations in two generic type parameters is the same, and the captured discrete type is identical, then the `TypeId` of the opaque types of these generic type parameters is identical.
- If a generic type parameter is distinguishable this way, it remains distinguishable in called implementations even if those have fewer bounds - the relevant distinction is 'baked' into the generic type parameter's opaque type.

These rules (and the transmutation permission in [layout-compatibility]) allow the following collection to remain sound with minimal perhaps unexpected behaviour:

```rust
use std::{
    any::TypeId,
    collections::{
        hash_map::{HashMap, RandomState},
        HashSet,
    },
    hash::{BuildHasher, Hash},
    mem::drop,
};

use ondrop::OnDrop;

#[derive(Default)]
pub struct ErasedHashSet<'a, S: 'a + BuildHasher + Clone = RandomState> {
    storage: HashMap<TypeId, *mut (), S>,
    droppers: Vec<OnDrop<Box<dyn FnOnce() + 'a>>>,
}

impl ErasedHashSet<'_, RandomState> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a, S: BuildHasher + Clone> ErasedHashSet<'a, S> {
    pub fn with_hasher(hasher: S) -> Self {
        Self {
            storage: HashMap::with_hasher(hasher),
            droppers: vec![],
        }
    }

    // This is the important part.
    pub fn insert<T: 'a>(&mut self, value: T) -> bool
    where
        T: Hash + Eq + 'static, // <-- Bounds.
    {
        let type_id = TypeId::of::<T>(); // <-- `TypeId` depends on implementations of bounds.
        let storage: *mut () = if let Some(storage) = self.storage.get_mut(&type_id) {
            *storage
        } else {
            let pointer = Box::into_raw(Box::new(HashSet::<T, S>::with_hasher(
                self.storage.hasher().clone(),
            )));
            self.droppers.push(OnDrop::new(Box::new(move || unsafe {
                // SAFETY: Only called once when the `ErasedHashSet` is dropped.
                //         The type is still correct since the pointer wasn't `.cast()` yet and
                //         both `S` and `T` are bounded on `'a`, so they are still alive at this point.
                drop(Box::from_raw(pointer));
            })));
            self.storage
                .insert(type_id, pointer.cast::<()>())
                .expect("always succeeds")
        };

        let storage: &mut HashSet<T, S> = unsafe {
            // SAFETY: Created with (close to) identical type above.
            //         Different `Hash` and `Eq` implementations are baked into `T`'s identity because of the bounds, so they result in distinct `TypeId`s above.
            //         It's allowed to transmute between types that differ in identity only by bound-irrelevant captured implementations.
            //         The borrowed reference isn't returned.
            &mut *(storage.cast::<HashSet<T, S>>())
        };
        storage.insert(value)
    }

    // ...
}
```

In particular, this code will ignore any scoped implementations on `T` that are not `Hash`, `Eq` or (implicitly) `PartialEq`, while any combination of distinct discrete type and *implementation environments* with distinct `Hash`, `Eq` or `PartialEq` implementations is cleanly separated.

See also [behaviour-changewarning-typeid-of-implementation-aware-generic-discretised-using-generic-type-parameters] for how to lint for an implementation of this collection that uses `TypeId::of::<HashSet<T, S>>()` as key, which *also* remains sound and deterministic but distinguishes too aggressively by irrelevant scoped implementations in consumer code, leading to unexpected behaviour.

(For an example of `TypeId` behaviour, see [logical-consistency] [of-type-erased-collections].)

## Layout-compatibility
[layout-compatibility]: #layout-compatibility

Types whose identities are only distinct because of a difference in *implementation environments* remain layout-compatible as if one was a `#[repr(transparent)]` newtype of the other.

It is sound to transmute an instance between these types **if** no inconsistency is observed on that instance by the bounds of any external-to-the-`transmute` implementation or combination of implementations, including scoped implementations and implementations on discrete variants of the generic. As a consequence, the `Self`-observed `TypeId` of instances of generic types **may** change in some cases.

For example, given a library

```rust
#[derive(Debug)]
pub struct Type<T>(T);

impl Type<usize> {
    pub fn method(&self) {}
}
```

then in another crate

- if `Debug` is used on an instance of `Type<T>`, then this instance may *not* be transmuted to one where `T: Debug` uses a different implementation and have `Debug` used on it again then and
- if `Type<usize>::method()` is used on an instance of `Type<usize>`, then that instance may not be transmuted (and used) to or from any other variant, including ones that only differ by captured *implementation environment*, because `method` has observed the *exact* type parameter through its constraints.

(In short: Don't use external-to-your-code implementations with the instance in any combination that wouldn't have been possible without transmuting the instance, pretending implementations can only observe the type identity according to their bounds.)

See [typeid-of-generic-type-parameters-opaque-types] for details on what this partial transmutation permission is for, and [behaviour-changewarning-typeid-of-implementation-aware-generic-discretised-using-generic-type-parameters] for a future incompatibility lint that could be used to warn implementations where this is relevant.

## No interception/no proxies

That each scoped `impl Trait for Type { /*...*/ }` is in scope for itself makes the use of the implementation it shadows in the consumer scope *inexpressible*. There can be no scoped implementation constrained to always shadow another.

This is intentional, as it makes the following code trivial to reason about:

```rust
{
    use a::{impl TheTrait for TheType}; // <-- Clearly unused, no hidden interdependencies.
    {
        use b::{impl TheTrait for TheType};
        // ...
    }
}
```

(The main importance here is to not allow non-obvious dependencies of imports. Implementations can still access associated items of a *specific* other implementation by bringing it into a nested scope or binding to its associated items elsewhere. See also [independent-trait-implementations-on-discrete-types-may-still-call-shadowed-implementations].)

## Binding choice by implementations' bounds
[binding-choice-by-implementations-bounds]: #binding-choice-by-implementations-bounds

Implementations bind to other implementations as follows:

| `where`-clause on `impl`? | binding-site of used trait | monomorphised by used trait? |
|-|-|-|
| Yes. | Bound at each binding-site of `impl`. | Yes, like-with or as-part-of type parameter distinction.  |
| No. | Bound once at definition-site of `impl`. | No. |

A convenient way to think about this is that *`impl`-implementations are blanket implementations over `Self` in different implementation environments*.

Note that `Self`-bounds on associated functions do **not** cause additional monomorphic variants to be emitted, as these continue to only filter the surrounding implementation.

Consider the following code with attention to the where clauses:

```rust
struct Type;

// ❶

trait Trait { fn function(); }
impl Trait for Type { fn function() { println!("global"); } }

trait Monomorphic { fn monomorphic(); }
impl Monomorphic for Type {
    fn monomorphic() { Type::function() }
}

trait MonomorphicSubtrait: Trait {
    fn monomorphic_subtrait() { Self::function(); }
}
impl MonomorphicSubtrait for Type {}

trait Bounded { fn bounded(); }
impl Bounded for Type where Type: Trait {
    fn bounded() { Type::function(); }
}

trait BoundedSubtrait: Trait {
    fn bounded_subtrait() { Type::function(); }
}
impl BoundedSubtrait for Type where Type: Trait {}

trait FnBoundedMonomorphic {
    fn where_trait() where Self: Trait { Self::function(); }
    fn where_monomorphic_subtrait() where Self: MonomorphicSubtrait { Self::monomorphic_subtrait(); }
}
impl FnBoundedMonomorphic for Type {}

trait NestedMonomorphic { fn nested_monomorphic(); }

trait BoundedOnOther { fn bounded_on_other(); }
impl BoundedOnOther for () where Type: Trait {
    fn bounded_on_other() { Type::function(); }
}

Type::function(); // "global"
Type::monomorphic(); // "global"
Type::monomorphic_subtrait(); // "global"
Type::bounded(); // "global"
Type::bounded_subtrait(); // "global"
Type::where_trait(); // "global"
Type::where_monomorphic_subtrait(); // "global"
Type::nested_monomorphic(); // "scoped"
()::bounded_on_other(); // "global"

{
    // ❷
    use impl Trait for Type {
        fn function() {
            println!("scoped");
        }
    }

    // use impl FnBoundedMonomorphic for Type {}
    // error: the trait bound `Type: MonomorphicSubtrait` is not satisfied

    Type::function(); // "scoped"
    Type::monomorphic(); // "global"
    // Type::monomorphic_subtrait(); // error; shadowed by scoped implementation
    Type::bounded(); // "scoped"
    Type::bounded_subtrait(); // "scoped"
    Type::where_trait(); // "global"
    Type::where_monomorphic_subtrait(); // "global"
    Type::nested_monomorphic(); // "scoped"
    ()::bounded_on_other(); // "global"

    {
        // ❸
        use impl MonomorphicSubtrait for Type {}
        use impl FnBoundedMonomorphic for Type {}

        impl NestedMonomorphic for Type {
            fn nested_monomorphic() { Type::function() }
        }

        Type::function(); // "scoped"
        Type::monomorphic(); // "global"
        Type::monomorphic_subtrait(); // "scoped"
        Type::bounded(); // "scoped"
        Type::bounded_subtrait(); // "scoped"
        Type::where_trait(); // "scoped"
        Type::where_monomorphic_subtrait(); // "scoped"
        Type::nested_monomorphic(); // "scoped"
        ()::bounded_on_other(); // "global"
    }
}
```

The numbers ❶, ❷ and ❸ mark relevant item scopes.

Generic item functions outside `impl` blocks bind and behave the same way as generic `impl`s with regard to scoped `impl Trait for Type`.

### `Trait` / `::function`

This is a plain monomorphic implementation with no dependencies. As there is a scoped implementation at ❷, that one is used in scopes ❷ and ❸.

### `Monomorphic` / `::monomorphic`

Another plain monomorphic implementations.

As there is no bound, an implementation of `Trait` is bound locally in ❶ to resolve the `Type::function()`-call.

This means that even though a different `use impl Trait for Type …` is applied in ❷, the global implementation remains in use when this `Monomorphic` implementation is called into from there and ❸.

Note that the use of `Self` vs. `Type` in the non-default function body does not matter at all!

### `MonomorphicSubtrait` / `::monomorphic_subtrait`

Due to the supertrait, there is an implied bound `Self: Trait` *on the trait definition, but not on the implementation*.

This means that the implementation remains monomorphic, and as such depends on the specific (global) implementation of `Trait` in scope at the `impl MonomorphicSubtrait …` in ❶.

As this `Trait` implementation is shadowed in ❷, the `MonomorphicSubtrait` implementation is shadowed for consistency of calls to generics bounded on both traits.

In ❸ there is a scoped implementation of `MonomorphicSubtrait`. As the default implementation is monomorphised for this implementation, it binds to the scoped implementation of `Trait` that is in scope here.

### `Bounded` / `::bounded`

The `Type: Trait` bound (can be written as `Self: Trait` &ndash; they are equivalent.) selects the `Bounded`-binding-site's `Type: Trait` implementation to be used, rather than the `impl Bounded for …`-site's.

In ❶, this resolves to the global implementation as expected.

For the scopes ❷ and ❸ together, `Bounded` gains one additional monomorphisation, as here another `Type: Trait` is in scope.

### `BoundedSubtrait` / `::bounded_subtrait`

As with `MonomorphicSubtrait`, the monomorphisation of `impl BoundedSubtrait for Type …` that is used in ❶ is shadowed in ❷.

However, due to the `where Type: Trait` bound *on the implementation*, that implementation is polymorphic over `Trait for Type` implementations. This means a second monomorphisation is available in ❷ and its nested scope ❸.

### `FnBoundedMonomorphic`

`FnBoundedMonomorphic`'s implementations are monomorphic from the get-go just like `Monomorphic`'s.

Due to the narrower bounds on functions, their availability can vary between receivers but always matches that of the global implementation environment:

#### `::where_trait`

Available everywhere since `Type: Trait` is in scope for both implementations of `FnBoundedMonomorphic`.

In ❶, this resolves to the global implementation.

In ❷, this *still* calls the global `<Type as Trait in ::>::function()` implementation since the global `FnBoundedMonomorphic` implementation is *not* polymorphic over `Type: Trait`.

In ❸, `FnBoundedMonomorphic` is monomorphically reimplemented for `Type`, which means it "picks up" the scoped `Type: Trait` implementation that's in scope there from ❷.

#### `::where_monomorphic_subtrait`

In ❶, this resolves to the global implementation.

In ❷, this *still* calls the global `<Type as MonomorphicSubtrait in ::>::monomorphic_subtrait()` implementation since the global `FnBoundedMonomorphic` implementation is *not* polymorphic over `Type: Trait`.

Note that `FnBoundedMonomorphic` *cannot* be reimplemented in ❷ since the bound `Type: MonomorphicSubtrait` on its associated function isn't available in that scope, which would cause a difference in the availability of associated functions (which would cause a mismatch when casting to `dyn FnBoundedMonomorphic`).

> It may be better to allow `use impl FnBoundedMonomorphic for Type {}` without `where_monomorphic_subtrait` in ❷ and disallow incompatible unsizing instead. I'm not sure about the best approach here.

In ❸, `FnBoundedMonomorphic` is monomorphically reimplemented for `Type`, which means it "picks up" the scoped `Type: Trait` implementation that's in scope there from ❷.

### `NestedMonomorphic` / `::nested_monomorphic`

The global implementation of `NestedMonomorphic` in ❸ the binds to the scoped implementation of `Trait` on `Type` from ❷ internally. This allows outside code to call into that function indirectly without exposing the scoped implementation itself.

### `BoundedOnOther` / `::bounded_on_other`

As this discrete implementation's bound isn't over the `Self` type (and does not involved generics), it continues to act only as assertion and remains monomorphic.

## Binding and generics

`where`-clauses without generics or `Self` type, like `where (): Debug`, **do not** affect binding of implementations within an `impl` or `fn`, as the non-type-parameter-type `()` is unable to receive an implementation environment from the discretisation site.

However, `where (): From<T>` **does** take scoped implementations into account because the blanket `impl<T, U> From<T> for U where T: Into<U> {}` is sensitive to `T: Into<()>` which is part of the implementation environment captured in `T`!

This sensitivity even extends to scoped `use impl From<T> for ()` at the discretisation site, as the inverse blanket implementation of `Into` creates a scoped implementation of `Into` wherever a scoped implementation of `From` exists.  
This way, existing symmetries are fully preserved in all contexts.

## Implicit shadowing of subtrait implementations

Take this code for example:

```rust
use std::ops::{Deref, DerefMut};

struct Type1(Type2);
struct Type2;

impl Deref for Type1 {
    type Target = Type2;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Type1 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn function1(_x: impl Deref + DerefMut) {}
fn function2(x: impl DerefMut) {
    x.deref();
}

{
    use impl Deref for Type1 {
        type Target = ();

        fn deref(&self) -> &Self::Target {
            &()
        }
    }

    function1(Type1(Type2)); // <-- Clearly impossible.
    function2(Type1(Type2)); // <-- Unexpected behaviour if allowed.
}
```

Clearly, `function1` cannot be used here, as its generic bounds would have to bind to incompatible implementations.

But what about `function2`? Here, the bound is implicit but `Deref::deref` can still be accessed. For type compatibility, this would have to be the shadowed global implementation, which is most likely unintended decoherence.

As such, **shadowing a trait implementation also shadows all respective subtrait implementations**. Note that the subtrait *may* still be immediately available (again), if it is implemented with a generic target and all bounds can be satisfied in the relevant scope:

```rust
trait Trait1 {
    fn trait1(&self);
}
trait Trait2: Trait1 { // <-- Subtrait of Trait1.
    fn uses_trait1(&self) {
        self.trait1();
    }
}
impl<T: Trait1> Trait2 for T {} // <-- Blanket implementation with bounds satisfiable in scope.

struct Type;
impl Trait1 for Type {
    fn trait1(&self) {
        print!("global");
    }
}

{
    use impl Trait1 for Type {
        fn trait1(&self) {
            print!("scoped");
        }
    }

    Type.uses_trait1(); // Works, prints "scoped".
}
```

If a subtrait implementation is brought into scope, it must be either an implementation with a generic target, or an implementation on a discrete type making use of the identical supertrait implementations in that scope. (This rule is automatically fulfilled by scoped implementation definitions, so it's only relevant for which scoped implementations can be imported via `use`-declaration.)

## Independent trait implementations on discrete types may still call shadowed implementations
[independent-trait-implementations-on-discrete-types-may-still-call-shadowed-implementations]: #independent-trait-implementations-on-discrete-types-may-still-call-shadowed-implementations

Going back to the previous example, but now implementing `Trait2` independently without `Trait1` in its supertraits:

```rust
trait Trait1 {
    fn trait1(&self);
}
trait Trait2 { // <-- Not a subtrait of `Trait1`.
    fn uses_trait1(&self);
}
impl Trait2 for Type { // <-- Implementation on discrete type.
    fn uses_trait1(&self) {
        self.trait1();
    }
}

struct Type;
impl Trait1 for Type {
    fn trait1(&self) {
        print!("global");
    }
}

{
    use impl Trait1 for Type {
        fn trait1(&self) {
            print!("scoped");
        }
    }

    Type.uses_trait1(); // Works, prints "global".
}
```

In this case, the implementation of `Trait2` is *not* shadowed at all. Additionally, since `self.trait1();` here binds `Trait` on `Type` directly, rather than on a generic type parameter, it uses whichever `impl Trait1 for Type` is in scope *where it is written*.

## Warnings

### Unused scoped implementation
[unused-scoped-implementation]: #unused-scoped-implementation

Scoped implementations and `use`-declarations of such receive a warning if unused. This can also happen if a `use`-declaration only reapplies a scoped implementation that is inherited from a surrounding item scope.

(rust-analyzer should suggest removing an unused `use`-declaration as fix in either case.)

An important counter-example:

Filename: library/src/lib.rs

```rust
pub struct Type;
pub struct Generic<T>;

pub trait Trait {}
use impl Trait for Type {}

pub type Alias = Generic<Type>;
```

Filename: main.rs
```rust
use std::any::TypeId;

use library::{Alias, Generic, Type};

assert_ne!(TypeId::of::<Alias>(), TypeId::of::<Generic<Type>>());
```

Here, the scoped implementation `use impl Trait for Type {}` **is** accounted for as it is captured into the type identity of `Alias`.

Since `Alias` is exported, the compiler cannot determine within the library alone that the type identity is unobserved. If it can ensure that that is the case, a (different!) warning could in theory still be shown here.

### Global trait implementation available
[global-trait-implementation-available]: #global-trait-implementation-available

Scoped implementations and `use`-declarations of such receive a specific warning if only shadowing a global implementation that would fully cover them. This warning also informs about the origin of the global implementation, with a "defined here" marker if in the same workspace. This warning is not applied to scoped implementations that at least partially (in either sense) shadow another scoped implementation.

(Partial overlap with a shadowed scoped implementation should be enough to suppress this because setting the import up to be a precise subset could get complex fairly quickly. In theory just copying `where`-clauses is enough, but in practice the amount required could overall scale with the square of scoped implementation shadowing depth and some imports may even have to be duplicated.)

It would make sense to let the definitions and also alternatively specific global implementations of traits with high implementation stability requirements like `serde::{Deserialize, Serialize}` deactivate this warning too, so that the latter don't cause it on the respective covered scoped implementations.

### Self-referential bound of scoped implementation

```rust
trait Foo { }

use impl<T> Foo for T where T: Foo { }
            ---------       ^^^^^^
```

A Rust developer may write the above to mean 'this scoped implementation can only be used on types that already implement this trait' or 'this scoped implementation uses functionality of the shadowed implementation'. However, since scoped `impl Trait for Type` uses item scope rules, any shadowed implementation is functionally absent in the entire scope. As such, this implementation, like the equivalent global implementation, cannot apply to any types at all.

The warning should explain that and why the bound is impossible to satisfy.

### Private supertrait implementation required by public implementation
[private-supertrait-implementation-required-by-public-implementation]: #private-supertrait-implementation-required-by-public-implementation

Consider the following code:

```rust
pub struct Type;

use impl PartialEq for Type {
    // ...
}

pub use impl Eq for Type {}
```

Here, the public implementation relies strictly on the private implementation to also be available. This means it effectively cannot be imported in `use`-declarations outside this module.

See also the error [incompatible-or-missing-supertrait-implementation].

### Public implementation of private trait/on private type

The code

```rust
struct Type;
trait Trait {}

pub use impl Trait for Type {}
             ^^^^^     ^^^^
```

should produce two distinct warnings similarly to those for private items in public signatures, as the limited visibilities of `Type` and `Trait` independently prevent the implementation from being imported in modules for which it is declared as visible.

### Scoped implementation is less visible than item/field it is captured in
[scoped-implementation-is-less-visible-than-itemfield-it-is-captured-in]: #scoped-implementation-is-less-visible-than-itemfield-it-is-captured-in

The code

```rust
pub struct Type;
pub struct Generic<U, V>(U, V);

trait Trait {} // <-- Visibility of the trait doesn't matter for *this* warning.

use impl Trait for Type {}
-----------------------

pub type Alias = Generic<Type, Type>;
                         ^^^^  ^^^^

pub fn function(value: Generic<Type, Type>) -> Generic<Type, Type> {
                               ^^^^  ^^^^              ^^^^  ^^^^
    value
}

pub struct Struct {
  private: Generic<Type, Type>, // This is fine.
  pub public: Generic<Type, Type>,
                      ^^^^  ^^^^
}
```

should produce eight warnings (or four/three warnings with multiple primary spans each, if possible). The warning should explain that the type can't be referred to by fully specified name outside the crate/module and that the implementation may be callable from code outside the crate/module.

(If [explicit-binding] is added to the RFC and used in such a way, then the warning should show up on the `Trait in module` span instead.)

Note that as with other private-in-public warnings, replacing

```rust
use impl Trait for Type {}
```

with

```rust
mod nested {
    use super::{Trait, Type};
    pub use impl Trait for Type {}
}
use nested::{impl Trait for Type};
```

in the code sample above should silence the warning.

### Imported implementation is less visible than item/field it is captured in
[imported-implementation-is-less-visible-than-itemfield-it-is-captured-in]: #imported-implementation-is-less-visible-than-itemfield-it-is-captured-in

This occurs under the same circumstances as above, except that

```rust
trait Trait {}
use impl Trait for Type {}
```

is replaced with

```rust
use a_crate::{
    Trait,
    impl Trait for Type,
};
```

(where here the implementation import is subsetting a blanket import, but that technicality isn't relevant. What matters is that the implementation is from another crate).

If the imported implementation is captured in a public item's signature, that can accidentally create a public dependency. As such this should be a warning too (unless something from that crate occurs explicitly in that public signature or item?).

## Errors

### Global implementation of trait where global implementation of supertrait is shadowed

A trait cannot be implemented globally for a discrete type in a scope where the global implementation of any of its supertraits is shadowed on that type.

```rust
struct Type;

trait Super {}
trait Sub: Super {}

impl Super for Type {}

{
    use impl Super for Type {}
    ----------------------- // <-- Scoped implementation defined/imported here.

    impl Sub for Type {}
    ^^^^^^^^^^^^^^^^^ //<-- error: global implementation of trait where global implementation of supertrait is shadowed
}
```

### Negative scoped implementation
[negative-scoped-implementation]: #negative-scoped-implementation

This occurs on all negative scoped implementations. Negative scoped implementations can be parsed, but are rejected shortly after macros are applied.

```rust
struct Type;
trait Trait {}

impl Trait for Type {}

{
    use impl !Trait for Type {}
    ^^^^^^^^^^^^^^^^^^^^^^^^ error: negative scoped implementation
}
```

### Incompatible or missing supertrait implementation
[incompatible-or-missing-supertrait-implementation]: #incompatible-or-missing-supertrait-implementation

Implementations of traits on discrete types require a specific implementation of each of their supertraits, as they bind to them at their definition, so they cannot be used without those.

```rust
struct Type;
trait Super {}
trait Sub: Super {}

impl Super for Type {}

mod nested {
    pub use impl Super for Type {}
    pub use impl Sub for Type {}
}

use nested::{impl Sub for Type};
             ^^^^^^^^^^^^^^^^^ error: incompatible supertrait implementation
```

Rustc should suggest to import the required scoped implementation, if possible.

See also the warning [private-supertrait-implementation-required-by-public-implementation]. See also [implicit-import-of-supertrait-implementations-of-scoped-implementations-defined-on-discrete-types] for a potential way to improve the ergonomics here.

### Scoped implementation of external sealed trait
[scoped-implementation-of-external-sealed-trait]: #scoped-implementation-of-external-sealed-trait

Given crate `a`:

```rust
mod private {
    pub trait Sealing {}
}
use private::Sealing;

pub trait Sealed: Sealing {}

pub use impl<T> Sealed for T {} // Ok.
```

And crate `b`:

```rust
use a::{
    Sealed,
    impl Sealed for usize, // Ok.
};

use impl Sealed for () {} // Error.
         ^^^^^^
```

Crate `b` cannot define scoped implementations of the external sealed trait `Sealed`, but can still import them.

See [no-external-scoped-implementations-of-sealed-traits] for why this is necessary.

## Behaviour change/Warning: `TypeId` of implementation-aware generic discretised using generic type parameters
[behaviour-changewarning-typeid-of-implementation-aware-generic-discretised-using-generic-type-parameters]: #behaviour-changewarning-typeid-of-implementation-aware-generic-discretised-using-generic-type-parameters

As a result of the transmutation permission given in [layout-compatibility], which is needed to let the `ErasedHashSet` example in [typeid-of-generic-type-parameters-opaque-types] *remain sound*, monomorphisations of a function that observe distinct `TypeId`s for [implementation-aware-generics] they discretise using type parameters may be called on the same value instance.

Notably, this affects `TypeId::of::<Self>()` in implementations with most generic targets, but not in unspecific blanket implementations on the type parameter itself.

This would have to become a future incompatibility lint ahead of time, and should also remain a warning after the feature is implemented since the behaviour of `TypeId::of::<Self>()` in generics is likely to be unexpected.

In most cases, implementations should change this to `TypeId::of::<T>()`, where `T` is the type parameter used for discretisation, since that should show the expected `TypeId` distinction.

Instead of `TypeId::of::<AStruct<U, V, W>>()`, `TypeId::of::<(U, V, W)>()` can be used, as tuples are [implementation-invariant-generics].

## Resolution on generic type parameters
[resolution-on-generic-type-parameters]: #resolution-on-generic-type-parameters

Scoped `impl Trait for Type`s (including `use`-declarations) can be applied to outer generic type parameters *at least* (see [unresolved-questions]) via scoped blanket `use impl<T: Bound> Trait for T`.

However, a blanket implementation can only be bound on a generic type parameter iff its bounds are fully covered by the generic type parameter's bounds and other available trait implementations on the generic type parameter, in the same way as this applies for global implementations.

## Method resolution to scoped implementation without trait in scope

[Method calls] can bind to scoped implementations even when the declaring trait is not separately imported. For example:

```rust
struct Type;
struct Type2;

mod nested {
    trait Trait {
        fn method(&self) {}
    }
}

use impl nested::Trait for Type {}
impl nested::Trait for Type2 {}

Type.method(); // Compiles.
Type2.method(); // error[E0599]: no method named `method` found for struct `Type2` in the current scope
```

This also equally (importantly) applies to scoped implementations imported from elsewhere.

[Method calls]: https://doc.rust-lang.org/book/ch05-03-method-syntax.html#method-syntax

## Scoped implementations do not implicitly bring the trait into scope

This so that no method calls on other types become ambiguous:

```rust
struct Type;
struct Type2;

mod nested {
    trait Trait {
        fn method(&self) {}
    }

    trait Trait2 {
        fn method(&self) {}
    }
}

use nested::Trait2;
impl Trait2 for Type {}
impl Trait2 for Type2 {}

use impl nested::Trait for Type {}
impl nested::Trait for Type2 {}

Type.method(); // Compiles, binds to scoped implementation of `Trait`.
Type2.method(); // Compiles, binds to global implementation of `Trait2`.
```

(If `Trait` was not yet globally implemented for `Type2`, and `Trait` and `Type2` were defined in other crates, then bringing `Trait` into scope here could introduce instability towards that implementation later being added in one of those crates.)

## Shadowing with different bounds

Scoped implementations may have different bounds compared to an implementation they (partially) shadow. The compiler will attempt to satisfy those bounds, but if they are not satisfied, then the other implementation is not shadowed for that set of generic type parameters and no additional warning or error is raised.

(Warnings for e.g. unused scoped implementations and scoped implementations that only shadow a covering global implementation are still applied as normal. It's just that partial shadowing with different bounds is likely a common use-case in macros.)

```rust
struct Type1;
struct Type2;

trait Trait1 {
    fn trait1() {
        println!("1");
    }
}
impl<T> Trait1 for T {} // <--

trait Trait2 {
    fn trait2() {
        println!("2");
    }
}
impl Trait2 for Type2 {} // <--

trait Say {
    fn say();
}
impl<T: Trait1> Say for T
where
    T: Trait1, // <--
{
    fn say() {
        T::trait1();
    }
}

{
    use impl<T> Say for T
    where
        T: Trait2 // <--
    {
        fn say() {
            T::trait2();
        }
    }

    Type1::say(); // 1
    Type2::say(); // 2
}
```

## No priority over type-associated methods

Scoped `impl Trait for Type` has *the same* method resolution priority as an equivalent global implementation would have if it was visible for method-binding in that scope. This means that directly type-associated functions still bind with higher priority than those available through scoped implementations.

## Coercion to trait objects

Due to the coercion into a trait object in the following code, the scoped implementation becomes attached to the value through the pointer meta data. This means it can then be called from other scopes:

```rust
use std::fmt::{self, Display, Formatter};

fn function() -> &'static dyn Display {
    use impl Display for () {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "scoped")
        }
    }

    &()
}

println!("{}", function()); // "scoped"
```

This behaves exactly as a global implementation would.

Note that the [`DynMetadata<dyn Display>`]s of the reference returned above and one that uses the global implementation would compare as distinct even if both are "`&()`".

[`DynMetadata<dyn Display>`]: https://doc.rust-lang.org/stable/core/ptr/struct.DynMetadata.html

## Interaction with return-position `impl Trait`

Consider the following functions:

```rust
trait Trait {}

fn function() -> impl Trait {
    use impl Trait for () {}

    () // Binds on trailing `()`-expression.
}

fn function2() -> impl Trait {
    use impl Trait for () {}

    {} // Binds on trailing `{}`-block used as expression.
}
```

In this case, the returned opaque types use the respective inner scoped implementation, as it binds on the `()` expression.

These functions do not compile, as the implicitly returned `()` is not stated *inside* the scope where the implementation is available:

```rust
trait Trait {}

fn function() -> impl Trait {
                 ^^^^^^^^^^
    use impl Trait for () {}
    ---------------------

    // Cannot bind on implicit `()` returned by function body without trailing *Expression*.
}

fn function2() -> impl Trait {
                  ^^^^^^^^^^
    use impl Trait for () {}
    ---------------------

    return; // Cannot bind on `return` without expression.
    -------
}
```

(The errors should ideally also point at the scoped implementations here with a secondary highlight, and suggest stating the return value explicitly.)

The binding must be consistent:

```rust
trait Trait {}

fn function() -> impl Trait {
    // error: Inconsistent implementation of opaque return type.
    if true {
        use impl Trait for () {}
        return ();
        ----------
    } else {
        use impl Trait for () {}
        return ();
        ^^^^^^^^^^
    }
}
```

This function *does* compile, as the outer scoped `impl Trait for ()` is bound on the `if`-`else`-expression as a whole.

```rust
trait Trait {}

fn function() -> impl Trait {
    use impl Trait for () {}

    if true {
        use impl Trait for () {} // warning: unused scoped implementation
        ()
    } else {
        use impl Trait for () {} // warning: unused scoped implementation
        ()
    }
}
```

This compiles because the end of the function is not reachable:

```rust
trait Trait {}

fn function() -> impl Trait {
    {
        use impl Trait for () {}
        return (); // Explicit `return` is required to bind in the inner scope.
    }
}
```

## Static interception of dynamic calls

As a consequence of binding outside of generic contexts, it *is* possible to statically wrap *specific* trait implementations on *concrete* types. This includes the inherent implementations on trait objects:

```rust
use std::fmt::{self, Display, Formatter};

{
    use impl Display for dyn Display {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            // Restore binding to inherent global implementation within this function.
            use ::{impl Display for dyn Display};

            write!(f, "Hello! ")?;
            d.fmt(f)?;
            write!(f, " See you!")
        }
    }

    let question = "What's up?"; // &str
    println!("{question}"); // "What's up?"

    let question: &dyn Display = &question;
    println!("{question}"); // Binds to the scoped implementation; "Hello! What's up? See you!"
}
```

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

## First-party implementation assumptions in macros
[first-party-implementation-assumptions-in-macros]: #first-party-implementation-assumptions-in-macros

If a macro outputs a call of the form `<$crate::Type as $crate::Trait>::method()`, it can currently make safety-critical assumptions about implementation details of the `method` that is called iff implemented in the same crate.

(This should also be considered relevant for library/proc-macro crate pairs where the macro crate is considered an implementation detail of the library even where the macro doesn't require an `unsafe` token in its input, even though "crate privacy" currently isn't formally representable towards Cargo.)

As such, **newly allowing the global trait implementation to be shadowed here can introduce soundness holes** iff `Trait` is not `unsafe` or exempt from scoped implementations.

(I couldn't come up with a good example for this. There might be a slim chance that it's not actually a practical issue in the ecosystem. Unfortunately, this seems to be very difficult to lint for.)

There are a few ways to mitigate this, but they all have significant drawbacks:

- Opt-in scoped-`impl Trait` transparency for macros

  This would make scoped `impl Trait for Type`s much less useful, as they couldn't be used with for example some derive macros by default. It would also be necessary to teach the opt-in along with macros, which may not be realistic considering existing community-made macro primers.

  Implementation is likely complicated because many procedural macros emit tokens only with `Span::call_site()` hygiene, so information on the distinct binding site origin may not be readily available.

  This could be limited to existing kinds of macro definitions, so that future revised macro systems can be opted in by default. Future macros could use an `unsafe` trait instead to assume an implementation, or make use of scoped `impl Trait for Type` to enforce a specific implementation in their output.

  Drawback: Whether globally implemented behaviour can be changed by the consumer would depend on the macro. It would be good to surface a transparency opt-in in the documentation here.

- Opt-in scoped-`impl Trait` *priority* for macros

  This would preserve practical usefulness of the proposed feature in most cases.

  This would add significant complexity to the feature, as resolution of scoped implementations wouldn't be exactly the same as for other items. (We should otherwise warn if a scoped `impl Trait for Type` outside a macro shadows binding a global implementation inside of it though, so at least the feature implementation complexity may be net zero in this regard.)

  This could be limited to existing kinds of macro definitions, with the same implications as for opt-in transparency above.

  Drawback: Whether globally implemented behaviour can be changed by the consumer would depend on the macro. It would be good to surface a priority opt-in in the documentation here.

- Forbid scoped `impl Trait for Type` if `Trait` and `Type` are from the same crate

  This would at best be a partial fix and would block some interesting uses of [using-scoped-implementations-to-implement-external-traits-on-external-types].

## Unexpected behaviour of `TypeId::of::<Self>()` in implementations on generics in the consumer-side presence of scoped implementations and `transmute`

As explained in [layout-compatibility] and [type-identity-of-generic-types], an observed `TypeId` can change for an instance under specific circumstances that are previously-legal `transmute`s as e.g. for the `HashSet`s inside the type-erased value-keyed collection like the `ErasedHashSet` example in the [typeid-of-generic-type-parameters-opaque-types] section.

This use case appears to be niche enough in Rust to not have an obvious example on crates.io, but see [behaviour-changewarning-typeid-of-implementation-aware-generic-discretised-using-generic-type-parameters] for a lint that aims to mitigate issues in this regard and could be used to survey potential issues.

## More `use`-declaration clutter, potential inconsistencies between files

If many scoped implementations need to be imported, this could cause the list of `use`-declarations to become less readable. If there are multiple alternatives available, inconsistencies could sneak in between modules (especially if scoped `impl Trait for Type` is used in combination with [specialisation](https://rust-lang.github.io/rfcs/1210-impl-specialization.html)).

This can largely be mitigated by centralising a crate's scoped trait imports and implementations in one module, then wildcard-importing its items:

```rust
// lib.rs
mod scoped_impls;
use scoped_impls::*;
```

```rust
// scoped_impls.rs
use std::fmt::Debug;

use a::{TypeA, TraitA};
use b::{TypeB, TraitB};

pub use a_b_glue::{impl TraitA for TypeB, impl TraitB for TypeA};
// ...

pub use impl Debug for TypeA {
    // ...
}
pub use impl Debug for TypeB {
    // ...
}

// ...
```

```rust
// other .rs files
use crate::scoped_impls::*;
```

## Type inference has to consider both scoped and global implementations

Complexity aside, this could cause compiler performance issues since caching would be less helpful.

Fortunately, at least checking whether scoped implementations exist at all for a given trait and item scope should be reasonably inexpensive, so this hopefully won't noticeably slow down compilation of existing code.

That implementation environment binding on generic type parameters is centralised to the type discretisation site(s) may also help a little in this regard.

## Cost of additional monomorphised implementation instances

The additional instantiations of implementations resulting from  [binding-choice-by-implementations-bounds] could have a detrimental effect on compile times and .text size (depending on optimisations).

This isn't unusual for anything involving *GenericParams*, but use of this feature could act as a multiplier to some extent. It's likely a good idea to evaluate relatively fine-grained caching in this regard, if that isn't in place already.

## Split type identity may be unexpected
[split-type-identity-may-be-unexpected]: #split-type-identity-may-be-unexpected

Consider crates like `inventory` or Bevy's systems and queries.

There may be tricky to debug issues for their consumers if a `TypeId` doesn't match between uses of generics with superficially the same type parameters, especially without prior knowledge of distinction by captured *implementation environments*.

A partial mitigation would be to have rustc include captured scoped implementations on generic type parameters when printing types, but that wouldn't solve the issue entirely.

Note that with this RFC implemented, `TypeId` would still report the same value iff evaluated on generic type parameters with distinct but bound-irrelevant captured implementations directly, as long as only these top-level implementations differ and no nested captured *implementation environments* do.

## Marking a generic as implementation-invariant is a breaking change

This concerns the split of [implementation-aware-generics] and [implementation-invariant-generics].

"Implementation-aware" is the logic-safe default.

"Implementation-invariant" has better ergonomics in some cases.

It would be great to make moving from the default here only a feature addition. To do this, a new coherence rule would likely have to be introduced to make implementations conflict if any type becoming implementation-invariant would make them conflict, and additionally to make such implementations shadow each other (to avoid all-too-unexpected silent behaviour changes).

However, even that would not mitigate the behaviour change of type-erasing collections that are keyed on such generics that become type-invariant later, so making this a breaking change is simpler and overall more flexible.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Avoid newtypes' pain points

Alternative keywords: ergonomics and compatibility.

### Recursively dependent `#[derive(…)]`

Many derives, like `Clone`, `Debug`, partial comparisons, `serde::{Deserialize, Serialize}` and `bevy_reflect::{FromReflect, Reflect}` require the trait to be implemented for each field type. Even with the more common third-party traits like Serde's, there are many crates with useful data structures that do not implement these traits directly.

As such, glue code is necessary.

#### Current pattern

Some crates go out of their way to provide a compatibility mechanism for their derives, but this is neither the default nor has it (if available) any sort of consistency between crates, which means finding and interacting with these mechanisms requires studying the crate's documentation in detail.

For derives that do not provide such a mechanism, often only newtypes like `NewSerdeCompatible` and `NewNeitherCompatible` below can be used. However, these do not automatically forward all traits (and forwarding implementations may be considerably more painful than the `derive`s), so additional glue code between glue crates may be necessary.

```rust
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

use bevy_compatible::BevyCompatible;
use neither_compatible::NeitherCompatible;
use serde_compatible::SerdeCompatible;

// I could not actually find much information on how to implement the Bevy-glue.
// I assume it's possible to provide at least this API by creating a newtype and implementing the traits manually.
use bevy_compatible_serde_glue::BevyCompatibleDef;
use neither_compatible_bevy_glue::NewNeitherCompatible; // Assumed to have `From`, `Into` conversions.
use neither_compatible_serde_glue::NeitherCompatibleDef;
use serde_compatible_bevy_glue::NewSerdeCompatible; // Assumed to have `From`, `Into` conversions.

/// A typical data transfer object as it may appear in a service API.
#[derive(Deserialize, Serialize, Reflect)]
#[non_exhaustive] // Just a reminder, since the fields aren't public anyway.
pub struct DataBundle {
    // Serde provides a way to use external implementations on fields (but it has to be specified for each field separately).
    // Bevy does not have such a mechanism so far, so newtypes are required.
    // The newtypes should be an implementation detail, so the fields are (for consistency all) private.
    #[serde(with = "NewSerdeCompatibleDef")]
    serde: NewSerdeCompatible,
    #[serde(with = "BevyCompatibleDef")]
    bevy: BevyCompatible,
    #[serde(with = "NewNeitherCompatibleDef")]
    neither: NewNeitherCompatible,
}

// Some of the newtypes don't implement `Default` (maybe it was added to the underlying types later and the glue crate doesn't want to bump the dependency),
// so this has to be implemented semi-manually instead of using the `derive`-macro.
impl Default for DataBundle {
    fn default() -> Self {
        DataBundleParts::default().into()
    }
}

// If the Bevy glue doesn't forward the Serde implementations, this is necessary.
#[derive(Deserialize, Serialize)]
#[serde(remote = "NewSerdeCompatible")]
#[serde(transparent)]
struct NewSerdeCompatibleDef(SerdeCompatible);

// Same as above, but here the implementation is redirected to another glue crate.
#[derive(Deserialize, Serialize)]
#[serde(remote = "NewNeitherCompatible")]
#[serde(transparent)]
struct NewNeitherCompatibleDef(#[serde(with = "NeitherCompatibleDef")] NeitherCompatible);

impl DataBundle {
    // These conversions are associated functions for discoverability.
    pub fn from_parts(parts: DataBundleParts) -> Self {
        parts.into()
    }
    pub fn into_parts(self) -> DataBundleParts {
        self.into()
    }

    // Necessary to mutate multiple fields at once.
    pub fn parts_mut(&mut self) -> DataBundlePartsMut<'_> {
        DataBundlePartsMut {
            serde: &mut self.serde.0,
            bevy: &mut self.bevy,
            neither: &mut self.neither.0,
        }
    }

    // Accessors to the actual instances with the public types.
    pub fn serde(&self) -> &SerdeCompatible {
        &self.serde.0
    }
    pub fn serde_mut(&mut self) -> &mut SerdeCompatible {
        &mut self.serde.0
    }

    // This also uses an accessor just for consistency.
    pub fn bevy(&self) -> &BevyCompatible {
        &self.bevy
    }
    pub fn bevy_mut(&mut self) -> &mut BevyCompatible {
        &mut self.bevy
    }

    // More accessors.
    pub fn neither(&self) -> &NeitherCompatible {
        &self.neither.0
    }
    pub fn neither_mut(&mut self) -> &mut NeitherCompatible {
        &mut self.neither.0
    }
}

// Conversions for convenience
impl From<DataBundleParts> for DataBundle {
    fn from(value: DataBundleParts) -> Self {
        Self {
            serde: value.serde.into(),
            bevy: value.bevy.into(),
            neither: value.neither.into(),
        }
    }
}

impl From<DataBundle> for DataBundleParts {
    fn from(value: DataBundle) -> Self {
        Self {
            serde: value.serde.into(),
            bevy: value.bevy,
            neither: value.neither.into(),
        }
    }
}

/// Used to construct and destructure [`DataBundle`].
#[derive(Default)] // Assume that all the actual field types have useful defaults.
#[non_exhaustive]
pub struct DataBundleParts {
    pub serde: SerdeCompatible,
    pub bevy: BevyCompatible,
    pub neither: NeitherCompatible,
}

/// Return type of [`DataBundle::parts_mut`].
#[non_exhaustive]
pub struct DataBundlePartsMut<'a> {
    pub serde: &'a mut SerdeCompatible,
    pub bevy: &'a mut BevyCompatible,
    pub neither: &'a mut NeitherCompatible,
}
```

If two traits that require newtype wrappers need to be added for the same type, the process can be even more painful than what's shown above, involving `unsafe` reinterpret casts to borrow a wrapped value correctly as each newtype and forwarding-implementing each trait manually if no transparent derive is available.

#### With scoped `impl Trait for Type`

Scoped `impl Trait for Type` eliminates these issues, in a standardised way that doesn't require any special consideration from the trait or derive crates:

```rust
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

use bevy_compatible::BevyCompatible;
use neither_compatible::NeitherCompatible;
use serde_compatible::SerdeCompatible;

// I could not actually find much information on how to implement Bevy-glue.
// It's about the same as manually implementing the traits for newtypes, though.
// Since many traits are required for `bevy_reflect`'s derives, those glue crates use the prelude pattern and provide one for each target type.
use bevy_compatible_serde_glue::{
    impl Deserialize<'_> for BevyCompatible,
    impl Serialize for BevyCompatible,
};
use neither_compatible_bevy_glue::preludes::neither_compatible::*;
use neither_compatible_serde_glue::{
    impl Deserialize<'_> for NeitherCompatible,
    impl Serialize for NeitherCompatible,
};
use serde_compatible_bevy_glue::preludes::serde_compatible::*;

/// A typical data transfer object as it may appear in a service API.
#[derive(Default, Deserialize, Serialize, Reflect)]
#[non_exhaustive]
pub struct DataBundle {
    // Everything just works.
    pub serde: SerdeCompatible,
    pub bevy: BevyCompatible,
    pub neither: NeitherCompatible,
}

// `Default` was derived normally.
// No glue for the glue is necessary.
// No conversions are needed to construct or destructure.
// `&mut`-splitting is provided seamlessly by Rust.
// No accessors are needed since the fields are public.
```

Even in cases where the glue API cannot be removed, it's still possible to switch to this simplified, easier to consume implementation and deprecate the original indirect API.

Note that the imported scoped implementations are *not* visible in the public API here, since they do not appear on generic type parameters in public items. There may still be situations in which defining a type alias is necessary to keep some scoped implementations away from generic type parameters. For a possible future way to eliminate that remaining friction, see [explicit-binding] in the [future-possibilities] section below.

Unlike with external newtypes, there are no potential conflicts beyond overlapping imports and definitions in the same scope. These conflicts can *always* be resolved both without editing code elsewhere and without adding an additional implementation:

- either by narrowing a local blanket implementation,
- by narrowing a blanket implementation import to a subset of the external implementation,
- or at worst by moving a generic implementation into a submodule and importing it for discrete types.

### Error handling and conversions
[error-handling-and-conversions]: #error-handling-and-conversions

When implementing services, it's a common pattern to combine a framework that dictates function signatures with one or more unrelated middlewares that have their own return and error types. The example below is a very abridged example of this.

Note that in either version, the glue code may be project-specific. Glue code is *very slightly* more concise when implemented with scoped `impl Trait for Type`, as intermediary `struct` definitions and the resulting field access can be avoided.

#### Current pattern

```rust
// crate `service`

use framework::{Error, Returned};
use middleware_a::{fallible_a, Error as ErrorA};
use middleware_b::{fallible_b, Error as ErrorB};

use framework_middleware_a_glue::{IntoReturnedExt as _, NewErrorA};
use framework_middleware_b_glue::{IntoReturnedExt as _, NewErrorB};

pub fn a() -> Result<Returned, Error> {
    // A `try` block should work eventually, but it may be not much less verbose.
    Ok((|| -> Result<_, NewErrorA> {
        fallible_a()?;
        Ok(fallible_a()?)
    })()?
    .into_returned())
}

pub fn b() -> Result<Returned, Error> {
    // The same as above.
    Ok((|| -> Result<_, NewErrorB> {
        fallible_b()?;
        Ok(fallible_b()?)
    })()?
    .into_returned())
}

pub fn mixed(condition: bool) -> Result<Returned, Error> {
    // Neither 'NewError' type provided by third-party crates can be used directly here.
    Ok((move || -> Result<_, NewError> {
        Ok(if condition {
            fallible_b()?;
            fallible_a()?.into_returned()
        } else {
            fallible_a()?;
            fallible_b()?.into_returned()
        })
    })()?)
}

// Custom glue to connect all three errors:
struct NewError(Error);
impl From<NewError> for Error {
    fn from(value: NewError) -> Self {
        value.0
    }
}
impl From<ErrorA> for NewError {
    fn from(value: ErrorA) -> Self {
        let intermediate: NewErrorA = value.into();
        Self(intermediate.into())
    }
}
impl From<ErrorB> for NewError {
    fn from(value: ErrorB) -> Self {
        let intermediate: NewErrorB = value.into();
        Self(intermediate.into())
    }
}
```

```rust
use service::{a, b, mixed};

fn main() {
    framework::setup()
        .add_route("a", a)
        .add_route("b", b)
        .add_route("mixed", mixed)
        .build()
        .run();
}
```

#### With scoped `impl Trait for Type`

```rust
// crate `service`

// More concise, since middleware errors are used only once in imports.
use framework::{Error, Returned};
use middleware_a::fallible_a;
use middleware_b::fallible_b;

// Note: It is often better to import `impl Into` here over `impl From`,
//       since middleware types often don't appear in public signatures.
//
//       If the target type of the import must appear as type parameter in a public signature,
//       a module that is wildcard-imported into each function body can be used instead,
//       which would amount to 6 additional and 2 modified lines here.
//
//       This RFC includes a warning for unintentionally exposed scoped implementations.
use framework_middleware_a_glue::{
    impl Into<Returned> for middleware_a::Returned,
    impl Into<Error> for middleware_a::Error,
};
use framework_middleware_b_glue::{
    impl Into<Returned> for middleware_b::Returned,
    impl Into<Error> for middleware_b::Error,
};

pub fn a() -> Result<Returned, Error> {
    // It just works.
    fallible_a()?;
    Ok(fallible_a()?.into())
}

pub fn b() -> Result<Returned, Error> {
    // Here too.
    fallible_b()?;
    Ok(fallible_b()?.into())
}

pub fn mixed(condition: bool) -> Result<Returned, Error> {
    // This too just works, as conversions bind separately.
    Ok(if condition {
        fallible_b()?;
        fallible_a()?.into()
    } else {
        fallible_a()?;
        fallible_b()?.into()
    })
}

// No custom glue is necessary at all.
```

```rust
// Unchanged. No change in the API of `service`, either.

use service::{a, b, mixed};

fn main() {
    framework::setup()
        .add_route("a", a)
        .add_route("b", b)
        .add_route("mixed", mixed)
        .build()
        .run();
}
```

Note that to export *discrete* scoped `impl Into` in addition to their scoped `impl From`, the glue crates can use the following pattern, which discretises the global implementation and as such binds to each scoped `impl From` in the respective exported scoped `impl Into`:

```rust
pub use ::{
    impl Into<framework::Returned> for middleware_a::Returned,
    impl Into<framework::Error> for middleware_a::Error,
};
```

## Preserve coherence

### Cross-crate stability

With this RFC, scopes are a 'mini version' of the environment that global implementations exist in. As this environment is sealed within one scope, and not composed from multiple crates that may update independently, the *orphan rule* is not necessary.

*All other* coherence rules and (for exported implementations) rules for what is and is not a breaking change apply *within each scope exactly like for global implementations*. In particular:

- Blanket implementations like

  ```rust
  // (Does not compile!)

  use std::fmt::{Debug, LowerHex, Pointer};
  mod debug_by_lower_hex;

  use debug_by_lower_hex::{impl<T: LowerHex> Debug<T> for T}; // <--

  use impl<T: Pointer> Debug<T> for T { // <--
    // ...
  }
  ```

  still conflict regardless of actual implementations of `LowerHex` and `Pointer` because they may overlap later and

- because scoped implementation are *explicitly subset* where they are imported, *it is not a breaking change to widen an exported scoped implementation*.

  (This is part of the reason why scoped `impl Trait for Type`s are anonymous; names would make these imports more verbose rather than shorter, since the subsetting still needs to happen in every case.)

### Logical consistency
[logical-consistency]: #logical-consistency

Binding external top-level implementations to types is equivalent to using their public API in different ways, so no instance-associated consistency is expected here. Rather, values that are used in the same scope behave consistently with regard to that scope's visible implementations.

#### of generic collections
[of-generic-collections]: #of-generic-collections

Generics are trickier, as their instances often do expect trait implementations on generic type parameters that are consistent between uses but not necessarily declared as bounded on the struct definition itself.

This problem is solved by making the `impl`s available to each type parameter part of the the type identity of the discretised host generic, including a difference in `TypeId` there as with existing monomorphisation.

(See [type-parameters-capture-their-implementation-environment] and [type-identity-of-generic-types] in the [reference-level-explanation] above for more detailed information.)

Here is an example of how captured *implementation environments* safely flow across module boundaries, often seamlessly due to type inference:

```rust
pub mod a {
    // ⓐ == ◯

    use std::collections::HashSet;

    #[derive(PartialEq, Eq)]
    pub struct A;

    pub type HashSetA = HashSet<A>;
    pub fn aliased(_: HashSetA) {}
    pub fn discrete(_: HashSet<A>) {}
    pub fn generic<T>(_: HashSet<T>) {}
}

pub mod b {
    // ⓑ

    use std::{
        collections::HashSet,
        hash::{Hash, Hasher},
    };

    #[derive(PartialEq, Eq)]
    pub struct B;
    use impl Hash for B {
        fn hash<H: Hasher>(&self, _state: &mut H) {}
    }

    pub type HashSetB = HashSet<B>; // ⚠
    pub fn aliased(_: HashSetB) {}
    pub fn discrete(_: HashSet<B>) {} // ⚠
    pub fn generic<T>(_: HashSet<T>) {}
}

pub mod c {
    // ⓒ == ◯

    use std::collections::HashSet;

    #[derive(PartialEq, Eq, Hash)]
    pub struct C;

    pub type HashSetC = HashSet<C>;
    pub fn aliased(_: HashSetC) {}
    pub fn discrete(_: HashSet<C>) {}
    pub fn generic<T>(_: HashSet<T>) {}
}

pub mod d {
    // ⓓ

    use std::{
        collections::HashSet,
        hash::{Hash, Hasher},
        iter::once,
    };

    use super::{
        a::{self, A},
        b::{self, B},
        c::{self, C},
    };

    use impl Hash for A {
        fn hash<H: Hasher>(&self, _state: &mut H) {}
    }
    use impl Hash for B {
        fn hash<H: Hasher>(&self, _state: &mut H) {}
    }
    use impl Hash for C {
        fn hash<H: Hasher>(&self, _state: &mut H) {}
    }

    fn call_functions() {
        a::aliased(HashSet::new()); // ⓐ == ◯
        a::discrete(HashSet::new()); // ⓐ == ◯
        a::generic(HashSet::from_iter(once(A))); // ⊙ == ⓓ

        b::aliased(HashSet::from_iter(once(B))); // ⓑ
        b::discrete(HashSet::from_iter(once(B))); // ⓑ
        b::generic(HashSet::from_iter(once(B))); // ⊙ == ⓓ

        c::aliased(HashSet::from_iter(once(C))); // ⓒ == ◯
        c::discrete(HashSet::from_iter(once(C))); // ⓒ == ◯
        c::generic(HashSet::from_iter(once(C))); // ⊙ == ⓓ
    }
}

```

Note that the lines annotated with `// ⚠` produce a warning due to the lower visibility of the scoped implementation in `b`.

Circles denote *implementation environments*:

| | |
|-|-|
| ◯ | indistinct from global |
| ⓐ, ⓑ, ⓒ, ⓓ | respectively as in module `a`, `b`, `c`, `d` |
| ⊙ | caller-side |

The calls infer discrete `HashSet`s with different `Hash` implementations as follows:

| call in `call_functions` | `impl Hash` in | captured in/at | notes |
|-|-|-|-|
| `a::aliased` | - | `type` alias | The implementation cannot be 'inserted' into an already-specified type parameter, even if it is missing. |
| `a::discrete` | - | `fn` signature | See `a::aliased`. |
| `a::generic` | `d` | `once<T>`&nbsp;call | |
| `b::aliased` | `b` | `type` alias | |
| `b::discrete` | `b` | `fn` signature | |
| `b::generic` | `d` | `once<T>`&nbsp;call | `b`'s narrow implementation cannot bind to the opaque `T`. |
| `c::aliased` | `::` | `type` alias | Since the global implementation is visible in `c`. |
| `c::discrete` | `::` | `fn` signature | See `c::aliased`.
| `c::generic` | `d` | `once<T>`&nbsp;call | The narrow global implementation cannot bind to the opaque `T`. |

#### of type-erased collections
[of-type-erased-collections]: #of-type-erased-collections

Type-erased collections such as the `ErasedHashSet` shown in [typeid-of-generic-type-parameters-opaque-types] require slightly looser behaviour, as they are expected to mix instances between environments where only irrelevant implementations differ (since they don't prevent this mixing statically like `std::collections::HashSet`, as their generic type parameters are transient on their methods).

It is for this reason that the `TypeId` of generic type parameters disregards bounds-irrelevant implementations.

The example is similar to the previous one, but `aliased` has been removed since it continues to behave the same as `discrete`. A new set of functions `bounded` is added:

```rust
#![allow(unused_must_use)] // For the `TypeId::…` lines.

trait Trait {}

pub mod a {
    // ⓐ == ◯

    use std::{collections::HashSet, hash::Hash};

    #[derive(PartialEq, Eq)]
    pub struct A;

    pub fn discrete(_: HashSet<A>) {
        TypeId::of::<HashSet<A>>(); // ❶
        TypeId::of::<A>(); // ❷
    }
    pub fn generic<T: 'static>(_: HashSet<T>) {
        TypeId::of::<HashSet<T>>(); // ❶
        TypeId::of::<T>(); // ❷
    }
    pub fn bounded<T: Hash + 'static>(_: HashSet<T>) {
        TypeId::of::<HashSet<T>>(); // ❶
        TypeId::of::<T>(); // ❷
    }
}

pub mod b {
    // ⓑ

    use std::{
        collections::HashSet,
        hash::{Hash, Hasher},
    };

    use super::Trait;

    #[derive(PartialEq, Eq)]
    pub struct B;
    use impl Hash for B {
        fn hash<H: Hasher>(&self, _state: &mut H) {}
    }
    use impl Trait for B {}

    pub fn discrete(_: HashSet<B>) { // ⚠⚠
        TypeId::of::<HashSet<B>>(); // ❶
        TypeId::of::<B>(); // ❷
    }
    pub fn generic<T: 'static>(_: HashSet<T>) {
        TypeId::of::<HashSet<T>>(); // ❶
        TypeId::of::<T>(); // ❷
    }
    pub fn bounded<T: Hash + 'static>(_: HashSet<T>) {
        TypeId::of::<HashSet<T>>(); // ❶
        TypeId::of::<T>(); // ❷
    }
}

pub mod c {
    // ⓒ == ◯

    use std::{collections::HashSet, hash::Hash};

    use super::Trait;

    #[derive(PartialEq, Eq, Hash)]
    pub struct C;
    impl Trait for C {}

    pub fn discrete(_: HashSet<C>) {
        TypeId::of::<HashSet<C>>(); // ❶
        TypeId::of::<C>(); // ❷
    }
    pub fn generic<T: 'static>(_: HashSet<T>) {
        TypeId::of::<HashSet<T>>(); // ❶
        TypeId::of::<T>(); // ❷
    }
    pub fn bounded<T: Hash + 'static>(_: HashSet<T>) {
        TypeId::of::<HashSet<T>>(); // ❶
        TypeId::of::<T>(); // ❷
    }
}

pub mod d {
    // ⓓ

    use std::{
        collections::HashSet,
        hash::{Hash, Hasher},
        iter::once,
    };

    use super::{
        a::{self, A},
        b::{self, B},
        c::{self, C},
        Trait,
    };

    use impl Hash for A {
        fn hash<H: Hasher>(&self, _state: &mut H) {}
    }
    use impl Hash for B {
        fn hash<H: Hasher>(&self, _state: &mut H) {}
    }
    use impl Hash for C {
        fn hash<H: Hasher>(&self, _state: &mut H) {}
    }

    use impl Trait for A {}
    use impl Trait for B {}
    use impl Trait for C {}

    fn call_functions() {
        a::discrete(HashSet::new()); // ⓐ == ◯
        a::generic(HashSet::from_iter(once(A))); // ⊙ == ⓓ
        a::bounded(HashSet::from_iter(once(A))); // ⊙ == ⓓ

        b::discrete(HashSet::from_iter(once(B))); // ⓑ
        b::generic(HashSet::from_iter(once(B))); // ⊙ == ⓓ
        b::bounded(HashSet::from_iter(once(B))); // ⊙ == ⓓ

        c::discrete(HashSet::from_iter(once(C))); // ⓒ == ◯
        c::generic(HashSet::from_iter(once(C))); // ⊙ == ⓓ
        c::bounded(HashSet::from_iter(once(C))); // ⊙ == ⓓ
    }
}

```

`// ⚠` and non-digit circles have the same meanings as above.

The following table describes how the types are observed at runtime in the lines marked with ❶ and ❷. It borrows some syntax from [explicit-binding] to express this clearly, but **denotes types as if seen from the global *implementation environment***.

| within function<br>(called by `call_functions`) | ❶ (collection) | ❷ (item) |
|-|-|-|
| `a::discrete` | `HashSet<A>` | `A` |
| `a::generic` | `HashSet<A: Hash in d + Trait in d>` | `A` |
| `a::bounded` | `HashSet<A: Hash in d + Trait in d>` | `A` ∘ `Hash in d` |
| `b::discrete` | `HashSet<B: Hash in `***`b`***` + Trait in`***` b`***`>` | `B` |
| `b::generic` | `HashSet<B: Hash in d + Trait in d>` | `B` |
| `b::bounded` | `HashSet<B: Hash in d + Trait in d>` | `B` ∘ `Hash in d` |
| `c::discrete` | `HashSet<C>` | `C` |
| `c::generic` | `HashSet<C: Hash in d + Trait in d>` | `C` |
| `c::bounded` | `HashSet<C: Hash in d + Trait in d>` | `C` ∘ `Hash in d` |

The combination ∘ is not directly expressible in `TypeId::of::<>` calls (as even a direct top-level annotation would be ignored without bounds). Rather, it represents an observation like this:

```rust
{
    use std::{any::TypeId, hash::Hash};

    use a::A;
    use d::{impl Hash for A};

    fn observe<T: Hash + 'static>() {
        TypeId::of::<T>(); // '`A` ∘ `Hash in d`'
    }

    observe::<A>();
}
```

##### with multiple erased type parameters

By replacing the lines

```rust
TypeId::of::<HashSet<T>>(); // ❶
TypeId::of::<T>(); // ❷
```

with

```rust
TypeId::of::<HashSet<(T,)>>(); // ❶
TypeId::of::<(T)>(); // ❷
```

(and analogous inside the discrete functions), the `TypeId` table above changes as follows:

| within function<br>(called by `call_functions`) | ❶ (collection) | ❷ (item) |
|-|-|-|
| `a::discrete` | `HashSet<(A,)>` | `(A,)` |
| `a::generic` | `HashSet<(A: Hash in d + Trait in d,)>` | `(A,)` |
| `a::bounded` | `HashSet<(A: Hash in d + Trait in d,)>` | `(A` ∘ `Hash in d,)` |
| `b::discrete` | `HashSet<(B: Hash in `***`b`***` + Trait in`***` b`***`,)>` | `(B,)` |
| `b::generic` | `HashSet<(B: Hash in d + Trait in d,)>` | `(B,)` |
| `b::bounded` | `HashSet<(B: Hash in d + Trait in d,)>` | `(B` ∘ `Hash in d,)` |
| `c::discrete` | `HashSet<(C,)>` | `(C,)` |
| `c::generic` | `HashSet<(C: Hash in d + Trait in d,)>` | `(C,)` |
| `c::bounded` | `HashSet<(C: Hash in d + Trait in d,)>` | `(C` ∘ `Hash in d,)` |

As you can see, the type identity of the tuples appears distinct when contributing to an implementation-aware generic's type identity but (along with the `TypeId`) remains appropriately fuzzy when used alone.

This scales up to any number of type parameters used in implementation-invariant generics, which means an efficient `ErasedHashMap<S: BuildHasher>` can be constructed by keying storage on the `TypeId::of::<(K, V)>()` where `K: Hash + Eq` and `V` are the generic type parameters of its functions.

### Logical stability

- Non-breaking changes to external crates cannot change the meaning of the program.
- Breaking changes should result in compile-time errors rather than a behaviour change.

This is another consequence of subsetting rather than named-model imports, as narrowing a scoped implementation can only make the `use`-declaration fail to compile, rather than changing which implementations are shadowed.

Similarly, types of generics with different captured *implementation environments* are strictly distinct from each other, so that assigning them inconsistently does not compile. This is weighed somewhat against ease of refactoring, so in cases where a type parameter is inferred and the host is used in isolation, which are assumed to not care about implementation details like that, the code will continue to align with the definition instead of breaking.

## Encourage readable code

This RFC aims to further decrease the mental workload required for code review, by standardising glue code APIs to some degree and by clarifying their use in other modules.

It also aims to create an import grammar that can be understood more intuitively than external newtypes when first encountered, which should improve the accessibility of Rust code somewhat.

### Clear imports

As scoped implementations bind implicitly like global ones, two aspects must be immediately clear at a glace:

- *Which trait* is implemented?
- *Which type* is targeted?

Restating this information in the `use`-declaration means that it is available without leaving the current file, in plaintext without any tooling assists. This is another improvement compared to newtypes or external definitions, where the relationship may not be immediately clear depending on their names.

Spelling scoped implementation imports out with keywords rather than just symbols makes their purpose easy to guess for someone unfamiliar with the scoped `impl Trait for Type` feature, possibly even for most English-speaking developers unfamiliar with Rust.

This is also true for blanket imports with `where`, which remain easy to parse visually due to the surrounding braces:

```rust
use std::fmt::{Debug, Display, Pointer};

// `Debug` and `Display` all `Pointer`-likes as addresses.
// The `Display` import is different only to show the long form
// with `where`. It could be written like the `Debug` import.
use cross_formatting::by_pointer::{
    impl<T: Pointer> Debug for T,
    {impl<T> Display for T where T: Pointer},
};

println!("{:?}", &()); // For example: 0x7ff75584c360
println!("{}", &()); // For example: 0x7ff75584c360
```

### Familiar grammar

The grammar for scoped implementations differs from that for global implementations by only a prefixed `use` and an optional visibility. As such, it should be easy to parse for developers not yet familiar with scoped implementations specifically.

The clear prefix (starting with at least two keywords instead of one) should still be enough to distinguish scoped implementations at a glance from global ones.

The header (the part before the `{}` block) of global implementations is reused unchanged for scoped implementation imports, including all bounds specifications, so there is very little grammar to remember additionally in order to `use` scoped `impl Trait for Type`s.

In each case, the meaning of identical grammar elements lines up exactly - only their context and perspective vary due to immediately surrounding tokens.

(See [grammar-changes] for details.)

### Stop tokens for humans

When looking for the scoped implementation affecting a certain type, strict shadowing ensures that it is always the closest matching one that is effective.

As such, readers can stop scanning once they encounter a match, instead of checking the entire file's length for another implementation that may be present in the outermost scope.

Aside from *implementation environments* captured *inside* generics, scoped implementations cannot influence the behaviour of another file without being mentioned explicitly.

## Unblock ecosystem evolution
[unblock-ecosystem-evolution]: #unblock-ecosystem-evolution

As any number of scoped glue implementations can be applied directly to application code without additional compatibility shims, it becomes far easier to upgrade individual dependencies to their next major version. Compatibility with multiple versions of crates like Serde and `bevy_reflect` can be provided in parallel through officially supported glue crates.

Additionally, scoped implementations are actually *more* robust than newtypes regarding certain breaking changes:

A newtype that implements multiple traits could eventually gain a global blanket implementation of one of its traits for types that implement another of its traits, causing a conflict during the upgrade.

In the presence of an overlapping scoped `impl Trait for Type`, the new blanket implementation is just unambiguously shadowed where it would conflict, which means no change is necessary to preserve the code's behaviour. A [global-trait-implementation-available] warning is still shown where applicable to alert maintainers of new options they have.

(See also [glue-crate-suggestions] for possible future tooling related to this pattern.)

### Side-effect: Parallelise build plans (somewhat) more

Serde often takes a long time to build even without its macros. If another complex crate depends on it just to support its traits, this can significantly stretch the overall build time.

If glue code for 'overlay' features like Serde traits is provided in a separate crate, that incidentally helps to reduce that effect somewhat:

Since the glue forms a second dependency chain that normally only rejoins in application code, the often heavier core functionality of libraries can build in parallel to Serde and/or earlier glue. Since the glue chain is likely to be less code, it matters less for overall build time whether it has to wait for one or two large crates first.

## Provide opportunities for rich tooling

### Discovery of implementations

As scoped implementations clearly declare the link between the trait and type(s) they connect, tools like rust-analyzer are able to index them and suggest imports where needed, just like for global traits.

(At least when importing from another crate, the suggested import should be for a specific type or generic, even if the export in question is a blanket implementation. Other generics of the export can usually be preserved, though.)

### Discovery of the feature itself

In some cases (where a trait implementations cannot be found at all), tools can suggest creating a scoped implementation, unless adding it in that place would capture it as part of the *implementation environment* of a type parameter specified in an item definition visible outside the current crate.

That said, it would be great if rust-analyzer could detect and suggest/enable feature-gated global implementations to some extent, with higher priority than creating a new scoped implementation.

### Rich and familiar warnings and error messages

Since scoped implementations work much like global ones, many of the existing errors and warnings can be reused with at most small changes. This means that, as developers become more familiar with either category of trait-related issues, they learn how to fix them for global and scoped implementations at the same time.

The implementation of the errors and warnings in the compiler can also benefit from the existing work done for global implementations, or in some cases outright apply the same warning to both scoped and global implementations.

Since available-but-not-imported scoped implementations are easily discoverable by the compiler, they can be used to improve existing errors like *error[E0277]: the trait bound `[…]` is not satisfied* and *error[E0599]: no method named `[…]` found for struct `[…]` in the current scope* with quick-fix suggestions also for using an existing scoped implementation in at least some cases.

### Maintenance warnings for ecosystem evolution

Scoped `impl Trait for Type`s lead to better maintenance lints:

If a covering global implementation later becomes available through a dependency, a warning can be shown on the local trait implementation for review. (See [global-trait-implementation-available].)

In the long run, this can lead to less near-duplicated functionality in the dependency graph, which can lead to smaller executable sizes.

### Automatic documentation

Scoped implementations can be documented and appear as separate item category in rustdoc-generated pages.

Rustdoc should be able to detect and annotate captured scoped implementations in public signatures automatically. This, in addition to warnings, should be another tool to help avoid accidental exposure of scoped implementations.

Implementation origin and documentation could be surfaced by rust-analyzer in relevant places.

## Why specific [implementation-invariant-generics]?
[why-specific-implementation-invariant-generics]: #why-specific-implementation-invariant-generics

This is a *not entirely clean* ergonomics/stability trade-off, as well as a clean resolution path for [behaviour-changewarning-typeid-of-implementation-aware-generic-discretised-using-generic-type-parameters].

> It is also the roughest part of this proposal, in my eyes. If you have a better way of dealing with the aware/invariant distinction, please do suggest it!

The main issue is that generics in the Rust ecosystem do not declare which trait implementations on their type parameters need to be consistent during their instances' lifetime, if any, and that traits like `PartialOrd` that do provide logical consistency guarantees over time are not marked as such in a compiler-readable way.

Ignoring this and not having distinction of [implementation-aware-generics]' discretised variants would badly break logical consistency of generic collections like `BTreeSet<T>`, which relies on `Ord`-consistency to function.

On the other hand, certain types (e.g. references and (smart) pointers) that often wrap values in transit between modules *really* don't care about implementation consistency on these types. If these were distinct depending on available implementations on their values, it would create *considerable* friction while defining public APIs in the same scope as `struct` or `enum` definitions that require scoped implementations for `derive`s.

Drawing a line manually here is an attempt to un-break this *by default* for the most common cases while maintaining full compatibility with existing code and keeping awareness of scoped `impl Trait for Type` entirely optional for writing correct and user-friendly APIs.

As a concrete example, this ensures that `Box<dyn Future<Output = Result<(), Error>>>` is automatically interchangeable even if spelled out in the presence of scoped [error-handling-and-conversions] affecting `Error`, but that `BinaryHeap<Box<u8>>` and `BinaryHeap<Box<u8: PartialEq in reverse + Ord in reverse>>` don't mix.

Functions pointers and closure trait( object)s should probably be fairly easy to pass around, with their internally-used bindings being an implementation detail. Fortunately, the Rust ecosystem already uses more specific traits for most configuration for better logical safety, so it's likely not too messy to make these implementation-invariant.

Traits and trait objects cannot be implementation invariant (including for their associated types!) because it's possible to define `OrderedExtend` and `OrderedIterator` traits with logical consistency requirement on `Ord` between them.

## Efficient compilation
[efficient-compilation]: #efficient-compilation

In theory, it should be possible to unify many instances of generic functions that may be polymorphic under this proposal cheaply before code generation. (Very few previously discrete implementations become polymorphic under scoped `impl Trait for Type`.)

This is mainly an effect of [layout-compatibility] and [binding-choice-by-implementations-bounds], so that, where the differences are only bounds-irrelevant, generated implementations are easily identical in almost all cases. The exception here are [implementation-aware-generics]' `TypeId`s (see also [typeid-of-generic-type-parameters-opaque-types]). Checking for this exception should be cheap if done alongside checks for e.g. function non-constness if possible, which propagates identically from callee to caller.

Given equal usage, compiling code that uses scoped implementations could as such be slightly more efficient compared to use of newtypes and the resulting text size may be slightly smaller in some cases where newtype implementations are inlined differently.

The compiler should treat implementations of the same empty trait on the same type as identical early on, so that no code generation is unnecessarily duplicated. However, unrelated empty-trait implementations must still result in distinct `TypeId`s when captured in a generic type parameter and observed there by a `where`-clause or through nesting in an implementation-aware generic.

## Alternatives

### Named implementations

Named implementations/models could be used to more-easily use potentially conflicting implementations in the same scope, but in exchange they would have to always be bound explicitly, which would likely hinder use outside of `derive`s and generics to an inconvenient level.

Additionally, the use of named implementations is not as obvious as stating the origin-trait-type triple in close proximity.

Scoped `impl Trait for Type` would not require proper-named models for later [explicit-binding], as the module already uniquely identifies an implementation for each type-trait combination.

(See also [prior-art]: [lightweight-flexible-object-oriented-generics].)

### Weakening coherence rules

There is likely still some leeway here before the Rust ecosystem becomes brittle, but at least the orphan rule specifically is essential for ensuring that global trait implementations do not lead to hard ecosystem splits due to strictly incompatible framework crates.

If *other* coherence rules are relaxed, scoped `impl Trait for Type` also benefits immediately since it is subject to all of them.

### Crate-private implementations as distinct feature

There is a previous [RFC: Hidden trait implementations] from 2018-2021 where the result was general acceptance, but postponement for logistical reasons.

Scoped `impl Trait for Type` together with its warnings [scoped-implementation-is-less-visible-than-itemfield-it-is-captured-in] and [imported-implementation-is-less-visible-than-itemfield-it-is-captured-in] can mostly cover this use-case, though with slightly more boilerplate (`use`-declarations) and not as-strict a limitation.

[RFC: Hidden trait implementations]: https://github.com/rust-lang/rfcs/pull/2529

### Required-explicit binding of scoped implementations inside generics

This could avoid the distinction between [implementation-aware-generics] and [implementation-invariant-generics] to some extent, at the cost of likely overall worse ergonomics when working with scoped implementations.

It's also likely to make `derive`-compatibility of scoped implementations inconsistent, because some macros may require explicit binding on field types while others would not.

# Prior art
[prior-art]: #prior-art

## Lightweight, Flexible Object-Oriented Generics
[lightweight-flexible-object-oriented-generics]: #lightweight-flexible-object-oriented-generics

Yizhou Zhang, Matthew Loring, Guido Salvaneschi, Barbara Liskov and Andrew C. Myers, May 2015

<https://www.cs.cornell.edu/andru/papers/genus/>

There are some parallels between Genus's models and the scoped `impl Trait for Type`s proposed in this RFC, but for the most part they are quite distinct due to Rust's existing features:

| Genus | scoped `impl Trait for Type` | reasoning |
|---|---|---|
| Proper-named models | Anonymous scoped implementations | Use of existing coherence constraints for validation. Forced subsetting in `use`-declarations improves stability. The `impl Trait for Type` syntax stands out in `use`-declarations and is intuitively readable. |
| Explicit bindings of non-default models | Only implicit bindings | Focus on simplicity. Mixed bindings for definitions with the same scope/type/trait triple are rare and can be emulated with newtypes where needed. More natural use with future specialisation. |
| Comparing containers inherently constrain type parameters in their type definition. | Available scoped implementations for discretised type parameters become part of the type identity. | <p>This is a tradeoff towards integration with Rust's ecosystem, as generics are generally not inherently bounded on collection types in Rust.</p><p>There is likely some friction here with APIs that make use of runtime type identity. See [split-type-identity-may-be-unexpected].</p> |

Some features are largely equivalent:

| Genus | Rust (*without* scoped `impl Trait for Type`) | notes / scoped `impl Trait for Type` |
|---|---|---|
| Implicitly created default models | Explicit global trait implementations | Duck-typed implementation of unknown external traits is unnecessary since third party crates' implementations are as conveniently usable in scope as if global. |
| Runtime model information / Wildcard models | Trait objects | Scoped implementations can be captured in trait objects, and the `TypeId` of generic type parameters can be examined. This does not allow for invisible runtime specialisation in all cases. |
| Bindings [only for inherent constraints on generic type parameters?] are part of type identity | not applicable | <p>Available implementations on type parameters of discretised implementation-aware generics are part of the type identity. Top-level bindings are not.</p><p>Genus's approach provides better remote-access ergonomics than 𝒢's and great robustness when moving instances through complex code, so it should be available. Fortunately, the existing style of generic implementations in Rust can simply be monomorphised accordingly, and existing reflexive blanket conversions and comparisons can bind regardless of a type parameter's captured *implementation environment*.</p><p>However, typical Rust code also very heavily uses generics like references and closures to represent values passed through crate boundaries. To keep friction acceptably low by default, specific utility types are exempt from capturing implementation environments in their type parameters.</p> |

## A Language for Generic Programming in the Large

Jeremy G. Siek, Andrew Lumsdaine, 2007

<https://arxiv.org/abs/0708.2255>

𝒢 and scoped `impl Trait for Type` are conceptually very similar, though this RFC additionally solves logical consistency issues that arise from having multiple alternative ways to fulfill a constraint and develops some ideas further than the paper. Other differences are largely due to 𝒢 being more C++-like while scoped `impl Trait for Type` attempts smooth integration with all relevant Rust language features.

A few notable similarities, in the paper's words:

- equivalent retroactive modeling (where existing Rust's is limited by orphan rules),
- (retained) separate compilation (though *some* information can flow between items in this RFC, but only where such information flows already exist in Rust currently),
- lexically scoped models,
- seemingly the same binding rules on generic type parameters within constrained models/generic implementations,

and key differences:

| 𝒢 | Rust / scoped `impl Trait for Type` | notes |
|---|---|---|
| Only discrete model imports | Includes generic imports and re-exports | This is pointed out as '[left] for future work' in the paper. Here, it follows directly from the syntax combination of Rust's `use` and `impl Trait for Type` items. |
| - | (Rust) Global implementations | The automatic availability of global implementations between separately imported traits and types offers more convenience especially when working with common traits, like those backing operators in Rust. |
| Model overloading, mixed into nested scopes | Strict shadowing | Strict shadowing is easier to reason about for developers (especially when writing macros!), as the search stops at the nearest matching implementation.<br>See Rust's trait method resolution behaviour and [interaction-with-specialisation] for how this is still practically compatible with a form of overload resolution.<br>See [scoped-fallback-implementations] for a possible future way to better enable adaptive behaviour in macro output. |
| - | (Rust) Trait objects | 𝒢 does not appear to support runtime polymorphism beyond function pointers. Scoped `impl Trait for Type` is seamlessly compatible with `dyn Trait` coercions (iff `Trait` is object-safe). |
| (unclear?) | Available implementations on discretised type parameters become part of the type identity of implementation-aware generics. | <p>This allows code elsewhere to access scoped implementations that are already available at the definition site, and leads to overall more semantically consistent behaviour.</p><p>The tradeoff is that it may be difficult to explicitly annotate types in cases of mixed bindings with this RFC. As newtypes and named configuration token types are still preferred for changed behaviour, such cases will hopefully be limited. Otherwise, see [explicit-binding] for bikeshedded syntax.</p> |

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- I'm not too sure about the "global" wording. *Technically* that implementation isn't available for method calls unless the trait is in scope... though it is available when resolving generics. Maybe "unscoped" is better?

- In macros, which function-call token should provide the resolution context from where to look for scoped `impl Trait for Type`s (in all possible cases)?

  This doesn't matter for `Span::call_site()` vs. `Span::mixed_site()` since scoped implementations would resolve transparently through both, but it does matter for `Span::def_site()` which should exclude them.

  It very much does matter if one of the opt-in mitigations for [first-party-implementation-assumptions-in-macros] is implemented.

- Should outer generic type parameters be visible on/in scoped `impl Trait for Type`, including `use`-declarations?

  That would enable the following pattern:

  ```rust
  use some_crate::Trait;

  fn function<T>(value: T) -> impl Trait {
      use impl Trait for T {
          // ...
      }

      #[derive(Trait)] // Based on fields' `: Trait`.
      struct Returned<T> {
          field: T,
      }

      Returned { field: value }
  }
  ```

  However, if [explicit-binding] is added then that is unnecessary, as the following would work:

  ```rust
  use some_crate::Trait;

  fn function<T>(value: T) -> impl Trait {
      mod scoped {
        use impl<T> some_crate::Trait for T {
            // ...
        }
      }

      #[derive(Trait)] // Based on fields' `: Trait`.
      struct Returned<T> {
          field: T,
      }

      Returned::<T: Trait in scoped> { field: value }
  }
  ```

## Which `struct`s should be implementation-invariant?
[which-structs-should-be-implementation-invariant]: #which-structs-should-be-implementation-invariant

This is a tough question because, runtime behaviour difference of [of-type-erased-collections] aside, the following makes shifting a type from [implementation-aware-generics] to [implementation-invariant-generics] a compilation-breaking change:

```rust
struct Type;
struct Generic<T>(T);
trait Trait {}

mod a {
    use super::{Type, Generic, Trait};
    pub use impl Trait for Type {}
    pub type Alias = Generic<T>;
}

mod b {
    use super::{Type, Generic, Trait};
    pub use impl Trait for Type {}
    pub type Alias = Generic<T>;
}

use impl Trait for a::Alias {}
use impl Trait for b::Alias {}
```

(It is *theoretically* possible to do such a later adjustment as part of an edition, even considering `TypeId` behaviour I think, but it's certainly not pretty.)

Splitting this along the line of "structs that use `<>` around type parameters" would feel cleaner, but the basic smart pointers, `Pin<P>`, `Option<T>` and `Result<T, E>` appear in crate API signatures enough that not including them would create considerable friction.

Other candidates for consideration:

- Other `DispatchFromDyn` types in the standard library like `Cell`, `SyncUnsafeCell`, `UnsafeCell`

# Future possibilities
[future-possibilities]: #future-possibilities

## Exporting a scoped implementation as global, `extern impl Trait`

***This should never be used for IO/serialisation traits.***

Application crates may want to provide a specific implementation globally, disregarding orphan rules since there are no downstream crates that could be impacted by future incompatibilities (and crate-local issues are largely mitigated by *Cargo.lock*).

This could later be allowed using a construct like

```rust
// Use an external implementation as global:
#[core::unstable_use_as_global]
use impl_crate::{impl Trait for Type};

// Provide a local implementation globally:
#[core::unstable_use_as_global]
use impl Trait for Type { /*...*/ }
```

To use a global implementation not available through one of its dependencies, a library crate would have to declare it:

```rust
extern impl Trait for Type;
```

This would result in a compile error if the declaration is not fully covered by a global trait implementation.

If the trait implementation is later made available plainly (that is: without `use`, subject to orphan rules) by a dependency, a warning should appear on the `extern impl` declaration, along with the suggestion to remove the `extern impl` item.

(However, I assume binding to implementations not-from dependencies or the same crate in this way has a lot of implications for code generation.)

There is previous discussion regarding a similar suggestion in a slightly different context: [[Pre-RFC] Forward impls](https://internals.rust-lang.org/t/pre-rfc-forward-impls/4628)  
Perhaps the downsides here could be mitigated by allowing `#[unstable_use_as_global]` very strictly only in application crates compiled with the `cargo --locked` flag.

## Scoped `impl Trait for Type` of auto traits, `Drop` and/or `Copy` with orphan rules

The crate in which a type is defined could in theory safely provide scoped implementations for it also for these traits.

- This is likely more complicated to implement than the scoped `impl Trait for Type`s proposed in this RFC, as these traits interact with more distinct systems.

- What would be the binding site of `Drop` in `let`-statements?

- This could interact with linear types, were those to be added later on.

  For example, database transactions could be opt-out linear by being `!Drop` globally but also having their crate provide a scoped `Drop` implementation that can be imported optionally to remove this restriction in a particular consumer scope.

## Scoped proxy implementations

In theory it *might* be possible to later add syntax to create an exported implementation that's *not in scope for itself*.

I'm **very** hesitant about this since doing so would allow transparent overrides of traits (i.e. proxying), which could be abused for JavaScript-style layered overrides through copy-pasting source code together to some extent.

## Analogous scoped `impl Type`

This could be considered as more-robust alternative to non-object-safe extension traits defined in third party crates.

A good example of this use case could be the [tap] crate, which provides generic extension methods applicable to *all* types, but where its use is *theoretically* vulnerable to instability regarding the addition of type-associated methods of the same name(s).

If instead of (or in addition to!) …:

```rust
// pipe.rs

pub trait Pipe {
    #[inline(always)]
    fn pipe<R>(self, func: impl FnOnce(Self) -> R) -> R
    where
        Self: Sized,
        R: Sized,
    {
        func(self)
    }

    // ...
}

impl<T> Pipe for T where T: ?Sized {}
```

…the extension could be defined as …:

```rust
pub use impl<T> T where T: ?Sized {
  #[inline(always)]
  fn pipe<R>(self, func: impl FnOnce(Self) -> R) -> R
  where
      Self: Sized,
      R: Sized,
  {
      func(self)
  }

  // ...
}
```

…then:

- The consumer crate could choose which types to import the extension for, weighing

  ```rust
  use tap::pipe::{impl Type1, impl Type2};
  ```

  against

  ```rust
  use tap::pipe::{impl<T> T where T: ?Sized};
  ```

- These *scoped extensions would shadow inherent type-associated items of the same name*, guaranteeing stability towards those being added.

  (This should come with some warning labels in the documentation for this feature, since *adding items to an existing public scoped extension* could be considered an easily-breaking change here.)

This has fewer benefits compared to scoped `impl Trait for Type`, but would still allow the use of such third-party extension APIs in library crates with very high stability requirements.

An open question here is whether (and how) to allow partially overlapping `use impl Type` in the same scope, in order to not shadow inherent associated items with ones that cannot be implemented for the given type.

- That could in theory be more convenient to use, but

- calls could be *subtly* inconsistent at the consumer side, i.e. accidentally calling an inherent method if a scoped extension method was expected and

- widening a public implementation to overlap more of another exported in the same module could break dependent crates if a wide blanket import applied to narrower extensions.

As such, *if* this feature was proposed and accepted at some point in the future, it would likely be a good idea to only allow non-overlapping implementations to be exported.

[tap]: https://crates.io/crates/tap

## Interaction with specialisation
[interaction-with-specialisation]: #interaction-with-specialisation

- Scoped `impl Trait for Type` can be used for consumer-side specialisation of traits for binding sites that are in item scope, by partially shadowing an outer scope's implementation.

  Note that this would **not** work on generic type parameters, as the selected implementation is controlled strictly by their bounds (See [resolution-on-generic-type-parameters].), but it would work in macros for the most part.

  This does not interact with [specialisation proper](https://rust-lang.github.io/rfcs/1210-impl-specialization.html), but rather is a distinct, less powerful mechanism. As such, it would not supersede specialisation.

- Scoped `impl Trait for Type` does not significantly interact with specialisation of global implementations.

  Any global specialisation would only be resolved once it's clear no scoped implementation applies.

- Specialisation could disambiguate scoped implementations which are provided (implemented or imported) in the same scope. For example,

  ```rust
  use dummy_debug::{impl<T> Debug for T};
  use debug_by_display::{impl<T: Display> Debug for T};
  use impl Debug for str {
      // ...
  }
  ```

  would then compile, in scope resolving `<str as Debug>` to the local implementation and otherwise binding `Debug` depending on whether `Display` is available at the binding site for each given type `T`.

  Local implementations do not necessarily have to be more specific compared to imported ones - in keeping with "this is the same as for global implementations", the way in which the scoped implementation is introduced to the scope should not matter to specialisation.

  **When importing scoped implementations from a module, specialisation should apply hierarchically.** First, the specificity of implementations is determined only by `use impl` implementations and `use`-declarations in the importing scope. If the trait bound binds to a `use`-declaration, then the actual implementation is chosen by specificity among those visible in the module they are imported from. If the chosen implementation there is an import, the process repeats for the next module. This ensures stability and coherence when published implementations are specialised in other modules.

  - I'm not sure how well this can be cached in the compiler for binding-sites in distinct scopes, unfortunately. Fortunately, specialisation of scoped `impl Trait for Type` does not seem like a blocker for specialisation of global trait implementations.

  - Should specialisation of scoped implementations require equal visibility? I think so, but this question also seems considerably out of scope for scoped `impl Trait as Type` as a feature itself.

## Scoped `impl Trait for Type` as associated item

Scoped `impl Trait for Type` could be allowed and used as associated non-object-safe item as follows:

```rust
trait OuterTrait {
    use impl Trait for Type;
}

fn function<T: OuterTrait>() {
    use T::{impl Trait for Type};
    // ...configured code...
}
```
```rust
impl OuterTrait for OtherType {
    // Or via `use`-declaration of scoped implementation(s) defined elsewhere!
    // Or specify that the global implementation is used (somehow)!
    use impl Trait for Type {
        // ...
    }
}

function::<OtherType>();
```

This would exactly supersede the following more verbose pattern enabled by this RFC:

```rust
trait OuterTrait {
    type Impl: ImplTraitFor<Type>;
}

trait ImplTraitFor<T: ?Sized> {
    // Copy of trait's associated items, but using `T` instead of the `Self` type and
    // e.g. a parameter named `this` in place of `self`-parameters.
}

fn function<T: OuterTrait>() {
    use impl Trait for Type {
        // Implement using `T::Impl`, associated item by associated item.
    }

    // ...configured code...
}
```

```rust
struct ImplTraitForType;
impl ImplTraitFor<Type> for ImplTraitForType {
    // Implement item-by-item, as existing scoped `impl Trait for Type` cannot be used here.
}

impl OuterTrait for OtherType {
    type Impl: ImplTraitFor<Type> = ImplTraitForType;
}

function::<OtherType>();
```

- *In theory* this could be made object-safe if the associated implementation belongs to an object-safe trait, but this would introduce much-more-implicit call indirection into Rust.

## Scoped fallback implementations
[scoped-fallback-implementations]: #scoped-fallback-implementations

A scoped fallback implementation could be allowed, for example by negatively bounding it *on the same trait* in the definition or import:

```rust
#[derive(Debug)]
struct Type1;

struct Type2;

{
    use debug_fallback::{impl<T> Debug for T where T: !Debug};

    dbg!(Type1); // Compiles, uses global implementation.
    dbg!(Type2); // Compiles, uses fallback implementation.
}
```

This would be a considerably less messy alternative to [autoref-] or [autoderef-specialisation] for macro authors.

Note that ideally, these fallback implementations would still be required to not potentially overlap with any other (plain or fallback) scoped implementation brought into that same scope.

[autoref-]: https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md
[autoderef-specialisation]: https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html

## Negative scoped implementations

It's technically possible to allow negative scoped implementations that only shadow the respective implementation from an outer scope. For example:

```rust
// signed-indexing/src/arrays/prelude.rs
use core::ops::Index;

pub use impl<T, const N: usize> !Index<usize> for [T; N] {}
pub use impl<T, const N: usize> Index<isize> for [T; N] {
    type Output = T;

    #[inline]
    #[track_caller]
    fn index(&self, index: isize) -> &T {
        match index {
            0.. => self[index as usize],
            ..=-1 => if let Some(index) = self.len().checked_add_signed(index) {
                self[index]
            } else {
                #[inline(never)]
                #[track_caller]
                fn out_of_bounds(len: usize, index: isize) -> ! {
                    panic!("Tried to index slice of length {len} with index {index}, which is too negative to index backwards here.");
                }

                out_of_bounds(self.len(), index);
            },
        }
    }
}
```

```rust
use signed_indexing::arrays::prelude::*;

let array = [1, 2, 3];

// Unambiguous:
let first = array[0];
let last = array[-1];
```

This is likely a rather niche use-case.

It could also be useful in the context of [scoped-fallback-implementations].

## Explicit binding
[explicit-binding]: #explicit-binding

It could be possible to explicitly state bindings. Here is an example:

```rust
use std::collections::BinaryHeap;

// Contains discrete implementations of `PartialOrd` and `Ord` that invert the comparison.
mod reverse;

// Uses whichever implementation is in scope.
let max_heap: BinaryHeap<u32> = [1, 3, 2, 4].into();

// Explicit binding. Requirements are like for a discrete import.
let min_heap: BinaryHeap<u32: PartialOrd in _ + Ord in reverse> = [1, 3, 2, 4].into();

while let Some(max) in max_heap.pop() {
  println!("{max}"); // 4, 3, 2, 1
}

while let Some(min) in min_heap.pop() {
  println!("{min}"); // 1, 2, 3, 4
}

// Uses whichever implementation is in scope.
dbg!(<u32 as Ord>::cmp(&1, &2)); // […] = Less

// Explicit binding. Requirements are like for a discrete import.
dbg!(<u32 as PartialOrd in reverse>::cmp(&1, &2)); // […] = Greater

// The previous example is syntactic sugar for general top-level binding:
dbg!(<(u32: PartialOrd in reverse) as PartialOrd>::cmp(&1, &2)); // […] = Greater

// The forms can be mixed to bind supertraits:
dbg!(<(u32: PartialOrd in _) as Ord in reverse>::cmp(&1, &2)); // […] = Greater

{
    let mut a = max_heap;
    let mut b = min_heap;

    // a.append(&mut b);
    //          ^^^^^^ error[E0308]: mismatched types
}
```

```rust
mod custom_defaults {
    use impl Default for &'static str {
        // ...
    }
}

#[derive(Default)]
pub struct Struct<'a> {
    pub a: (&'a str: Default in custom_defaults),

    // The custom `Default` is not captured here,
    // since it's not actually in scope.
    pub b: Vec<&'a str>,
}
```

This is of course syntax bikeshedding.

Specifying implementations on fields manually is a way to provide them only to `derive` and other attribute macros, as these top-level implementations do *not* bind to the type and as such are *not* used by code that doesn't restate the type explicitly. (The built-in macros should be defined as doing so from the get-go. Unfortunately, for other macros this is likely an optional implementation detail.)

Since the specified top-level implementation doesn't bind persistently inside `Struct`, the exported signature is just `struct Struct<'a> {pub a: &'a str, pub b: Vec<&'a str>}`.

Binding only `PartialEq` would still shadow the discrete global `Ord` implementation, so binding both is required.

As the scoped implementation of `Ord` in `reverse` is on a discrete type, it requires the specific supertrait implementation that is in scope for its definition. This should make it possible to infer the module here. (See also [implicit-import-of-supertrait-implementations-of-scoped-implementations-defined-on-discrete-types] below.)

Top-level bindings require parentheses. To explicitly bind a global implementation, `::` can be used in place of the module path.

For stability reasons (against relaxation of bounds) and because they matter for type identity, explicit bindings should be allowed where no matching bound is present, but should produce an 'unused' warning iff neither published nor used in the same crate (including for type identity distinction).

## Implicit import of supertrait implementations of scoped implementations defined on discrete types
[implicit-import-of-supertrait-implementations-of-scoped-implementations-defined-on-discrete-types]: #implicit-import-of-supertrait-implementations-of-scoped-implementations-defined-on-discrete-types

As subtype implementations defined on discrete types always require specific supertrait implementations, the import of these supertrait implementations could be made implicit.

This would also affect [explicit-binding], changing

```rust
let min_heap: BinaryHeap<u32: PartialOrd in _ + Ord in reverse> = [1, 3, 2, 4].into();
```

to

```rust
let min_heap: BinaryHeap<u32: Ord in reverse> = [1, 3, 2, 4].into();
```

and

```rust
// (How to specify a scoped implementation with supertraits?)
dbg!(<u32 as PartialOrd in reverse>::cmp(&1, &2)); // […] = Greater
```

to

```rust
dbg!(<u32 as Ord in reverse>::cmp(&1, &2)); // […] = Greater
```

The downside is that `use`-declarations would become less obvious. Implied supertrait implementation imports could be enabled only for [explicit-binding] to avoid this.

If this is added later than scoped `impl Trait for Type`, then private scoped implementations **must not** be implicitly exported through this mechanism. (It's likely a good idea to not allow that anyway, as it would be surprising.) Making previously crate-private implementations available that way could lead to unsoundness.

## Conversions where a generic only cares about specific bounds' consistency

With specialisation and more expressive bounds, an identity conversion like the following could be implemented:

```rust
// In the standard library.

use std::mem;

impl<T, U, S: BuildHasher> From<HashSet<T, S>> for HashSet<U, S>
where
    T: ?Hash + ?Eq, // Observe implementations without requiring them.
    U: ?Hash + ?Eq,
    T == U, // Comparison in terms of innate type identity and observed implementations.
{
    fn from(value: HashSet<T, S>) -> Self {
        unsafe {
            // SAFETY: This type requires only the `Hash` and `Eq` implementations to
            //         be consistent for correct function. All other implementations on
            //         generic type parameters may be exchanged freely.
            //         For the nested types this is an identity-transform, as guaranteed
            //         by `T == U` and the shared `S` which means the container is also
            //         guaranteed to be layout compatible.
            mem::transmute(value)
        }
    }
}
```

This could also enable adjusted borrowing:

```rust
// In the standard library.

use std::mem;

impl<T, S: BuildHasher> HashSet<T, S> {
    fn as_with_item_impl<U>(&self) -> HashSet<U, S>
    where
        T: ?Hash + ?Eq, // Observe implementations without requiring them.
        U: ?Hash + ?Eq,
        T == U, // Comparison in terms of innate type identity and observed implementations.
    {
        unsafe {
            // SAFETY: This type requires only the `Hash` and `Eq` implementations to
            //         be consistent for correct function. All other implementations on
            //         generic type parameters may be exchanged freely.
            //         For the nested types this is an identity-transform, as guaranteed
            //         by `T == U` and the shared `S` which means the container is also
            //         guaranteed to be layout compatible.
            &*(self as *const HashSet<T, S> as *const HashSet<U, S>)
        }
    }
}
```

(But at that point, it may be better to use something like an unsafe marker trait or unsafe trait with default implementations.)

## Sealed trait bounds

This is probably pretty strange, and may not be useful at all, but it likely doesn't hurt to mention this.

Consider [explicit-binding] in bounds like here:

```rust
use another_crate::{Trait, Type1, Type2};

pub fn function<T: Trait in self>() {}

pub use impl Trait for Type1 {}
pub use impl Trait for Type2 {}
```

With this construct, `function` could privately rely on implementation details of `Trait` on `Type1` and `Type2` without defining a new sealed wrapper trait. It also becomes possible to easily define multiple sealed sets of implementations this way, by defining modules that export them.

Overall this would act as a more-flexible but also more-explicit counterpart to sealed traits.

Iff the caller is allowed to use this function without restating the binding, then removing the scope would be a breaking change (as it is already with bindings captured on type parameters in public signatures, so that would be consistent for this syntactical shape).

Binding an implementation in a call as `function::<T: Trait in a>()` while it is constrained as `fn function<T: Trait in b>() { … }` MUST fail for distinct modules `a` and `b` even if the implementations are identical, as otherwise this would leak the implementation identity into the set of breaking changes.

> That convenience (automatically using the correct implementations even if not in scope) also really should exist only iff there already is robust, near-effortless tooling for importing existing scoped implementations where missing. Otherwise this features here *would* get (ab)used for convenience, which would almost certainly lead to painful overly sealed APIs.

## Glue crate suggestions
[glue-crate-suggestions]: #glue-crate-suggestions

If crates move some of their overlay features into glue crates, as explained in [unblock-ecosystem-evolution], it would be nice if they could suggest them if both they and e.g. Serde were `cargo add`ed to the project dependencies.

An example of what this could look like:

```toml
[package]
name = "my-crate"
version = "0.1.2"
edition = "2021"

[dependencies]
# none

[suggest-with.serde."1"]
my-crate_serde_glue = "0.1.0"

[suggest-with.bevy_reflect."0.11"]
my-crate_bevy_reflect_glue = "0.1.2"

[suggest-with.bevy_reflect."0.12"]
my-crate_bevy_reflect_glue = "0.2.1"
```

(This sketch doesn't take additional registries into account.)

Ideally, crates.io should only accept existing crates here (but with non-existing version numbers) and Cargo should by default validate compatibility where possible during `cargo publish`.

## Reusable limited-access APIs

Given a newtype of an unsized type, like

```rust
#[repr(transparent)]
pub struct MyStr(str);
```

for example, there is currently no safe-Rust way to convert between `&str` and `&MyStr` or `Box<MyStr>` and `Box<str>`, even though *in the current module which can see the field* this is guaranteed to be a sound operation.

One good reason for this is that there is no way to represent this relationship with a marker trait, since any global implementation of such a trait would give outside code to this conversion too.

With scoped `impl Trait for Type`, the code above could safely imply a marker implementation like the following in the same scope:

```rust
// Visibility matches newtype or single field, whichever is more narrow.

use unsafe impl Transparent<str> for MyStr {}
use unsafe impl Transparent<MyStr> for str {}
// Could symmetry be implied instead?
```

(`Transparent` can and should be globally reflexive.)

This would allow safe APIs with unlimited visibility like

```rust
pub fn cast<T: Transparent<U>, U>(value: T) -> U {
    unsafe {
        // SAFETY: This operation is guaranteed-safe by `Transparent`.
        std::mem::transmute(value)
    }
}
```

and

```rust
unsafe impl<T: Transparent<U>, U> Transparent<Box<U>> for Box<T> {}
unsafe impl<'a, T: Transparent<U>, U> Transparent<&'a U> for &'a T {}
unsafe impl<'a, T: Transparent<U>, U> Transparent<&'a mut U> for &'a mut T {}
```

which due to their bound would only be usable where the respective `T: Transparent<U>`-implementation is in scope, that is where by-value unwrapping-and-then-wrapping would be a safe operation (for `Sized` types in that position).

Overall, this would make unsized newtypes useful without `unsafe`, by providing a compiler-validated alternative to common reinterpret-casts in their implementation. The same likely also applies to certain optimisations for `Sized` that can't be done automatically for unwrap-then-wrap conversions as soon as a custom `Allocator` with possible side-effects is involved.

If a module wants to publish this marker globally, it can do so with a separate global implementation of the trait, which won't cause breakage. (As noted in [efficient-compilation], the compiler should treat implementations of empty traits as identical early on, so that no code generation is unnecessarily duplicated.)

> *Could* sharing pointers like `Arc` inherit this marker from their contents like `Box` could? I'm unsure. They probably *shouldn't* since doing this to exposed shared pointers could easily lead to hard-to-debug problems depending on drop order.
>
> A global
>
> ```rust
> unsafe impl<T: Transparent<U>, U> Transparent<UnsafeCell<U>> for UnsafeCell<T> {}
> ```
>
> should be unproblematic, but a global
>
> ```rust
> unsafe impl<T> Transparent<T> for UnsafeCell<T> {}
> ```
>
> (or vice versa) **must not** exist to allow the likely more useful implementations on `&`-like types.
