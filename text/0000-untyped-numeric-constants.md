- Feature Name: untyped_numeric_constant
- Start Date: 2017-04-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allows numeric constants to be defined without specifying an explicit type. Each time that constant is used, the type will be inferred as if it was an untyped numeric literal.

# Motivation
[motivation]: #motivation

The major motivation for this is FFI bindings, in particular when converting a constant of the form `#define FOO 273`. The equivalent in Rust would be `pub const FOO: ??? = 273;`, but figuring out which type to use can often be difficult. In some cases it can even be impossible, where a given constant is used in multiple places each with a different type. In `winapi` alone there are over twenty five thousand constants, and those decisions make up a significant amount of time when writing the bindings and cannot be automated. Even worse, if the wrong decision is made, it cannot later be changed in the future without causing a breaking change. Untyped numeric constants would solve this completely.

Even crates which aren't FFI bindings, but provide a variety of numerical constants can benefit from this feature.

# Detailed design
[design]: #detailed-design

Allow untyped numeric constants to be defined like so:

```Rust
const INTEGER = 273;
const FLOAT = 4.2;
```

When an untyped numeric constant is used, it will be equivalent to using the literal directly, and type inference will function identically. Thus the following two statements are identical in behavior (assuming `a` and `b` are later used identically) :

```Rust
let a = INTEGER + 5;
let b = 273 + 5;
```

The type of an untyped numeric constant can be different each time it is used. Thus the following is valid:

```Rust
let a: i32 = INTEGER;
let b: u32 = INTEGER;
```

It may also be desirable to create constants using simple constant expressions, for example:

```Rust
const ROOM_TEMPERATURE = INTEGER + 20;
```

However, this would be much more complicated to support than simply untyped integer constants and can be reserved for a future extension if necessary.

No support for inferring integer constants as floats or vice versa is being proposed.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Just call them untyped integer constants or untyped float constants or untyped numeric constants.

Teach them as an extension of standard constants.

People who use such constants won't even notice the difference so there's no pressing need to teach it right away.

It will require additions to all the documentation with regards to constants.

# Drawbacks
[drawbacks]: #drawbacks

* New feature that has to be supported.
* Can get in the way of a future RFC for inferred constants.

# Alternatives
[alternatives]: #alternatives

* One alternative is to use macro constants, which involves defining a macro for each constant, and invoking the constant via `FOO!()` instead of `FOO`. It is verbose and ugly, clutters the global macro namespace, and will probably uncover some performance regression in rustc. Macros 2.0 may get rid of the global namespace pollution, and an RFC could make calling the macro as simple as `FOO!` but it is still far from ideal.
* The status quo of having to decide on a type for each constant, or provide multiple versions of the constant each with a different type.

# Unresolved questions
[unresolved]: #unresolved-questions

* Is this the right syntax? Is there other syntax that would be preferable?
* Currently you can explicitly specify the type of an integer literal via suffixes. How would a user explicitly specify the type of a constant? Type ascription?
* Do we support constant expressions involving arithmetic? What about arbitrary const fn support? This may end up requiring more generalized polymorphic generic support.