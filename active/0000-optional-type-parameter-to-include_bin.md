- Start Date: 2014-06-17
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add the ability to optionally provide a type argument to `std::macros::builtin::include_bin!` macro to make it yield an expression of that type instead of type `&'static [u8]`.

# Motivation

The added type safety when using `include_bin!` to initialize static data that shouldn't be of type `&[u8]`.

# Detailed design

The `include_bin!` macro would take an optional second type argument after the non-optional `$file:expr` argument separated by a comma (or perhaps a colon). This type, say `T` would have to implement the `std::kinds::Sized` trait and `T` would also have to be pointer-free, i.e. no part of the data in `T` is allowed to be of a pointer or a reference type. If the size of `T` (`std::mem::size_of::<T>()`) is different from the size of the file specified by the first, `$file:expr` argument, then this macro invocation should result in a compile-time error saying something like: _"The size of the file doesn't match the size of the type"_. An invocation of the `include_bin!` macro that uses this optional type parameter should be evaluable in a static context, just like an invocation that doesn't use the optional type parameter is.

Example:
```
static VALUES: [i32, ..100] = include_bin!("values.bin", [i32, ..100]);
```

# Drawbacks

# Alternatives

# Unresolved questions
