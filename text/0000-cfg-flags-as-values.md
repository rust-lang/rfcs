- Feature Name: cfg_values
- Start Date: 2015--08-17
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC describes a set of macros to be added to the language - named `cfg_integer!(name)`,
`cfg_str!(name)`, `option_cfg_integer!(name)`, and `option_cfg_str!(name)`. A call to one of these
macros will expand to the value of the configuration flag with the given name, either as a compile time literal,
`Some(literal)`, or `None`. 

`option_cfg_str!(name)` and  `option_cfg_integer!(name)` will expand into
* `Some(literal)` if `name` is defined and is a valid string or integer

For example, `cfg_str!(target_os)` will expand to the string literal `"linux"` when targeting Linux,
`cfg_integer!(target_pointer_width)` will expand to the integer literal `64` when targeting a 64-bit
machine, and `option_cfg_str!(target_env)` will expand to the value `Some("gnu")` on a Gnu system and
`None` on a Windows system.

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

Which could be made more generic and simplier with a call to `cfg_integer!(target_pointer_width)`.


# Detailed design

Note that I already have a working implementation [here](https://github.com/dylanmckay/rust/tree/cfg-flag-as-int).

This will add four macros - `cfg_integer`, `option_cfg_integer`, `cfg_str`, and `option_cfg_str`, under the `cfg_values` feature gate.
Each one will take a single argument - the name of the configuration flag, and will expand to some kind of compile
time constant or identifier.

The `option_` variants will expand to `None` if a configuration flag does not exist, or cannot be converted to the specified type
(i.e. `option_cfg_integer!(target_os) == None`).

The other variants (`cfg_integer!`, and `cfg_str!`) will expand to a compile time integer or string literal, generating an error if 
their respective flags are not defined, or cannot be converted to a string or an integer.

## `cfg_integer` macro

The `cfg_integer!(flag_name)` macro will find the configuration flag named `flag_name`, convert it into an unsuffixed
integer literal (so that users of the macro can use type inference to avoid unnecessary casting).

If the value of the flag is not a valid integer, the compiler will generate an error.

## `cfg_str` macro

The `cfg_str!(flag_name)` macro is the simplest of all of the macros. It will verbatim evaluate to whatever the
value of the configuration flag is. All cfg flags are valid strings, and so there should be no edge cases to handle.

It expands into a string literal - for example, `cfg_str!(target_os)` would be the same as writing `"windows"` on
a Windows machine.

## `option_cfg_integer!(flag_name)` and `option_cfg_str!(flag_name)` macros

These macros work exactly the same as their unprefixed counterparts, but instead of expanding to a literal on success
and an error on failure, they are expanded to `Some(literal)` on success, and `None` on failure.

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

* `cfg_integer!(flag)` => `cfg_value!(flag: int)`
* `cfg_str!(flag)` => `cfg_value!(flag: str)`

Mimicing Rust's variable declartion style.

# Unresolved questions

None.
