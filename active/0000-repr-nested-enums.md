- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow enums that cannot contain values other than discriminants and C-like
enums to have an integer representation. Currently one can cast variants of
only simple enums to integers.

# Motivation

With this feature implemented, we can have better control over C-like enums:

```rust
// exception.rs
#[repr(u8)]
pub enum Fault {
    DivideError = 0,
    NMI = 2,
    Breakpoint = 3,
    // ...
}

// interrupt.rs
#[repr(u8)]    // < this is allowed
pub enum Interrupt {
    Fault(Fault),
    IRQ0 = 32,
    IRQ1 = 33,
    IRQ2 = 34,
    Syscall = 0x80
}
```

Moreover, teepee's status module could be refactored by removing the
`StatusClass` enum.

# Detailed design

An enum is C-like if all its variants belong to two groups:

* variants that have empty bodies
* variants that contain exactly one member which is a C-like enum

Allow the use non-empty variant bodies and explicitly assigned discriminants
in a single C-like enum.

Use the error message `discriminant value already exists [E0081]` to disallow
conflicting values in enums that have an integer representation mandated by
the `repr` attribute. The set of values of a C-like enum type is the union of
the set of all values it contains, and the set of its empty-bodied values.

# Drawbacks

* Adds some trans code that is not trivial.
* Yet another feature that might not be future proof.

# Alternatives

* Allow C-like enum variants that contain integers smaller than the
  representation of the enum. (Perhaps with only one such member per enum.)
* This could be approached as a general enum discriminant optimization.
  However, further improvements shouldn't be nearly as much of a language
  change as this one and are deemed to be an implementation detail out of
  scope of this RFC.
* Some kind of subtyping and variant inheritance is possible. That is, all
  possible values could be accessible under one namespace.

# Unresolved questions

Should these enums be C-like by default, even without `repr`?
