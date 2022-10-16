- Feature Name: `calling_default_trait_methods`
- Start Date: 2022-10-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow trait impls to call default implementations of methods from the overriding implementations.

# Motivation
[motivation]: #motivation

This is useful whenever an overriding implementation merely wants to augment, not completely replace, the default implementation.

## Example: `syn::visit::Visit`
[example-visit]: #example-visit

For example, consider the trait [syn::visit::Visit](https://docs.rs/syn/1.0.102/syn/visit/trait.Visit.html). Each of the visit methods has a default implementation which calls a corresponding function of the same name. This allows implementors to call the function to include the default behavior.

The simplified `syn` code looks like:

```rust
pub trait Visit {
    fn visit_block(&mut self, i: &Block) {
        visit_block(self, i);
    }
}

pub fn visit_block<V>(v: &mut V, node: &Block)
where
    V: Visit + ?Sized,
{
    tokens_helper(v, &node.brace_token.span);
    for it in &node.stmts {
        v.visit_stmt(it);
    }
}
```

and user code looks like:

```rust
struct MyVisit;

impl Visit for MyVisit {
    fn visit_block(&mut self, i: &Block) {
        visit_block(self, i);
        ...
    }
}
```

This requires the trait author to expose the default implementations intentionally.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Trait implementations can call the default implementation using `self.super.foo()`, or when ambiguous, `<Struct::super as Trait>::foo(self)`.

Given a trait like:

```rust
pub trait Visit {
    fn visit_block(&mut self, i: &Block) {
        ...
    }
}
```

an impl can call the default implementation like so:

```rust
struct MyVisit;

impl Visit for MyVisit {
    fn visit_block(&mut self, i: &Block) {
        self.super.visit_block(i);
        // Alternatively:
        <MyVisit::super as Visit>::visit_block(self, i);
        ...
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Method resolution in default implementations
[method-resolution-in-default]: #method-resolution-in-default

The default implementation will still call overridden implementations when calling trait methods, as it would normally. For recursive default implementations, the recursive call will resolve to the overriding implementation. For example, given:

```rust
trait Trait {
    fn count_down(&self, n: usize) {
        println!("Trait: {n}");
        if n > 0 {
            self.count_down(n - 1);
        }
    }
}

struct Struct;

impl Trait for Struct {
    fn count_down(&self, n: usize) {
        println!("Struct: {n}");
        self.super.count_down(n);
    }  
}
```

calling `Struct.count_down(2)` will print:

```
Struct: 2
Trait: 2
Struct: 1
Trait: 1
Struct: 0
Trait: 0
```

## Permissive supercalls
[permissive-supercalls]: #permissive-supercalls

Supercalls are permitted anywhere, on the basis that the default implementation can only call other trait methods which the caller would be able to call anyway.

# Drawbacks
[drawbacks]: #drawbacks

This could lead to default implementations being called in contexts where this was not intended.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Other syntax was considered, such as:

- `super.method()`: This implicitly refers to `self` while not naming it, which could be surprising. It also makes it difficult to call default implementations on other values of the same type.
- `<super::Trait>::method(self)`: Using `super::` as a prefix could conflict with the existing semantics of `super`.
- `Trait::method(self)`, `<Trait>::method(self)`: These are already valid and call the overriding implementation.
- `<super as Trait>::method(self)`: When there is no `self` receiver (i.e. `<super as Trait>::method()`), it's not clear which concrete type is used. This becomes relevant if the default implementation calls another method in the same trait, which would then need to resolve to its overridden implementation.

In any case, something like universal function call syntax will be necessary in some cases to resolve ambiguity.

# Prior art
[prior-art]: #prior-art

## Java

In Java, classes can implement multiple interfaces, which can have default methods.  The equivalent syntax for calling an interface method would be `MyInterface.super.method()`. Static interface methods in Java can't be overridden and can't call non-static methods, so the question of how to dispatch further method calls inside the default implementation does not arise.

## C++

In C++, classes can call member functions from base classes using `MyBaseClass::method()`.  This won't work in Rust, because calling `MyTrait::method(self)` will call the overriding implementation.

## Python

Python supports `super().method()` for super calls and specifies a method resolution order (MRO) for disambiguation of multiple superclasses define a method of the same name. The method resolution order can be surprising, so we consider it better to be explicit when ambiguous in Rust, which is consistent with other Rust behavior.

## Rust ambiguous method names

If a struct implements multiple traits containing methods with the same name, Rust will require the programmer to disambiguate method calls, even within an impl of a trait declaring the method being called.  For example:

```rust
trait TraitA {
    fn foo(&self);
    fn bar(&self);
}

trait TraitB {
    fn foo(&self);
}

struct Struct;

impl TraitA for Struct {
    fn foo(&self) {
    }
    fn bar(&self) {
        self.foo()
    }
}

impl TraitB for Struct {
    fn foo(&self) {
    }
}
```

will print:

```
error[E0034]: multiple applicable items in scope
  --> src/main.rs:16:14
   |
16 |         self.foo()
   |              ^^^ multiple `foo` found
```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

It may make sense to limit contexts in which default implementations may be explicitly called, at least to begin with, as it would be possible to allow them in more places in a backwards-compatible manner.

# Future possibilities
[future-possibilities]: #future-possibilities

None.
