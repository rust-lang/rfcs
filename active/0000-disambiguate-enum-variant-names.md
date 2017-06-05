- Start Date: 2014-05-28
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Allow multiple enums in the same module to have the same variant names, and have `EnumType::EnumVariant`
to disambiguate between them. No change in behaviour for singly defined enum variant names.

# Motivation

In a language with algebraic datatypes like Rust or Haskell, there are problems which are excellently solved
using algebraic datatypes with a large number of variants, and with overlapping semantics, making memorable
names a scarce resource.

These applications include: abstract syntax trees (often clashing with the tokenizers ADT's) and dumb enumerations
for safe interaction with C libraries (some C libraries use the same constants for different semantic applications.)

# Detailed design

A simple case of enumeration disambiguation is the following:

    enum A {
        X,
        Y
    }

    enum B {
        Y,
        Z
    }

This is currently not allowed on pain of multiple definition error.

A use case of this proposal, given the above definitions, could be:

    let a : A = A::Y; /* disambiguate in expression */
    let b : B = Z; /* no disambiguation nessecary, Z is unique to B */

    match a {
        X => println!("X"), 
        A::Y => println!("Y") /* disambiguate in pattern */
    }

    match b {
        B::Y => println!("Y"),
        Z => println!("Z")
    }

# Drawbacks

Complicates compilation slightly, and probably opens up for bad practices.

# Alternatives

The existing alternative is to put each enum inside a module, remniscent of SML design practices:

    mod A {
        enum A {
            X,
            Y
        }
    }

    mod B {
        enum B {
            X,
            Y
        }
    }

This does pose the problem that referring to the type itself in the above example, is `A::A`.

# Unresolved questions

None so far.
