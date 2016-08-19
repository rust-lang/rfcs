- Feature Name: Function Name Macro
- Start Date: (2016-08-19)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

`function` is a macro, which expands to the fully qualified function name from which
it was invoked. It works similar to `std::file` or `std::line` and expands to
an expression of type `&'static str`.

# Motivation
[motivation]: #motivation

There are currently macros for getting the source file name, `std::file`, and
the line number, `std::line`. To complete this pair, a macro to for getting
the current function name would be useful. All three macros are useful for
e.g. generating log output with more context about where something was
happening. The file and line is good for being able to look up the exact
location, however the fully qualified function name gives additional context
without even having to look into the source code.

# Detailed design
[design]: #detailed-design

This feature would have an equivalent design and implementation to
`std::file`. It would return the fully qualified function name, e.g.

 - `hello::main` for function `main` crate `hello`
 - `hello::bar::foo` for function `foo` in crate `hello` and module `bar`
 - `hello::bar::Foo::new` for function `new` in implementation of struct
   `Foo`, in crate `hello` and module `bar`
 - `hello::bar::Foo<T>::new` for function `new` in implementation of struct
   `Foo<T>`, in crate `hello` and module `bar`

# Drawbacks
[drawbacks]: #drawbacks

Any addition to the standard library will need to be maintained forever, so it
is worth weighing the maintenance cost over the value added. Given that this
is a feature that is considered useful in other languages (e.g. `__FUNC__` in
C) and is widely used there, it seems to be a useful addition to Rust too.

# Alternatives
[alternatives]: #alternatives

There are no alternatives to implement this in user code at this point, it
would have to be provided by the compiler and standard library.

# Unresolved questions
[unresolved]: #unresolved-questions

 - Should a different naming scheme be used?
   - An alternative would be using the naming scheme of backtraces, but
     that is implementation defined and more disconnected from the code
