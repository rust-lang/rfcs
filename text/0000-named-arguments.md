- Feature Name: `named_arguments`
- Start Date: 2020-07-21
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add named arguments to functions. Functions can have both positional and named arguments. In function calls, named arguments can be specified with their name to increase readability and maintainability.

# Motivation
[motivation]: #motivation

Rust strives to be maintainable for large code bases. Named arguments would allow some function calls to be more explicit and thus easier to read and write.

Let’s consider the following fictive examples:

```rust
http.get("https://www.rust-lang.org/", None);

Window::new("Hello", 20, 50, 500, 250);

free_fall(100.0, 0.0, 9.81);
```

Those are all functions that I could see written in Rust. Someone calling those functions has to remember the order in which the arguments appear. And at first sight, they can't really tell what the arguments stand for.

A common workaround is to use longer function names, e.g. `get_with_timeout`, `new_with_bounds`, `free_fall_with_z0_and_v0_and_g`. However, function names that long are often considered ugly, and in the case of `new_with_bounds`, the order of arguments is still unclear.

Now let’s consider calling the same functions with named arguments:

```rust
http.get("https://www.rust-lang.org/", .timeout = None);

Window::new(.title = "Hello", .x = 20, .y = 50, .width = 500, .height = 250);

free_fall(.z0 = 100.0, .v0 = 0.0, .g = 9.81);
```

It is now very clear what the arguments stand for, which makes maintaining such code easier.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Each function argument can be _named_ or _positional_. Positional arguments are identified by their argument position in function calls, named arguments are identified by their name:

```rust
positional_args(1, 42, "world", true);

named_args(.x = 1, .y = 42, .hello = "world", .b = true);
```

## Function definition

A named argument has an _argument name_ and an _argument pattern_. The argument name is part of the function's API, it's what is used _outside_ of the function. In contrast, the argument pattern is used _within_ the function.

Usually, the argument pattern is the same as the argument name. The following function has a named argument `.x`, which is bound to a variable `x`:

```rust
fn identity(.x: i32) -> i32 { x }
```

It is also possible to specify a different pattern. This pattern can be used to rename or destructure the argument, for example:

```rust
fn scale(.point Point(x, y): Point, .by coeff u64) -> Point {
    Point(coeff * x, coeff * y)
}

scale(.point = Point(14, 21), .by = 4);
```

This also allows ignoring the argument (e.g. `.arg _: i32`) or making it mutable (e.g. `.arg mut arg: i32`).

If a function has both positional and named arguments, the named arguments must come _after_ the positional arguments:

```rust
fn good(a: i32, .b: i32, .c: i32);

fn bad(.a: i32, b: i32, .c: i32); // error!
```

The `self` argument of a method can't be a named argument.

### Trait implementations

If the function is in a trait implementation, each argument must either be positional, or its name must match the argument name in the trait:

```rust
trait Trait {
    fn f(.a: i32, .b: i32);
}
impl Trait for () {
    fn f(_: i32, .b _: i32) {} // okay, because 1st argument is positional and
                               // 2nd argument name matches the trait definition
}
```

The following is forbidden, however:

```rust
trait Trait {
    fn f(_: i32, .b: i32);
}
impl Trait for i32 {
    fn f(.a: i32, .c: i32) {} // ERROR! 1st argument is not named in the trait definition
                              // ERROR! 2nd argument `.c` is named `.b` in the trait definition
}
```

