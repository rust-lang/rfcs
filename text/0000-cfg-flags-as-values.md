- Feature Name: cfg_values
- Start Date: 2015--08-17
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC describes a set of macros to be added to the language - named `cfg_int!(name)`,
`cfg_str!(name)`, `cfg_float!(name)`, and `cfg_ident!(name)`. A call to one of these
macros will expand to the value of the configuration flag with the given name as a compile time literal,
generating a compile time error if the flags do not exist, or cannot be converted into the specified type.

For example, `cfg_str!(target_os)` will expand to the string literal `"linux"` when targeting Linux,
`cfg_int!(target_pointer_width)` will expand to the integer literal `64` when targeting a 64-bit
machine, and `cfg_ident!(target_env)` will expand to the identifier `gnu` on a Gnu system.

# Motivation

Today, proper support for configuration flags is lacking - there are only two operations you can do:

* Conditionally include/exclude a block of code depending on whether is cfg-flag is defined or has
  a specific value (`#[cfg(..)]`)
* Check whether a cfg-flag is defined (`cfg!(..)`)

There is no way to get the value of a configuration flag. Currently Rust works around this by doing
things such as:

``` rust
#[cfg(target_pointer_width = "32")]
int_module! { isize, 32 }
#[cfg(target_pointer_width = "64")]
int_module! { isize, 64 }
```

And

``` rust
#[cfg(target_os = "android")]   pub mod android;
#[cfg(target_os = "bitrig")]    pub mod bitrig;
#[cfg(target_os = "dragonfly")] pub mod dragonfly;
// ..
// ..
#[cfg(target_os = "macos")]     pub mod macos;
#[cfg(target_os = "nacl")]      pub mod nacl;
#[cfg(target_os = "netbsd")]    pub mod netbsd;
#[cfg(target_os = "openbsd")]   pub mod openbsd;
```

Which could be made more generic and simplier with a call to `cfg_int!(target_pointer_width)` or
`cfg_ident!(target_os)` respectively.


# Detailed design

Note that I already have a working implementation [here](https://github.com/dylanmckay/rust/tree/cfg-flag-as-int).

This will add four macros - `cfg_int`, `cfg_float`, `cfg_str`, and `cfg_ident`, under the `cfg_values` feature gate.
Each one will take a single argument - the name of the configuration flag, and will expand to some kind of compile
time constant or identifier.

For each of these macros, if the configuration flag requestion does not exist, a compile time error will be
generated.

## `cfg_int` macro

The `cfg_int!(flag_name)` macro will find the configuration flag named `flag_name`, convert it into an unsuffixed
integer literal (so that users of the macro can use type inference to avoid unnecessary casting).

If the value of the flag is not a valid integer, the compiler will generate an error.

## `cfg_float` macro

The `cfg_float!(flag_name`) macro will work identically to the integral version, but with floats. It will evaluate
to an unsuffixed floating point literal, and generate an error if the flag's value is not floating point. This will
work with all values that `cfg_int!(flag_name)` works with - i.e. if `cfg_int!(flag)` maps to `12`, then
`cfg_float!(flag)` maps to `12.0`.

## `cfg_str` macro

The `cfg_str!(flag_name)` macro is the simplest of all of the macros. It will verbatim evaluate to whatever the
value of the configuration flag is. All cfg flags are valid strings, and so there should be no edge cases to handle.

It expands into a string literal - for example, `cfg_str!(target_os)` would be the same as writing `"windows"` on
a Windows machine.

## `cfg_ident` macro

`cfg_ident!` expands to the value of the cfg flag as if it were an identifier embedded directly in the code.

For example
``` rust
#[cfg(target_os = "android")]   pub mod android;
```

Could be written as
``` rust
pub mod cfg_ident(target_os);
```

If the value of the flag is not a valid identifier, an error will be generated.

# Drawbacks

* There are four different macros - this is a large change, especially as they essentially all do the same thing
* It may make code harder to read (especially if not used responsibly)
* Doesn't have the largest number of usecases

# Alternatives

We could leave it as it is today. The macros are only useful in a small set
of situations, however in those situations it can work very well. We wouldn't
be much worse off if we left support for these features out of Rust.

Currently, you can only do conditional operations on configuration flags,
which can be quite limiting. This is probably the most straightforward and intuitive
way support for the feature could be implemented.

In order to minimize the number of macros defined, we could instead define the macros
like so:

* `cfg_int!(flag)` => `cfg_value!(flag: int)`
* `cfg_str!(flag)` => `cfg_value!(flag: str)`

Mimicing Rust's variable declartion style.

# Unresolved questions

None.
