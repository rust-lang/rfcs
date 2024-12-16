# RFC: Making Rust Friendly

## Summary

This RFC proposes adding the chung and unchung keywords to Rust to make it more approachable for new users, simplifying code by automatically marking functions as unsafe and removing the borrow checker. The new keywords would enable a simpler programming experience, making Rust "friendlier" by introducing automatic reference counting (via cyclical RAII) and making certain warnings non-optional.

## Motivation

Rust's strict guarantees around memory safety, including its ownership model and the borrow checker, are powerful, but they can be difficult for beginners to understand and work with. While the borrow checker prevents common bugs, it can also be a barrier to entry, making the language feel "unfriendly" to newcomers. The primary goal of this RFC is to simplify Rust's memory model, remove some of its safety guarantees by default, and make the language more approachable for developers coming from other languages.

The introduction of chung and unchung keywords aims to reduce friction and make Rust more "friendly" without compromising on all the powerful features it offers. This RFC is targeted at reducing verbosity and simplifying common patterns to make Rust code easier to write and understand.

## Detailed Design

New Keywords

1. chung:

The chung keyword marks a function as automatically unsafe—meaning it can perform operations that violate Rust's safety guarantees without requiring an explicit unsafe block.

It effectively relaxes Rust's strict rules around borrowing and ownership. The function can do anything that the compiler would typically reject, including mutable aliasing, raw pointer dereferencing, and memory leaks.

A chung function will automatically generate reference counting for objects passed into it, replacing the borrow checker with reference-counted cyclic RAII (Resource Acquisition Is Initialization).

```rust
chung fn foo(bar: &str) {
    // The borrow checker is disabled for this function.
    // Reference counting happens automatically on the `bar` argument.
}
```

2. unchung:

The unchung keyword undoes the chung keyword, restoring some level of borrowing and ownership constraints, but with simplified rules.

unchung allows fine-tuned control, making functions "chung-compatible" while still ensuring some safety checks are in place.

The borrow checker is reinstated, but with fewer constraints, allowing reference counting to control the lifecycle of objects.

```rust
unchung fn bar() {
    // The borrow checker is partially enabled in this function.
}
```


# Function Behavior

Automatic Unsafe: Any function marked with chung is considered unsafe and can potentially violate the safety guarantees of Rust. However, this is simplified into a more user-friendly interface, where users don't need to write unsafe blocks manually.

Automatic Reference Counting: The function will automatically track references to any variables passed to it via reference counting, using a simplified RAII model. This ensures that memory is automatically cleaned up when no longer in use, preventing dangling references and memory leaks without the need for Box, Rc, or Arc.

```rust
chung fn foo(bar: &str) {
    // bar is reference-counted automatically inside this function.
}
```

# Borrow Checker

The borrow checker is disabled by default inside any chung function. It is replaced with an automatic reference-counting mechanism based on cyclical RAII. The cyclic RAII model will automatically manage the lifetimes of references and objects inside chung-marked functions.

Since the borrow checker is disabled, developers are freed from the constraints of ensuring there are no mutable references to the same data simultaneously.

chung fn bar() {
    let mut s1 = String::new();
    let mut s2 = s1.clone();
    // No borrowing rules are checked; s1 and s2 can coexist.
}


Warnings Lint Forbid by Default

A new lint rule will be introduced, marking warnings as forbidden by default in chung functions. This ensures that all code within these functions is considered acceptable without user intervention for minor issues like unused variables or possible panics.

This policy will make Rust less strict, particularly for beginners who might find the default linter's warnings overwhelming.


chung fn foo() {
    let unused_var = 10;
    // The warning for unused variables will be forbidden in this function.
}


Reference Counting with Cyclical RAII

The reference counting mechanism will automatically manage the memory of objects in chung functions. It will handle cases of cyclic references, which the Rust borrow checker normally disallows.

chung fn cyclic_references() {
    let x = Rc::new(RefCell::new(5));
    let y = Rc::clone(&x);
    // No borrow checker errors, as reference counting manages the memory.
}


Backwards Compatibility

The proposal will not break any existing code. All chung and unchung keywords will only be applicable to new or modified functions. Existing functions will continue to behave as they did before.


Rationale

Beginner-Friendly: The primary motivation behind chung and unchung is to make Rust more approachable for beginners and reduce the barriers associated with Rust’s strict ownership and borrowing rules.

Less Verbosity: Rust’s current system requires many explicit unsafe blocks, memory management decisions (like Box, Rc, Arc), and dealing with complex ownership rules. The chung keyword simplifies all of this by making these decisions implicit.

Reference Counting: This new approach automates many of the decisions around memory management and ownership, particularly for cyclic structures that require manual intervention in the current system.


Drawbacks

Loss of Safety Guarantees: The most significant drawback is the automatic disabling of safety guarantees, which could lead to memory bugs or undefined behavior in code that relies on the chung keyword.

Uncertainty in Large Projects: In larger projects where memory safety is critical, using chung extensively could lead to harder-to-debug issues, making it unsuitable for high-assurance systems.

Less Control for Experts: By abstracting away memory management decisions, advanced Rust users lose fine-grained control over object lifetimes and memory allocation.


Future Work

Investigate introducing additional keywords for more control over safety and memory management, like safe (opposite of chung), to allow a more nuanced approach for different use cases.

Improve the RAII reference counting system to handle complex cycles and larger object graphs more efficiently.


Conclusion

The introduction of the chung and unchung keywords would significantly change Rust’s memory model and approachability, making it easier to write and understand code. While it simplifies Rust for beginners, it does so at the cost of certain safety guarantees, making it less appropriate for advanced or safety-critical applications. However, it aligns with the goal of making Rust more "friendly" by lowering the barriers to entry.