When calling a trait method, only the argument names in the trait definition are considered. See [this section](#calling-trait-functions) for an example.

## Function call

When calling a function, named arguments can be specified with their name. However, named arguments can't be followed by positional arguments. For example:

```rust
fn foo(a: i32, .b: i32, .c: i32) {}

foo(.a = 1, .b = 2, .c = 3); // ERROR! 1st argument is not named
foo(     1, .b = 2, .c = 3); // ok
foo(     1,      2, .c = 3); // ok
foo(     1,      2,      3); // ok
foo(     1, .b = 2,      3); // ERROR! named arguments can't be followed by positional arguments
```

Both positional and named arguments must appear in the same order as in the function declaration:

```rust
fn foo(a: i32, .b: i32, .c: i32) {}

foo(.b = 1, .c = 2, 3); // ERROR! 1st argument is not named
foo(1, .c = 2, .b = 3); // ERROR! expected argument name `.b`, found `.c`
                        //  note: argument names must appear in the same order as in the function declaration
```

Since the argument names [are not part of the type](#interactions-with-the-type-system), named arguments can only be specified if the function is called directly and not via a function pointer, e.g.

```rust
fn foo(.a: i32, .b: i32) {}

let f: fn(i32, i32) = foo;

f(4, 2);           // ok
f(.a = 4, .b = 2); // ERROR! `f` can't be called with named arguments
```

The same applies to the `Fn*` family of traits:

```rust
fn foo(.a: i32, .b: i32) {}

fn higher_order(f: impl Fn(i32, i32)) {
    f(4, 2);           // ok
    f(.a = 4, .b = 2); // ERROR! `f` can't be called with named arguments
}
higher_order(foo);       // ok
higher_order(|_, _| {}); // ok
```

### Calling trait functions
[calling-trait-functions]: #calling-trait-functions

When calling a trait function, only the trait definition is considered:

```rust
trait Trait {
    fn f(self, .arg: i32);  // named argument
}
impl Trait for bool {
    fn f(self, _: i32) {}   // positional argument
}

true.f(.arg = 42);                    // works
Trait::f(true, .arg = 42);            // works
<bool as Trait>::f(true, .arg = 42);  // works
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Grammar

A function parameter can be either a positional argument (`Pattern : Type`) or a named argument (`.name: Type` or `.name Pattern : Type`):

*FunctionParam* :<br>
 &nbsp; &nbsp; &nbsp; _OuterAttribute_* *FunctionParamInner*<br>
*FunctionParamInner* :<br>
 &nbsp; &nbsp; &nbsp; *ArgName* `:` *Type*<br>
 &nbsp; &nbsp; | *ArgName* *Pattern* `:` *Type*<br>
 &nbsp; &nbsp; | *Pattern*  `:` *Type*<br>
*ArgName* :<br>
 &nbsp; &nbsp; &nbsp; `.` IDENTIFIER

Parameters of call expressions can be preceded by an argument name and an equals sign:

*CallParams* :<br>
 &nbsp; &nbsp; &nbsp; *CallParam* ( `,` *CallParam* )* `,` ?<br>
*CallParam* :<br>
 &nbsp; &nbsp; &nbsp; ( *ArgName* `=` ) ? *Expression*<br>
*ArgName* :<br>
 &nbsp; &nbsp; &nbsp; `.` IDENTIFIER

_Syntactically_, this allows named arguments followed by positional arguments. However, Rust should emit a compiler error when this happens.

## Semantics

Since named arguments are just syntactic sugar, they have no influence on semantics. They also don't influence type inference.

## API compatibility

Making a positional argument named is a backwards compatible change. However, changing or removing the argument name is a breaking change. Only the argument _pattern_ can be changed backwards compatibly.

For example, the following changes are backwards compatible:

```rust
fn foo(a: i32) {}
fn foo(.a: i32) {}
fn foo(.a b: i32) {}
fn foo(.a _: i32) {}
```

## ABI compatibility

Named arguments don't affect the ABI or code generation in any way. For example, you can use named arguments in `extern "C"` functions.

## Interactions with the type system
[interactions-with-the-type-system]: #interactions-with-the-type-system

Named arguments don't interact with the type system, since named arguments are _not_ part of the function's type. For example, the following is invalid:

```rust
fn foo(.a: i32, .b: i32) {}

let f: fn(.a: i32, .b: i32) = foo; // forbidden!
let f: fn(i32, i32)         = foo; // correct

fn higher_order(f: impl Fn(.a: i32, .b: i32)) {} // forbidden!
fn higher_order(f: impl Fn(i32, i32)) {}         // correct
```

Also, named arguments are _not_ involved in type inference.

# Drawbacks
[drawbacks]: #drawbacks

## Minor: Verbosity

In function calls, the syntax is slightly more verbose than in other languages such as Python or Swift:

```rust
// Swift
greet(person: "Tim", alreadyGreeted: true)
// Rust
greet(.person = "Tim", .already_greeted = true)
```

## Minor: Named arguments aren't mandatory

Because named arguments aren't mandatory in function calls, this might encourage bad habits where people don't specify named arguments out of laziness. This is especially bad when the function names are short and unspecific, as in the example above.

However, this can also be seen as a good thing: If named arguments were mandatory, API authors might be hesitant to add named arguments, since they make function calls more verbose. However, if you can choose at every function call if you want to specify argument names, there are no downsides for an API author in making function arguments named.

The main reason why named arguments aren't mandatory is that existing APIs can be converted to use named arguments backwards compatibly. This would not be possible if named arguments were mandatory.

### Remedy: Add a clippy lint

Laziness to specify named arguments can be remedied by adding a clippy lint that warns when a named argument is omitted.

However, it should be allowed to omit the argument name, when it matches the variable, field or call expression that is passed as the argument, for example:

```rust
fn foo(.arg1: i32, .arg2: i32, .arg3: i32) {}

let arg1 = 42;
let s = Struct { arg2: 42 };
fn arg3() -> i32 { 42 }

foo(.arg1 = arg1, .arg2 = s.arg2, .arg3 = arg3()) // no warning
foo(arg1, s.arg2, arg3())                         // no warning
```

## Major: Short function names can conflict

With named arguments, it's often desired to shorten long function names, for example:

```rust
// Before:
fn partition_at_index_by_key<K, F>(&mut self, index: usize, mut f: F) -> ...
where ...;

// After:
fn partition<K, F>(&mut self, .index: usize, .key mut f: F) -> ...
where ...;
```

However, this might conflict with another function called `partition` that doesn't accept any arguments.

This drawback could be remedied with optional arguments; see the _future possibilities_ section below for more details.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why make named arguments opt-in?

In some languages, like Kotlin or C#, all parameters can be used as named arguments. This has severe implications on semver compatibility: Changing an argument name is an API breaking change.

Rust cares _a lot_ about compatibility, so we can't just expose argument names from the public API, which are currently an implementation detail. **This RFC gives the API authors control**: They can decide which arguments are part of the API, and which can be changed backwards compatibly.

This adds a semver constraint that named arguments stay the name, but it's a kind of constraint that already exists with public struct fields. Furthermore, the problem is mitigated by the possibility to change the pattern without changing the argument name.

This also has the following benefits:

* Argument names in traits and trait implementations are consistent
* Arguments that are abbreviated or start with an underscore can be renamed before making them public, e.g. `_dx: f64` can be converted to `.delta_x _: f64`.

## About the syntax

### Why the dot syntax?

This syntax might seem odd at first, but I believe that it is the most ergonomic syntax that can be added backwards compatibly. On the function definition site, it is just one additional character in the simplest case. In function calls, an additional character is necessary to not break backwards compatibility:

* `foo(a: b)` is already used for [type ascription](https://github.com/rust-lang/rfcs/blob/master/text/0803-type-ascription.md)
* `foo(a = b)` is already used for assignments

It would be possible to keep the dot syntax but omit the `=` to make it shorter:

```rust
free_fall(.z0 100.0, .v0 0.0, .g 9.81);
```

Or use a different operator:

```rust
free_fall(z0~ 100.0, v0~ 0.0, g~ 9.81);
```

But I believe that either of these would look too unfamiliar and confusing for many people.

It would be possible to use `:` instead of `=`:

```rust
free_fall(.z0: 100.0, .v0: 0.0, .g: 9.81);
```

This is slightly shorter, since `=` is usually formatted with a space before and after the sign. However, some people dislike how this looks, since the syntax contains many dots.

### Why not add an exception to type ascription?

This would add another surprising corner case to the grammar. I think it would be bad if `foo(!x: bool)` would be parsed as type ascription and `foo(x: bool)` would be parsed as a named argument. This could lead to confusing error messages. Type ascription could be forbidden in function parameters, but this has the same potential for confusion: Why is `foo({ !x: bool })` allowed but `foo(!x: bool)` is not?

### Why not use `=`, break backwards compatibility and make a new edition?

Rust guarantees that all editions are supported forever, and can interoperate with each other to prevent an ecosystem split. When a new edition is released, you are not required to update, it is perfectly fine to stay on the current edition.

However, named arguments that only work in the latest edition impose a problem: If functions with named arguments can't be called in older editions, this causes an ecosystem split. And if they can be called in older editions by leaving out the argument names, this can decrease readability considerably. For example:

```rust
// crate using the 202x edition
/// # Example:
/// ```
/// split("hello world", at = ' ', limit = 2, case_sensitive = true);
/// ```
fn split(string: &str, .at: char, .limit: usize, .case_sensitive: bool) {}

// a dependent crate using the 2018 edition has to write
split("hello world", ' ', 2, true);
```

If this function was written today, it might be named `split_at_with_limit` instead, and an enum would be used instead of a `bool`, so it would be readable without named arguments. However, since it was designed with named arguments in mind, the function name is unspecific, making it very difficult to understand without argument names.

Therefore I believe that named arguments should be supported in all editions.


## Alternative: Make named arguments in function calls mandatory

See the _drawbacks_ section above.

## Alternative: Do nothing

There are many ways how code can be made more readable in the absence of named arguments. However, I consider them "hacks" and "workarounds", whereas named arguments is in many cases the most elegant, expressive and readable solution.

### Parameter hints in IDEs

IDEs can provide hints for function arguments. Both IntelliJ-Rust (the plugin that provides Rust language support in JetBrains IDEs) and rust-analyzer (a Rust language server that is going to replace RLS) can do this. Many Rust programmers use these IDE-powered parameter hints, which proves that there is demand for named arguments, at least when reading code.

Unfortunately, parameter hints are not available when reading code in a blog post, in an online forum, on GitHub or any other website. Furthermore, some editors aren't able to show parameter hints.

Another problem is that IDEs are not perfect. For example, when reading code in a `#[cfg(target_os = "windows")]` module on a Linux machine, neither IntelliJ-Rust nor rust-analyzer can provide parameter hints.

Therefore, I believe that Rust code shouldn't rely on an IDE to be readable and expressive.

### Comments

For example:

```rust
free_fall(/*z0*/ 100.0, /*v0*/ 0.0, /*g*/ 9.81);

free_fall(100.0, 0.0, 9.81); // z0, v0, g

free_fall(
    100.0,  // z0
    0.0,    // v0
    9.81,   // g
);
```

Since Rust code like this appears very rarely in the wild, it appears that most Rust programmers either aren't aware of this possibility, or don't like writing code like this.

### Enums for `bool`

Instead of a `bool` argument, a custom enum with descriptive variant names can be created, for example:

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CaseSensitivity {
    CaseSensitive,
    CaseInsensitive,
}
```

This improves type safety, but it also has disadvantages:

 * It adds boilerplate for the library author
 * The custom enum must be `use`d at every call site, which also adds boilerplate for the library user
 * It doesn't implement every trait `bool` implements
 * Using it is more verbose. For example, `if is_case_sensitive {}` might become `if case_sensitivity == CaseSensitivity::CaseSensitive {}`

### The newtype pattern

The newtype pattern can be used to wrap arguments in structs with descriptive names:

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Width(pub i32);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Height(pub i32);

pub fn set_size(Width(w): Width, Height(h): Height) {}
```

To reduce boilerplate, multiple values can be combined in one struct:

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

pub fn set_size(Size { width, height }: Size) {}
```

This also improves type safety, since a `Size` can't be passed to a function expecting a `Point`. However, this kind of type safety is often not required and not worth the additional complexity and boilerplate.

### The builder pattern

The builder pattern is a pattern where an intermediate object containing all the required parameters is created using method calls. This object is then passed to the actual function, which is also a method of the intermediate object. For example:

```rust
SplitBuilder::new("hello world") // positional arguments go here
    .split_at(' ')               // the following methods emulate named arguments
    .limit(2)
    .case_sensitive(true)
    .finish()                    // the actual function
```

The obvious downside is that this requires a lot of boilerplate. To reduce boilerplate, there are crates with procedural macros, which allow creating builder types in a declarative way. One problem with this is that it adds dependencies and increases compile times; in debug builds, it also affects runtime performance.

The other problem is that all arguments are optional (it doesn't prevent you from calling `SplitBuilder::new("hello world").finish()` directly), and this is not always desired. It is possible to create more intermediate types and transitions that ensure that all required arguments are specified, but this adds more boilerplate (if implemented manually) and complexity.

For large APIs with a lot of functions, using the builder pattern everywhere is not feasible.

### Function names

One workaround to make function calls more expressive is to add more information to the function name. For example:

```rust
// this:
fn split(string: &str, .at: char, .limit: usize, .case_sensitive: bool) {}

// could be written as
fn split_at_with_limit_and_case_sensitivity(
    string: &str,
    at: char,
    limit: usize,
    case_sensitive: bool,
) {}
```

However, the version using named arguments is more readable, it reads almost like a sentence:

```rust
split("Hello, world!", .at = ' ', .limit = 2, .case_sensitive = true)
// vs.
split_at_with_limit_and_case_sensitivity("Hello, world!", ' ', 2, true)
```

### Being realistic

Instead of looking at how code _could_ be written in carefully crafted APIs, we should look at how code is being written _in reality_. Programmers don't always have time to rack their brains over how to create the most beautiful API. They want to get things done.

Named arguments allow iterating quickly without sacrificing readability, because they are dead simple. There's no need to create new types or make up long function names.

This is particularly true for private functions. Since they are not part of the public API, little thought is often given to making them readable. This makes the code harder to maintain as the code base grows. Ideally, both public and private code should be readable and expressive.

## Alternative: Implement structural records instead

Structural records are an active RFC to define anonymous structs. If they are implemented, we'll be able to write:

```rust
fn free_fall({ z0, v0, g }: { z0: f64, v0: f64, g: f64 });
free_fall({ z0: 5.0, v0: 0.0, g: 9.81 });

// equivalent code with named arguments:
fn free_fall(.z0: f64, .v0: f64, .g: f64);
free_fall(.z0 = 5.0, .v0 = 0.0, .g = 9.81);
```

However, there are some major differences:

* Structural records as function parameters are like _mandatory_ named arguments

* Converting positional arguments to a structural record is not backwards compatible

* A structural record emulating named arguments is more verbose, since every identifier is repeated in the function declaration

* Converting positional arguments to named arguments is easier than converting them to a structural record

* While named arguments are just syntactic sugar, structural records are a new kind of type, which can appear anywhere, not just in function parameters.

  (This can be seen both as a benefit and as a downside, since it's more powerful, but also more complex and more effort to implement in the compiler.)

I therefore believe that structural records are not a good replacement for named arguments.

# Prior art
[prior-art]: #prior-art

Named arguments are available in a lot of languages, including _C#, Dart, Kotlin, Python, R, Ruby, Scala, Smalltalk, Swift_ and _Visual Basic_.

Furthermore, in some languages such as JavaScript, named arguments can be emulated with anonymous objects.

### Syntax in other languages

The leading dot is similar to struct initialization syntax in C, e.g. `{ .city = "Hamilton", .prov = "Ontario" }`.

The `=` sign after the argument name is used in several languages that support named arguments, e.g. Python, Kotlin and Scala. Most other languages use a `:` for this instead.

### Rust macros

Some macros in the standard library have a syntax similar to named arguments, for example:

```rust
println!("The answer is {x}{y}", x = 4, y = 2);
```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Should it be allowed to specify named arguments in any order?

# Future possibilities
[future-possibilities]: #future-possibilities

## Optional arguments

With named arguments, it's possible to omit some arguments and use a default value for them:

```rust
fn foo(.a: i32 = 0, .b: i32 = 0, .c: i32 = 0, .d: i32 = 4) {}

foo(.c = 5);
// this function call desugars to:
foo(.a = 0, .b = 0, .c = 5, .d = 4);
```

This is an alternative for the builder pattern, but with significantly less boilerplate.

Optional arguments have been requested many times, and I hope that Rust will support them eventually. They're not included in this RFC to make it as small and uncontroversial as possible.

## Function overloading

It would be possible to have functions with the same name but with a different number of arguments or with different argument names:

```rust
fn foo(a: i32) {
    foo(a, 0);
}
fn foo(a: i32, b: i32) {
    foo(.a = a, .b = b);
}
fn foo(.a: i32, .b: i32) {...}
fn foo(.c: i32, .d: i32) {...}
```

However, I would prefer optional arguments instead of function overloading. Overloadable functions are more powerful than functions with optional arguments, but the additional power is rarely needed and can make code more verbose and harder to understand.
