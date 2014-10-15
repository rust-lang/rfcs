- Start Date: 2014-10-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add ad-hoc operator overloading to Rust and remove the current-trait based 
overloading. 'ad-hoc' in this RFC means that when the compiler looks for an 
implementation of an operator overload it does so by looking for a specific 
method, rather than for a specific trait. The current trait-based system is a 
strict subset of this enhancement, which means this RFC is backwards 
compatible.

# Motivation

## Rust specific motivation

The current system for operator overloading involves implementing a specific 
trait (decorated with a lang-item attribute) that the compiler will then look 
for when the operator syntax is encountered. This is problematic, because 
implementing a trait carries with it a few restrictions:

* The relevant method's signature cannot be changed
    * The method cannot be declared `unsafe`
    * No lifetimes can be added/removed to the methods that are not present in
      the original trait
    * The methods may not be made private
    * The methods may not have additional type parameters

* The relevant trait implementation has to fulfill coherence requirements since 
  the operator overloading traits are 3rd party to most of the Rust's library 
  ecosystem

The first issue in particular is often felt, for example when attempting to 
define an `unsafe` variant of indexing and slicing.

## Other languages

Major languages that are similar to Rust use ad-hoc operator overloading (e.g. 
Haskell, Scala, C++, D). The author is not aware of a language with trait-like 
constructs that use them as the sole method of operator overloading. Haskell in 
particular has a type class that is typically instantiated to provide operator 
overloading, but that type class merely has specially named functions (as 
Haskell allows symbolic function names).

## Philosophical points

In many ways, ad-hoc operator overloading is identical to having multiple 
traits having a name with the same method. While this is discouraged, it is 
still allowed to happen. There is no additional lack of clarity of this code 
calling some arbitrary method:

```rust
a + b
```

versus this code using a trait (or maybe inherent, who knows!) method name:

```rust
a.add(&b)
```

In the latter case only by examining the traits available at call time as well 
as the inherent methods can the reader be sure about what code is actually 
evoked. This is not seen as a problem for normal methods, and therefore 
shouldn't be a problem for operators.

# Detailed design

## General usage

A new attribute `#[operator="xxx"]` would be introduced that can be applied to 
inherent and trait methods, like so:

```rust
trait MyTrait {
    #[operator="add"]
    fn add(&self, rhs: &uint) -> uint;
}

struct S;

impl S {
    #[operator="index"]
    unsafe fn index(&self, index: &uint) -> uint;
}

impl MyTrait for S {
    // operator attribute invalid here
    fn add(&self, rhs: &uint) -> uint;
}
```

`"xxx"` in the above definition would be replaced by a string corresponding to 
each operator (the author suggests using the method names of the current 
operator overloading traits). Invalid method signatures are accepted, but are 
naturally detected when the de-sugaring happens at the call site.

## Possible implementation

When the compiler sees a decorated method, it adds a duplicate, un-typeable, 
entry to the set of methods this type implements (or this trait provides). E.g. 
for `#[operator = "add"]` it will create a method named `<add>` (normally an 
invalid method name). This new entry acts as an alias to the regular method, 
which is still available under its raw name for disambiguation purposes. When 
the compiler sees an operator being used, it will rewrite the expression to 
some de-sugaring. E.g. this code:

```rust
a + b
```

would be transformed to this (by constructing the AST directly, rather than 
going through the lexer):

```
a.<add>(&b)
```

The exact de-sugaring should be chosen such that the methods of the current 
operator overloading traits continue to work (i.e. the desugaring will 
typically auto-borrow the RHS).

## Fate of the current operator overloading traits

The current traits will remain in place, but cease to be decorated by the 
lang-items (and thus those lang-items will be removed). E.g. this current trait:

```rust
#[lang="add"]
pub trait Add<RHS,Result> {
    fn add(&self, rhs: &RHS) -> Result;
}
```

would be changed to:

```rust
pub trait Add<RHS,Result> {
    #[operator="add"]
    fn add(&self, rhs: &RHS) -> Result;
}
```

For operator overloading traits with multiple methods, each method gets its own 
version of the `#[operator]` attribute.

The user code would not be affected.

# Drawbacks

There are no drawbacks.

# Alternatives

The alternative to this RFC is to add tons of traits for each possible 
variation in method signature and/or forbid some arguably valid uses of 
operator overloading.

# Unresolved questions

Although this RFC makes operator overloading a lot more flexible, it is still 
constrained by the de-sugaring chosen for a particular operator. E.g. you won't 
be able to create an `+` operator overload that moves the RHS. Even though you 
can mark such a method with `#[operator="add"]`, the de-sugaring will borrow 
the RHS and cause a type-check error. Addressing this issue is orthogonal to 
this RFC.
