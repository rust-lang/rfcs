- Feature Name: `associated_traits`
- Start Date: 2026-03-28
- RFC PR: [rust-lang/rfcs#3938](https://github.com/rust-lang/rfcs/pull/3938)
- Rust Issue: None

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

### Associated trait declarations

A trait definition may contain **associated trait declarations**, introduced with the `trait` keyword. An associated trait declaration names a trait constraint whose concrete value is provided by each impl.

**Grammar** (in trait body):

```
AssocTraitDecl =
    "trait" IDENT ";"
  | "trait" IDENT ":" Bounds ";"
  | "trait" IDENT "=" Bounds ";"
  | "trait" IDENT ":" Bounds "=" Bounds ";"
  | "trait" IDENT GenericParams ";"
  | "trait" IDENT GenericParams WhereClause ";"
  | "trait" IDENT GenericParams ":" Bounds ";"
  | "trait" IDENT GenericParams WhereClause ":" Bounds "=" Bounds ";"
```

Examples:

```rust
trait Container {
    trait Elem;                         // bare declaration
    trait Constraint: Clone;            // with supertrait bound
    trait Format = Send;                // with default value
    trait Handler<T>: Debug;            // generic
    trait Codec<T> where T: Send;       // generic with where clause
}
```

**Declaration bounds** (the part after `:`) are *requirements* on every impl's value. If `trait Elem: Clone;`, then every impl must provide a value that implies `Clone`. These bounds are also available to callers: a function with `T: C::Elem` may assume `T: Clone` when the declaration includes `: Clone`.

**Defaults** (the part after `=`) provide a value that impls may omit, analogous to default associated types. If an impl omits the associated trait, the default is used.

### Associated trait implementations

In a trait impl, the associated trait's value is provided with `trait Name = Bounds;`:

```rust
impl Container for MyVec {
    trait Elem = Send + Clone;
}
```

The value may be:
- One or more trait bounds: `Send`, `Clone + Debug`
- Trait bounds with associated type constraints: `IntoIterator<Item = i32>`
- Lifetime bounds: `'static`, `Send + 'static`
- Relaxed bounds: `?Sized`, `Send + ?Sized`
- Any combination of the above: `Debug + Send + 'static`

**Validation**: The compiler checks that the impl's value satisfies the declaration's supertrait bounds. If the declaration says `trait Elem: Clone;`, the impl value must include `Clone` (or a subtrait of `Clone`).

Associated traits are **not permitted in inherent impls** — only in trait impls:

```rust
impl MyStruct {
    trait Foo = Send;  // ERROR: not allowed in inherent impls
}
```

### Using associated traits as bounds

An associated trait may appear anywhere a trait bound is expected, using the path syntax `C::Elem`:

```rust
fn process<C: Container, T: C::Elem>(item: T) { ... }
```

This means: "`T` must satisfy whatever constraint `C::Elem` resolves to." In a generic context, the concrete value is not yet known, but any declaration bounds (supertraits) are available:

```rust
trait Container {
    trait Elem: Debug + 'static;
}

// T: Debug and T: 'static are available here from the declaration
fn to_debug<C: Container, T: C::Elem>(item: T) -> Box<dyn Debug> {
    Box::new(item)  // OK: T: Debug + 'static from declaration bounds
}
```

At a concrete call site (e.g., `process::<MyVec, i32>(42)`), the solver resolves `MyVec`'s impl of `Container`, extracts `Elem = Send + Clone`, and verifies `i32: Send + Clone`.

Associated traits may also appear in:

- **Where clauses**: `where T: C::Elem`
- **Inline bounds**: `fn foo<T: C::Elem>()`
- **`impl Trait`**: `fn foo(arg: impl C::Elem)` and `fn foo() -> impl C::Elem`
- **Combined with other bounds**: `T: C::Elem + PartialEq`

### Fully-qualified (UFCS) syntax

When a type parameter is bounded by multiple traits that each declare an associated trait with the same name, the shorthand `T::Elem` is ambiguous. Fully-qualified syntax resolves the ambiguity:

```rust
trait Readable { trait Constraint; }
trait Writable { trait Constraint; }

fn transfer<T: Readable + Writable,
            R: <T as Readable>::Constraint,
            W: <T as Writable>::Constraint>(r: R, w: W) { ... }
```

The syntax `<T as Trait>::AssocTrait` mirrors the existing UFCS syntax for associated types (`<T as Trait>::AssocType`).

### Positions where associated traits are rejected

Associated traits are constraints, not types. They are rejected in positions that expect a type:

- **Type position**: `let x: T::Elem = ...;` → error: "expected type, found trait"
- **Return type**: `fn foo() -> T::Elem` → error (unless `impl T::Elem`)
- **Struct fields**: `struct S { field: T::Elem }` → error
- **`dyn` position**: `dyn T::Elem` → error: "associated traits cannot be used with dyn"

The `dyn` restriction exists because `dyn` requires a compile-time-known vtable layout, and in a generic context the set of traits behind `T::Elem` is not yet known. This is the same fundamental constraint that prevents `dyn T` where `T` is a type parameter.

A trait that *has* associated traits can still be used as `dyn Trait` — the associated trait is simply inaccessible in the dyn context.

### Generic associated traits

Associated traits may have their own generic parameters, analogous to generic associated types (GATs):

```rust
trait Transform {
    trait Constraint<T: Clone>;
}

impl Transform for MyTransform {
    trait Constraint<T: Clone> = PartialEq<T> + Debug;
}
```

Generic parameters may include types, lifetimes, and bounds. Where clauses are also supported:

```rust
trait Codec {
    trait Decode<'a, T> where T: Send;
}
```

Usage includes the generic arguments:

```rust
fn decode<C: Codec, T: C::Decode<'static, u8>>() { ... }
```

### Interaction with associated types

Associated traits and associated types may coexist in the same trait. They occupy the same name namespace — a trait cannot have both `type Foo` and `trait Foo` with the same name.

An associated type may be bounded by an associated trait from the same trait:

```rust
trait Container {
    trait ElemConstraint;
    type Elem: Self::ElemConstraint;
}
```

### Interaction with generic associated types

Associated traits compose with GATs. An associated trait can constrain the parameters of a GAT at the use site:

```rust
trait PointerFamily {
    trait Bounds;
    type Pointer<T>;
}

fn wrap<F: PointerFamily, T: F::Bounds>(val: T) -> F::Pointer<T> { ... }
```

GAT parameters may also be directly bounded by associated traits:

```rust
trait Universe {
    trait BoundsIn;
    trait BoundsOut;
    type Ref<T: Self::BoundsOut>: RefLike<T>;
    type Cell<T: Self::BoundsIn>: CellLike<T>;
}
```

When a GAT parameter is bounded by `Self::Bounds` and used in an impl, the compiler substitutes the concrete associated trait value to check that the impl's GAT definition satisfies its requirements. No additional `where` clauses are required on downstream types that use the GAT — bounds on GAT parameters are checked at instantiation.

### Interaction with trait inheritance

Associated traits are inherited through supertraits:

```rust
trait Base { trait Elem; }
trait Extended: Base { trait Extra; }

fn use_both<T: Extended, E: T::Elem + T::Extra>() { ... }
```

UFCS can disambiguate inherited associated traits:

```rust
fn use_base<T: Extended, E: <T as Base>::Elem>() { ... }
```

### Interaction with `impl Trait`

`impl C::Elem` works in both argument and return position, creating an opaque type bounded by the associated trait:

```rust
trait Handler {
    trait Arg;
    fn handle(&self, arg: impl Self::Arg);
}
```

### Cross-crate usage

Associated trait declarations and their values are visible across crate boundaries. A crate can define a trait with associated traits, and downstream crates can implement and use them.

### Auto-trait interaction

The concrete types used with associated traits participate in auto-trait inference normally. For example, if `Universe::Ref<T>` is `Arc<T>`, then a struct containing `U::Ref<U::Cell<State>>` is `Send + Sync` when `U` is `Shared` (with `Arc<Mutex<State>>`), and `!Send` when `U` is `Isolated` (with `Rc<RefCell<State>>`).

### Comparison with associated types

| Aspect | Associated Type | Associated Trait |
|--------|----------------|-----------------|
| Declaration | `type Foo;` | `trait Foo;` |
| Value | `type Foo = i32;` | `trait Foo = Send;` |
| Valid positions | Type position | Bound position |
| Generics | Yes (GATs) | Yes (generic associated traits) |
| Defaults | Yes | Yes |
| UFCS | `<T as Trait>::Foo` | `<T as Trait>::Foo` |
| `dyn` compatible | Yes (with limitations) | No |

## Drawbacks
[drawbacks]: #drawbacks

- **Language complexity**: This adds a new kind of associated item. Rust already has associated types, associated constants, and associated functions. A fourth category increases the surface area that all users, tools, and documentation must understand.

- **Partial overlap with where clauses**: Some simple cases that associated traits address can be handled today via additional trait parameters or where clauses, though less ergonomically and without the ability to vary per implementation.

- **Solver complexity**: Associated traits introduce a new kind of predicate that both the old and new trait solvers must handle, adding to the compiler's internal complexity. However, the resolution logic follows established patterns from associated type projection.

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
