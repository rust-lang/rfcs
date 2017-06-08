- Feature Name: type_aliases_in_enum_repr_attribute
- Start Date: 2016-05-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allows type aliases of rust integer types to be used in `#[repr(...)]` attributes of C-like enumerations to specify variant representation.

# Motivation
[motivation]: #motivation

The crate `libc` provides FFI bindings to system functions and defines related constants. Some of these constants form sets of allowed arguments to parameters of certain functions. The types of these constants can vary depending on the target.

A crate can wrap these constants into enumerations whose variants represent and are technically represented by the constants' values. In order to use the type of these constants as the underlying type, the crate currently needs to duplicate the determination of the correct rust integer type based on the target, because the `repr` attribute does not allow for type aliases of rust integer types.

Using the type of the constants as an underlying type of the enumeration allows for zero-cost conversion between variants and arguments to the FFI functions in low level code.

# Detailed design
[design]: #detailed-design

Introduce a new syntax variant of the `repr` attribute for enumerations: `#[repr(type = <type>)]` where `<type>` is a rust integer type or a type alias for such a type in the scope of the current module. The Alternatives section explains why `#[repr(<type>)]` is not proposed.

For example:

```Rust
type foo = i32;

#[repr(type = foo)]
enum bar {
    BAZ = 1,
}
```

# Drawbacks
[drawbacks]: #drawbacks

It complicates the syntax. In particular, specifying the underlying type to one of the rust integer types could be accomplished in two syntactically different ways, for example:

```Rust
#[repr(i32)]
```

and

```Rust
#[repr(type = i32)]
```

# Alternatives
[alternatives]: #alternatives

Using `#[repr(<type>)]` instead of `#[repr(type = <type>)]` is an alternative. However, this would lead to ambiguities when a type alias shares the name of a valid non-type argument to `repr`, for example:

```Rust
type C = i32;

#[repr(C)]
...
```

There is the alternative of doing nothing. As mentioned in the motivation, one can achieve the same result duplicating the correct type determination based on the target using `cfg_attr` in conjunction with `repr`.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
