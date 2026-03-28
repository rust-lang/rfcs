- Feature Name: `associated_traits`
- Start Date: 2026-03-28
- RFC PR: [rust-lang/rfcs#3938](https://github.com/rust-lang/rfcs/pull/3938)
- Rust Issue: [rust-lang/rfcs#2190](https://github.com/rust-lang/rfcs/issues/2190)

## Summary
[summary]: #summary

Allow traits to declare **associated traits** — named trait constraints that are defined abstractly in a trait and given concrete values in implementations. Just as associated types let a trait abstract over *which type* is used, associated traits let a trait abstract over *which constraints* are imposed. This is the trait-level analog of associated types.

```rust
#![feature(associated_traits)]

// 1. Declare an associated trait in a trait definition.
trait Container {
    trait Elem;
    fn process<T: Self::Elem>(&self, item: T);
}

// 2. Each impl chooses the concrete constraints.
impl Container for MyContainer {
    trait Elem = Send + Clone;
    fn process<T: Self::Elem>(&self, item: T) { /* ... */ }
}

// 3. Generic code outside the impl uses `C::Elem` as a bound.
//    This is the primary use case: the caller is generic over
//    the constraints without knowing what they are.
fn process_item<C: Container, T: C::Elem>(container: &C, item: T) {
    container.process(item);
}

// 4. Fully-qualified syntax also works.
fn process_qualified<C: Container, T: <C as Container>::Elem>(container: &C, item: T) {
    container.process(item);
}
```

## Motivation
[motivation]: #motivation

### The problem

Rust programmers frequently need to write traits that are *generic over constraints*. Today, the only way to parameterize a trait over which bounds should apply to some type is to fix them at the trait definition site, or to duplicate the entire trait hierarchy for each set of constraints.

Consider a plugin framework:

```rust
// Without associated traits — constraints are baked in.
trait Plugin {
    fn run<T: Send + 'static>(&self, task: T);
}
```

Every implementor must accept `Send + 'static`, even if some plugin systems (e.g., single-threaded ones) don't need `Send`. The only workaround today is to duplicate the trait:

```rust
trait SendPlugin {
    fn run<T: Send + 'static>(&self, task: T);
}

trait LocalPlugin {
    fn run<T: 'static>(&self, task: T);
}
```

This duplication cascades through every consumer of the trait, every generic function, and every downstream crate.

### Use cases from the community

[rust-lang/rfcs#2190](https://github.com/rust-lang/rfcs/issues/2190) (76 👍, 12 👀) has collected real-world use cases since 2017. The most prominent:

**Async runtime agnosticism.** A runtime trait can abstract over whether futures must be `Send`:

```rust
trait Runtime {
    trait FutureConstraint;
    fn spawn<F: Future<Output = ()> + Self::FutureConstraint>(f: F);
}

impl Runtime for TokioRuntime {
    trait FutureConstraint = Send + 'static;
    // ...
}

impl Runtime for LocalRuntime {
    trait FutureConstraint = 'static;
    // ...
}
```

Today, the async ecosystem is split between `Send` and `!Send` runtimes, requiring parallel trait hierarchies and duplicated generic code.

**Type constructor families (higher-kinded types lite).** `PointerFamily` abstracts over `Arc` vs. `Rc` by parameterizing the element constraints:

```rust
trait PointerFamily {
    trait Bounds;
    type Pointer<T>;
}

impl PointerFamily for ArcFamily {
    trait Bounds = Send + Sync;
    type Pointer<T> = Arc<T>;
}

impl PointerFamily for RcFamily {
    trait Bounds = Clone;
    type Pointer<T> = Rc<T>;
}
```

**UI component frameworks.** An `Events` associated trait lets different component types declare which event traits are valid:

```rust
trait Component {
    type Props: Clone + 'static;
    trait Events;
    fn new(props: Self::Props) -> Self;
}
```

**Monitor/capability patterns.** A data structure constrains which kinds of monitors it accepts:

```rust
trait DataStructure {
    trait Mon: Monitor;
}
```

### Summary of benefits

- Eliminates trait duplication when constraints vary per implementation.
- Composes naturally with existing features: associated types, generic associated types, trait inheritance, `impl Trait`, UFCS.
- Directly addresses a long-standing community request (2017–present, 76+ upvotes).

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Declaring an associated trait

Inside a trait body, you can declare an associated trait using the `trait` keyword, mirroring how `type` declares an associated type:

```rust
trait Container {
    trait Elem;
}
```

This says: "every implementation of `Container` must specify what `Elem` means as a trait bound."

### Providing a value in an impl

In an `impl` block, you provide the associated trait's value using `trait Name = TraitBound;`:

```rust
impl Container for MyVec {
    trait Elem = Send + Clone;
}
```

The value can be any valid trait bound: a single trait, a compound bound (`Send + Clone`), a trait with associated type constraints (`IntoIterator<Item = i32>`), a lifetime bound (`'static`), or `?Sized`.

### Using an associated trait as a bound

Once declared, an associated trait can be used anywhere a regular trait bound is expected:

```rust
fn process<C: Container, T: C::Elem>(container: &C, item: T) {
    // T satisfies whatever Elem resolves to for this C.
}
```

This is the *primary use*: the caller sees `T: C::Elem`, and the trait solver resolves `C::Elem` to the concrete bounds from the impl (e.g., `Send + Clone` for `MyVec`).

### Declaration bounds (supertraits)

You can require that every implementation's value satisfies certain minimum bounds:

```rust
trait Processor {
    trait Constraint: Clone;  // Every impl's value must include Clone
}

impl Processor for MyProc {
    trait Constraint = Clone + Send;  // OK: Clone is satisfied
}

impl Processor for BadProc {
    trait Constraint = Send;  // ERROR: does not satisfy Clone
}
```

### Defaults

Associated traits can have defaults, just like associated types:

```rust
trait Runtime {
    trait FutureConstraint = Send;  // Default; most runtimes want Send
    fn spawn<F: Future<Output = ()> + Self::FutureConstraint>(f: F);
}

// TokioRuntime is happy with the default (Send).
impl Runtime for TokioRuntime {
    fn spawn<F: Future<Output = ()> + Self::FutureConstraint>(f: F) { /* ... */ }
}

// A single-threaded runtime overrides the default.
impl Runtime for LocalRuntime {
    trait FutureConstraint = 'static;  // No Send requirement
    fn spawn<F: Future<Output = ()> + Self::FutureConstraint>(f: F) { /* ... */ }
}
```

### Generic associated traits

Associated traits can have their own generic parameters, analogous to generic associated types:

```rust
trait Transform {
    trait Constraint<T: Clone>;
}

impl Transform for MyTransform {
    trait Constraint<T: Clone> = PartialEq<T>;
}
```

Where clauses are also supported:

```rust
trait Codec {
    trait Decode<T> where T: Send;
}
```

### UFCS disambiguation

When a type parameter is bounded by multiple traits that each define an associated trait with the same name, you can use fully-qualified syntax to disambiguate:

```rust
trait Readable { trait Constraint; }
trait Writable { trait Constraint; }

fn transfer<T: Readable + Writable, R: <T as Readable>::Constraint>(data: R) {}
```

### `impl Trait` syntax

Associated traits work with `impl Trait` in argument position:

```rust
trait Handler {
    trait Arg;
    fn handle(&self, arg: impl Self::Arg);
}
```

### Call-site value constraints

You can constrain an associated trait's value at the call site using `where` clause syntax:

```rust
fn print_element<C: Container, T: C::Elem>(x: T)
where
    C::Elem: Debug,  // The impl's value for Elem must include Debug
{
    println!("{:?}", x);
}
```

This is different from `T: C::Elem + Debug` (which constrains `T` independently). The value constraint `C::Elem: Debug` constrains the *Container's impl* — if the impl provides `trait Elem = Send` (no Debug), the call is rejected even if `T` happens to implement Debug.

### Where associated traits *cannot* appear

- **Type position**: `let x: T::Elem = ...` is an error — associated traits are constraints, not types.
- **`dyn` position**: `dyn T::Elem` is an error. Rust type-checks generic function bodies once, before monomorphization, so the set of traits behind `T::Elem` is not yet known. Since `dyn` types require a compile-time-known vtable layout, `dyn` with an associated trait that depends on a type parameter is fundamentally incompatible with Rust's generics model. This is the same reason `dyn T` where `T` is a type parameter is rejected.
- **Inherent impls**: `impl MyStruct { trait Foo = Send; }` is an error — associated traits only make sense in trait impls.

### Error messages

When the feature gate is absent:

```
error[E0658]: associated traits are experimental
  --> src/lib.rs:3:5
   |
3  |     trait Elem;
   |     ^^^^^^^^^^
   |
   = note: see issue #99999 for more information
   = help: add `#![feature(associated_traits)]` to the crate attributes
```

When an associated trait is used in type position:

```
error: expected type, found trait `Container::Elem`
  --> src/lib.rs:10:14
   |
10 |     let _x: T::Elem = todo!();
   |             ^^^^^^^ not a type; `Elem` is an associated trait
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Syntax

**Declaration** (in trait body):

```
trait Ident ;
trait Ident : Bounds ;
trait Ident = DefaultBounds ;
trait Ident : Bounds = DefaultBounds ;
trait Ident < GenericParams > ;
trait Ident < GenericParams > where WhereClauses ;
```

**Implementation** (in impl body):

```
trait Ident = Bounds ;
trait Ident < GenericParams > = Bounds ;
```

**Usage** (in bound position):

```
T: Container::Elem
T: <C as Container>::Elem
T: C::Elem<i32>
```

### AST representation

A new variant `AssocItemKind::Trait` is added to the AST, containing:
- `ident`: the name
- `generics`: generic parameters and where clauses
- `bounds`: declaration bounds (supertraits)
- `value`: the default or impl value (a list of `GenericBound`)
- `has_value`: whether a `= ...` value is present

### HIR representation

Two new variants:
- `TraitItemKind::Trait(GenericBounds)` — in trait definitions
- `ImplItemKind::Trait(GenericBounds)` — in trait implementations

A new HIR bound variant:
- `GenericBound::AssocTraitBound(&Ty, &PathSegment, Span, Option<DefId>)` — represents `B: C::Elem` in bound position, where the optional `DefId` carries UFCS trait disambiguation.

### DefKind and middle representation

- `DefKind::AssocTrait` — a new `DefKind` variant, distinct from `AssocTy`.
- `AssocKind::Trait { name: Symbol }` — a new `AssocKind` variant at the `ty` level.
- `AssocTag::Trait` — used to probe for associated trait items specifically.

### Predicate

A new clause kind is added:

```rust
ClauseKind::AssocTraitBound(AssocTraitBoundPredicate {
    self_ty: Ty,      // The bounded type (B)
    projection: AliasTerm,  // The projection (<C as Container>::Elem)
})
```

### Name resolution

The resolver accepts partial resolutions (paths with unresolved trailing segments) in `PathSource::Trait(AliasPossibility::Maybe)` when the base is a type parameter, `Self` type param, or `Self` type alias. This allows `C::Elem` in bound position to resolve `C` as a type parameter and leave `Elem` as an unresolved associated item.

For UFCS paths (`<T as Trait>::Elem`), the resolver handles fully-qualified paths through `PathSource::TraitItem`, now extended to accept `DefKind::AssocTrait`.

### Type checking

**Projection**: Associated traits do *not* participate in type projection. `project()` returns `NoProgress` for `AssocTrait` projections. `type_of()` is unreachable (`span_bug!`) for associated traits.

**Solver support**: Associated traits are supported by both the old and new trait solvers. The old solver resolves `AssocTraitBound` predicates through its fulfillment engine (selecting the impl, extracting value bounds, emitting concrete trait obligations) and its evaluation path (mirroring the fulfillment logic for speculative queries). The new solver handles them via a dedicated `compute_assoc_trait_bound_goal` function. The feature is gated as `unstable`, requiring `#![feature(associated_traits)]`.

**Bound enforcement**: When `B: C::Elem` appears as a bound, the HIR type lowering emits a `ClauseKind::AssocTraitBound` predicate. Both solver resolve this as follows:

1. **Structurally normalize** the self-type of the projection's trait reference to determine whether the concrete impl is known.
2. **For abstract types** (type parameters, aliases, placeholders): add only the parent trait obligation (e.g., `C: Container`) and return success. The concrete value bounds cannot be resolved yet because the impl is unknown; declaration bounds are separately validated by `compare_impl_assoc_trait` at the impl site.
3. **For concrete types**: iterate over relevant impls matching the trait reference. For each candidate impl:
   a. Probe the impl and unify the goal trait reference with the impl's trait reference.
   b. Fetch the eligible associated trait item from the impl via `fetch_eligible_assoc_item`.
   c. Read `item_bounds()` on the impl item to extract the concrete value traits.
   d. For each value trait, emit a new `TraitPredicate` goal with the original self-type (e.g., `B: Send` if `Elem = Send`).

This means `Rc<i32>: C::Elem` is correctly rejected when `Elem = Send`.

**Declaration bounds**: When a declaration has bounds (`trait Elem: Clone;`), `compare_impl_item` checks that the impl's value satisfies those supertraits.

**UFCS disambiguation**: When the optional `constraint_trait` is present in `AssocTraitBound`, the bound resolution filters candidates to only the specified trait, correctly disambiguating `<T as Readable>::Constraint` from `<T as Writable>::Constraint`.

### Interaction with other features

**Associated types**: Associated traits and associated types coexist in the same trait body. They cannot share the same name (`E0325`). A trait can have both `type Item` and `trait Constraint`. An associated type can be bounded by an associated trait: `type Item: Self::Constraint;`.

**Generic associated types**: Associated traits can be declared alongside generic associated types and used to constrain their parameters at the use site, as in the `PointerFamily` pattern.

**Trait inheritance**: Associated traits are inherited through supertraits. If `trait Base { trait Elem; }` and `trait Extended: Base { ... }`, then `Extended` inheritors can use `Self::Elem`.

**`impl Trait`**: `impl C::Elem` in return position or argument position works through opaque type lowering, creating an opaque type bounded by the associated trait.

**Cross-crate**: Associated trait declarations and values are available across crate boundaries. The metadata encodes `DefKind::AssocTrait` and `explicit_item_bounds` for associated trait items.

**`dyn Trait`**: Using an associated trait as a dyn bound (`dyn T::Elem`) is rejected because Rust type-checks generic bodies before monomorphization — the concrete trait set behind `T::Elem` is not yet known, and `dyn` requires a compile-time-known vtable layout. This is the same fundamental constraint that prevents `dyn T` where `T` is a type parameter. A trait that merely *has* associated traits can still be used as `dyn Trait` — the associated trait is simply unused in the dyn context.

### Comparison with associated types

| Aspect | Associated Type | Associated Trait |
|--------|----------------|-----------------|
| Declaration | `type Foo;` | `trait Foo;` |
| Value | `type Foo = i32;` | `trait Foo = Send;` |
| Position | Type position | Bound position |
| `type_of` | Returns the type | Unreachable (`span_bug!`) |
| Projection | Yes (`T::Foo` is a type) | No (`T::Foo` is a constraint) |
| DefKind | `AssocTy` | `AssocTrait` |
| Generics | Yes (generic associated types) | Yes (generic associated traits) |
| Defaults | Yes | Yes |
| UFCS | `<T as Trait>::Foo` | `<T as Trait>::Foo` |

## Drawbacks
[drawbacks]: #drawbacks

- **Language complexity**: This adds a new kind of associated item. Rust already has associated types, associated constants, and associated functions. A fourth category increases the surface area that all users, tools, and documentation must understand.

- **Partial overlap with where clauses**: Some simple cases that associated traits address can be handled today via additional trait parameters or where clauses, though less ergonomically and without the ability to vary per implementation.

- **Solver complexity**: The `AssocTraitBound` predicate adds a new resolution pathway in both the old and new trait solvers — the old solver through its fulfillment engine and evaluation path, the new solver via a dedicated `compute_assoc_trait_bound_goal` function. Both pathways must be maintained alongside existing projection and trait obligation machinery.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why not trait aliases?

[Trait aliases](https://github.com/rust-lang/rfcs/pull/1733) (e.g., `trait SendClone = Send + Clone;`) are fixed at definition time. They cannot vary per implementation of an enclosing trait. Associated traits are the "associated" analog — they provide the same expressiveness that associated types provide over type aliases.

### Why not additional generic parameters?

Instead of `trait Container { trait Elem; }`, one could write `trait Container<Elem: ?Sized> { ... }`. This makes `Elem` a *generic parameter*, not an *associated item*. The tradeoffs mirror those of generic parameters vs. associated types:

- Generic parameters allow multiple implementations for the same type (e.g., `impl Container<Send> for Vec` and `impl Container<Clone> for Vec`), which is usually not desired.
- Associated items enforce that each impl provides exactly one value, which is the common case.
- Associated items don't appear in the type signature at every use site.

### Impact of not doing this

Without associated traits, the Rust ecosystem will continue to duplicate trait hierarchies when constraints need to vary (as seen with async runtimes, serialization frameworks, and plugin systems). This duplication is a persistent source of boilerplate and a barrier to writing truly generic code.

## Prior art
[prior-art]: #prior-art

### Haskell: ConstraintKinds

GHC's [`ConstraintKinds`](https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/constraint_kind.html) extension allows constraints to be used as first-class kinds. A type family can return a constraint:

```haskell
type family ElemConstraint (c :: * -> *) :: Constraint
type instance ElemConstraint MyVec = (Send, Clone)
```

This is more general than associated traits (constraints can appear in more positions), but Rust's associated traits achieve the most commonly needed subset — parameterizing traits over constraints — in a way that fits Rust's existing associated item pattern.

### Haskell: RMonad

The [`rmonad`](https://hackage.haskell.org/package/rmonad) library uses `ConstraintKinds` to define restricted monads, enabling data structures like `Set` (which requires `Ord`) to be monads. Associated traits in Rust would enable similar patterns: a `Family` trait with `trait Bounds` can restrict what types a type constructor accepts.

### Scala: Abstract type members

Scala's abstract type members serve a similar role to Rust's associated types, and Scala's type bounds can express constraint parameterization. However, Scala does not have a direct analog to "associated trait constraints" as a named, implementor-chosen bound.

### Swift: Associated type constraints

Swift's protocol associated types can have constraints (`associatedtype Element: Sendable`), but these are fixed at the protocol level — implementors cannot choose different constraints. Rust's associated traits go further by making the constraint itself the associated item.

## Out of scope
[out-of-scope]: #out-of-scope

- **Negative associated trait bounds**: e.g., `trait Elem = !Send;`. This intersects with negative impls and is deferred.
- **`dyn` with associated traits**: `dyn T::Elem` is not supported because Rust's type system requires dyn vtable layouts to be known at type-checking time, before monomorphization resolves `T`. This is the same constraint that prevents `dyn T` for type parameters. A trait with associated traits can still be used as `dyn Trait`; only the associated trait itself cannot appear as a dyn bound.
- **Trait-level generic parameters**: `fn foo<trait T>()` — allowing traits as first-class generic parameters — is a distinct and more general feature. It is the natural dual: associated traits are to trait aliases as associated types are to type aliases; trait parameters would be to generic type parameters as associated traits are to associated types.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

None at this time.

## Future possibilities
[future-possibilities]: #future-possibilities

- **Trait-level generic parameters**: As proposed in the original issue, allowing `fn foo<trait Trait>()` where `Trait` is a first-class generic parameter (see [Out of scope](#out-of-scope)). This would complement associated traits by enabling fully generic constraint parameterization at call sites.

- **Higher-kinded types interaction**: As noted by several commenters on the original issue, associated traits compose well with HKT-like patterns. A `Family` trait with `trait Bounds` and `type Of<T: Self::Bounds>` is a step toward restricted monads / restricted functors, similar to Haskell's `rmonad`.

- **Non-lifetime HRTBs**: Combined with [non-lifetime higher-ranked trait bounds](https://github.com/rust-lang/rust/issues/108185) (`for<T: Foo> Bar<T>: Baz<T>`), associated traits would enable significantly more expressive generic constraint patterns.

- **Const associated traits**: By analogy with const generics, one could imagine associated traits that are const-evaluable, though the motivation for this is unclear.
