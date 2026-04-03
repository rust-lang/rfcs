- Feature Name: `natural-method-disambiguation`
- Start Date: 2026-01-27
- RFC PR: [rust-lang/rfcs#3913](https://github.com/rust-lang/rfcs/pull/3913)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

The proposal introduces two new forms of method call syntax for method name disambiguation that keep the receiver on the left and preserve chaining.

1.  **Trait Method Call**: `expr.<path::to::Trait>::method(args)` allows invoking a specific trait's method inline without breaking the method chain.
2.  **Inherent Method Call**: `expr.Self::method(args)` is an explicit way to call an inherent method.

## Motivation
[motivation]: #motivation

### Method chain break
Currently, Rust's "Fully Qualified Syntax" (UFCS), e.g., `<Type as Trait>::method(&obj)` (or less commonly `Trait::method(&obj)`), is the main mechanism to disambiguate method calls between inherent implementations and traits, or between multiple traits.

It is worth noting that the proposed syntax is essentially a minor reordering that shortens the construct by removing `Type as` and the `&`/`&mut` operators, which carry no specific disambiguation information in this context.

While robust, UFCS forces a reversal of the visual data flow, breaking the fluent interface pattern:
*   **Fluent (Ideal)**: `object.process().output()`
*   **Broken (Current)**: `<Trait>::output(&object.process())`

### Silent bugs and Fragility

Currently, Rust's method resolution follows a fixed priority: it defaults to an inherent method if one exists. If no inherent method is found, the compiler looks for traits in scope that provide the method. If exactly one such trait is implemented for the type, the compiler selects it; otherwise, it returns an error.

This creates a "Primary and Fallback" mechanism where the compiler can silently switch between logic. If a primary (inherent) method is removed or renamed, the compiler may silently fall back to a trait implementation. Conversely, adding an inherent method can unexpectedly shadow an existing trait method call.

In rare cases, modifying one part of the code can unexpectedly alter logic elsewhere, causing a chain reaction of errors that makes it difficult to locate the root cause.

### Summary

This RFC aims to fully solve the problem of fluent method chaining. The second problem (fragility) requires a more complex approach, with this RFC being the first step. More details on potential future solutions are discussed in the [Future Possibilities](#future-possibilities) section.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

There are three ways to have something callable on `obj`:
- as a field containing a pointer to a function
- as an inherent method
- as a trait method 

While the first one is not confusing and has its unique syntax `(value.field)(args)`, the other two may cause some unexpected errors that seem unrelated to the actual mistake.

Imagine you have this piece of code 
```rust
use std::fmt::Display;

struct SomeThing<T> {
    something: fn(T),
}

impl<T: Copy + Display> SomeThing<T> {
    fn something(&self, _arg: T) {
        println!("inherent fn called, got {}", _arg)
    }
}

trait SomeTrait<T: Copy + Display> {
    fn something(&self, _arg: T);
}

impl<T: Copy + Display> SomeTrait<T> for SomeThing<T> {
    fn something(&self, _arg: T) {
        println!("trait fn called, got {}", _arg);

        print!("\t");
        self.something(_arg);
    }
}

fn main() {
    let value = SomeThing { something: |_arg: i32| {println!("fn pointer called, got {}", _arg)} };

    value.something(1);
    (value.something)(2);
    SomeTrait::something(&value, 3); // So that this can be compiled and checked in the current version of Rust
}
```

it works, it handles all three ways and prints
```plain
inherent fn called, got 1
fn pointer called, got 2
trait fn called, got 3
    inherent fn called, got 3
```

but if you change the line `impl<T: Copy + Display> SomeTrait<T> for SomeThing<T> {` to `impl<T: Copy + Display, U: Copy> SomeTrait<T> for SomeThing<U> {` instead of producing an error for the mismatch, the code compiles successfully and prints 

```plain
inherent fn called, got 1
fn pointer called, got 2
trait fn called, got 3
    trait fn called, got 3
    trait fn called, got 3
    trait fn called, got 3
    trait fn called, got 3
    trait fn called, got 3
    trait fn called, got 3
    trait fn called, got 3
    trait fn called, got 3
    trait fn called, got 3
    ...
```

> [!NOTE]
> Since `U: Copy` lacks `+ Display` bound required by the inherent implementation, the inherent method is not applicable within this context, causing the compiler to resolve to the trait method silently.

You would also get the same undesirable behavior in another case. You could rename `something` in `SomeThing`'s impl block and forget to rename it in the `SomeTrait`'s impl block
```rust
impl<T: Copy + Display> SomeTrait<T> for SomeThing<T> {
    fn something(&self, _arg: T) {
        println!("trait fn called, got {}", _arg);

        print!("\t");
        self.something(_arg); // here
    }
}
```

To prevent this and ensure the compiler rejects broken code, it would be better to use `self.Self::something(_arg)` instead of `self.something(_arg)`.

```rust
impl<T: Copy + Display> SomeTrait<T> for SomeThing<T> {
    fn something(&self, _arg: T) {
        println!("trait fn called, got {}", _arg);

        print!("\t");
        self.Self::something(_arg);
    }
}
```

`value.Self::method()` allows the compiler to only use an inherent method called `method` and errors if it hasn't been found.

### Method Chain Conflicts

Sometimes the ambiguity arises not within an implementation, but when using a type that implements traits with overlapping method names.

Consider a scenario where you have a `Builder` struct that implements both a `Reset` trait and has an inherent `reset` method.

```rust
struct Builder;
impl Builder {
    fn build(&self) -> String { "done".to_string() }
    fn reset(&self) -> &Self { self }
}

trait Reset {
    fn reset(&self) -> &Self;
}

impl Reset for Builder {
    fn reset(&self) -> &Self { self }
}

fn main() {
    let b = Builder {};
    // Defaults to the inherent method `reset` but silently falls back to the trait implementation if the inherent method is removed or renamed
    b.reset().build(); 
}
```

Using the explicit qualification syntax, you can explicitly choose which method to use without breaking the chain:

```rust
fn main() {
    let b = Builder;

    // Use the inherent reset method
    b.Self::reset().build();

    // Use the trait's reset method explicitly
    b.<Reset>::reset().build();
}
```

The `obj.<path::to::Trait>::method()` syntax allows disambiguating calls to trait methods.

This syntax is frequently used for disambiguation between different traits on the same object. Consider a more complex example from a simulation game where you have a `HydrocarbonDeposit` type. This type might implement multiple traits representing different resource extraction methods, such as `resources::sources::HeavyOilSource`, `resources::sources::LightOilSource`, and `resources::sources::NaturalGasSource`.

Each of these traits might have a `simulate` method that is not in-place (i.e., it returns a simulated version of the object rather than modifying it).

```rust
use resources;

fn process_deposit(deposit: HydrocarbonDeposit) {
    // Run simulation specifically for heavy oil extraction behavior
    let simulated = deposit
        .<HeavyOilSource>::simulate()
        .<LightOilSource>::simulate()
        .<NaturalGasSource>::simulate();
}
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Grammar Extensions

The `MethodCallExpr` grammar is extended in two specific ways:

1.  **Angle Bracketed Path**: `Expr '.' '<' TypePath '>' '::' Ident '(' Args ')'`
    *   This syntax is used for **Explicit Trait Method Calls**.
    *   **Resolution**: The `TypePath` is resolved. If it resolves to a trait, the `Ident` method from that trait is invoked with `Expr` as the receiver (the first argument).
    *   **Desugaring**: `obj.<Path>::method(args)` desugars to `<Type as Path>::method(obj, args)`, ensuring correct autoref/autoderef behavior for `obj`.
    *   **Restriction**: `Expr.<Self>::method(Args)` is not allowed (use `Expr.Self::method` instead).

2.  **Explicit Inherent Path**: `Expr '.' 'Self' '::' Ident '(' Args ')'`
    *   This syntax is used for **Explicit Inherent Method Calls**.
    *   **Resolution**: The `Ident` is looked up strictly within the inherent implementation of `Expr`'s type.
    *   **Semantics**: `obj.Self::method()` resolves to the inherent method `method` (equivalent to `<Type>::method` or `<Type as Type>::method`). It effectively bypasses trait method lookup.

### Resolution Logic Summary

*   **Case: `obj.<Trait>::method(...)`**
    *   Resolves to `<Type as Trait>::method(obj, args)`.

*   **Case: `obj.Self::method(...)`**
    *   Resolves to `<Type>::method(obj, args)`.

## Drawbacks
[drawbacks]: #drawbacks

*   **Parser Complexity**: The parser requires distinct rules to distinguish `.` followed by `<` (explicit trait call) versus `.` followed by `Self`.
*   **Visual Noise**: The syntax `.<...>::` adds complexity to method chains.
*   **Inconsistency**: It may confuse some users that `Self` does not require brackets while traits do.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

*   **Why Angle Brackets for Trait Method Calls?**
    *   `value.Trait::method` looks like there is something called `Trait` inside the `value` while `Trait` is coming from the scope of the call.
    *   `value.<Trait>::method` aligns with Rust's existing use of angle brackets for type-related disambiguation (like UFCS `<Type as Trait>::method`).
    *   **Reservation**: We specifically reserve the unbracketed syntax `value.Category::method()` (where `Category` is not `Self`) for possible future language features, such as "Categorical" or "Facet" views of an object.

*   **Why No Parentheses for Inherent Method Call?**

    *  The construct `<path::to::Trait>` has a consistent, independent meaning (the trait itself) regardless of the object it is applied to, which the angle brackets appropriately denote. Conversely, `<Self>` without the `obj.` prefix is context-dependent: it might refer to the type of `obj`, a different type entirely (e.g., the `Self` of the surrounding impl block), or nothing at all. Therefore, it is semantically preferable to associate `Self` more strongly with the object instance using the `obj.Self` syntax, effectively treating it as a pseudo-member access
    
    *   In `impl` blocks, we can apply `obj.Self` to objects that do not have the type named `Self` in that block. `obj.<Self>` would look like we are trying to apply a method of one type to an object of another type even if they happen to be the same. 
    
    *  Despite being technically feasible for the compiler to parse, `obj.<Self>` would appear clunky and unidiomatic. 

## Prior art
[prior-art]: #prior-art

## Prior art

Several programming languages face similar challenges with method disambiguation when inherent implementations conflict with trait-/interface-/extension-provided methods. Approaches generally fall into three categories: explicit qualification syntax, cast-based selection, or strict implicit resolution (sometimes with anti-features).

- **C++**  
  C++ supports direct qualification with the scope resolution operator: `obj.Base::method()` or `obj.Trait::method()`.  
  This is the closest analogue to the proposed `obj.Self::method()` (for inherent methods) and `obj.<Trait>::method()` (for trait methods).  
  It preserves chaining ergonomics and readability, which is a key inspiration for this RFC.

- **C#**  
  When a type explicitly implements interface members or when multiple interfaces provide the same method, disambiguation is typically achieved via explicit casts: `((IInterface)obj).Method()`.  
  Extension methods (somewhat analogous to blanket impls over traits) are resolved statically and can be called explicitly via the static class if needed, but there is no dedicated qualification syntax for instance calls.  
  Cast-based approaches interrupt method chaining and reduce readability compared to qualification.

- **Java**  
  Similar to C#, external disambiguation requires casts: `((Interface)obj).method()`.  
  Inside a class implementing multiple interfaces with default methods, one can use qualified super calls: `Interface.super.method()`.  
  Again, external calls rely on casts, which do not chain naturally.

- **Kotlin**  
  Within a class, qualified super calls are supported: `super<Interface>.method()`.  
  For external calls on an object, disambiguation uses casts: `(obj as Interface).method()`.  
  The internal syntax is close to the proposal, but external calls suffer the same chaining issues as cast-based approaches.

- **Swift** (anti-example)  
  Swift deliberately prohibits any form of type qualification or annotation on method calls to keep the grammar simple.  
  This can lead to ambiguities in generic code that require workarounds, such as passing explicit type information through parameters or using separate overloads.  
  This demonstrates the pitfalls of making disambiguation impossible when it is occasionally needed.

Many other languages (e.g., Haskell, Go) rely entirely on implicit resolution via type-class/instance selection or interface satisfaction, with coherence rules preventing most ambiguities. When ambiguities do arise, they are usually treated as errors requiring code restructuring rather than providing a syntactic escape hatch.

The current Rust approach (`<Type as Trait>::method(&mut obj)` and `Trait::method(&mut obj)`) works but is verbose and it breaks natural chaining. This proposal builds on C++-style qualification while adapting it to Rust’s orphan and coherence rules, offering explicit control without sacrificing ergonomics.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

*None*

## Future possibilities
[future-possibilities]: #future-possibilities

*   **Scoped Prioritization**: We can also introduce syntax like `use Trait for Foo` or `use Self for Foo` within a function scope to change default resolution without changing call sites.
*   **Disabling Inherent Preference**: A specialized macro or attribute could be introduced to opt-out of the default "inherent-first" resolution rule.
*   **Warning on Signature Collision**: Since identical signatures (matching positional argument types) are rare, we could warn on such overlaps. This would flag fragile call sites and detect when a newly added inherent method silently hijacks existing trait-based invocations.