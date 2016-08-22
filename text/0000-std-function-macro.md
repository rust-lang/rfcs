- Feature Name: Function Name Macro
- Start Date: (2016-08-19)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

`function_name` is a macro, which expands to the fully qualified function name
from which it was invoked. It works similar to `std::file`, `std::line` or
`std::module_path` and expands to an expression of type `&'static str`.

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

 - Plain functions use their name, e.g.

````
    fn some_function(arg: SomeThing) {
    ...
    }
````

   would result in `some_function`

 - Generic functions use their name, followed by the generic arguments, e.g.

````
    fn some_function<T, U>(arg: SomeThing) {
    ...
    }
````

   would result in `some_function<T, U>`


 - Functions inside non-generic `impl` blocks for structs would use the name of the
   struct, followed by `::` and the function name, e.g.

````
    struct SomeStruct {...}

    impl SomeStruct {
        fn some_function(&self, arg: SomeThing) {
        ...
    }
````

   would result in `SomeStruct::some_function`

 - Functions inside generic `impl` blocks for structs would use the name of the
   struct, followed by the generic parameters as given by the `impl` block,
   followed by `::` and the function name, e.g.

````
    struct SomeStruct<T> {...}

    impl SomeStruct<i32> {
        fn some_function(&self, arg: SomeThing) {
        ...
    }
````

   would result in `SomeStruct<i32>::some_function` and e.g.

````
    struct SomeStruct<T> {...}

    impl<T> SomeStruct<T> {
        fn some_function(&self, arg: SomeThing) {
        ...
    }
````

   would result in `SomeStruct<T>::some_function`

 - Functions inside trait `impl` blocks would use, enclosed in angle brackets,
   the name of the type, followed by `as`, followed by the trait name, and
   then `::` followed by the name of the function, e.g.

````
    impl SomeTrait for SomeStruct {
        fn some_function(&self, arg: SomeThing) {
        ...
    }
````

   would result in `<SomeStruct as SomeTrait>::some_function`

 - Closures are using the name of the surrounding function, followed by `::`
   and the string `<closure>`. As closures are not named, coming up with a
   meaningful name for them automatically is not possible.

# Drawbacks
[drawbacks]: #drawbacks

Any addition to the standard library will need to be maintained forever, so it
is worth weighing the maintenance cost over the value added. Given that this
is a feature that is considered useful in other languages (e.g. `__FUNC__` in
C) and is widely used there, it seems to be a useful addition to Rust too.

Also adding a new macro to the standard library will make it impossible to use
a macro with the same name in user code, because macros are (as of now) not
namespaced.

# Alternatives
[alternatives]: #alternatives

There are no alternatives to implement this in user code at this point, it
would have to be provided by the compiler and standard library.

# Unresolved questions
[unresolved]: #unresolved-questions

 - Different name for the macro
   - `function` would be nicer but easily conflicts with existing code
   - `fn` would be consistent with how functions are declared
   - `function_path`, makes more sense if the `module_path` would be
     prepended (see below)
 - Should a different naming scheme be used?
   - An alternative would be using the naming scheme of backtraces, but
     that is implementation defined and more disconnected from the code
 - Should `module_path` be prepended or not?
   - Seems redundant and easy to get in any case via the other macro
 - A macro or intrinsic?
   - Macro would be more in line with existing, similar macros. As such more
     discoverable and consistent
   - Intrinsic would not pollute the global namespace
   - Intrinsics are (currently) not stable
 - Are there better ways of naming closures?
