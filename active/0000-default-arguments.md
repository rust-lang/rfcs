- Start Date: 2014-06-14
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add the support for default function arguments and named function arguments in Rust.

# Motivation

There have been demand for a long time for default and named arguments in Rust. They cover various use cases:

* API design. Some functions in the Rust standard distribution libraries or in third party libraries have many similar methods that only vary by the presence of an additional parameter. Having to find and to memorize different names for them can be cumbersome.

* If this feature can be added to Rust, it can't be added post-1.0, as a lot of functions in the Rust standard library must be rewritten in order to benefit this feature.

* Foreign bindings. This has been discussed on the mailing list recently about Qt5 bindings. Binding functions that make heavy use of default arguments or operator overloading can be very messy with the current system.

# Detailed design

## Split example

Instead of:

```rust
fn split(s: &str, sep: char) { ... }
fn splitn(s: &str, sep: char, count: uint) { ... }

split("hello world", ' ');
splitn("hello world", ' ', 2);
```

We can have:

```rust
fn split(s: &str, sep: char = ' ', count: Option<uint> = None) { ... }

split("hello world"); // -> split("hello world", ' ', None)
split("hello,world", sep: ','); // -> split("hello,world", ',', None)
split("hello world 42", count: Some(2)); // -> split("hello world", ' ', Some(2))
```

## Practical design

> I'll refer to function/closure/proc declaration/call as "function declaration/call" in order to avoid repetitions.

### Function declaration

In function declaration, you can specify a default value for arguments.

The default value must be a static constant and having the same type as the argument.

All arguments can have a default value or not, not only the last arguments (as in C++). As any other function, the arguments without default value must be set, either by setting previous argument or by setting it with named argument.

```
fn_args ::= fn_arg (',' fn_args)? ;
fn_arg ::= ident arg_type? default_value? ;
arg_type ::= ':' type ;
default_value ::= '=' static_expr ;
```

`Example:`

```rust
fn toto(a: uint, b: uint) { ... }
fn toto(a: uint, b: uint = 5) { ... }
fn toto(a: uint, b: uint = 5, c: uint) { ... }
//                       ~~~~~^~~~~~~
// This argument MUST be set at call-time
// Either by setting previous argument, or
// by setting this argument with named arg syntax.

let f = |a| ...;
let f = |a, b = 1u| ...;
let f = |a, b: uint = 1| ...;

let f = proc(a: uint) { ... };
let f = proc(a: uint, b: uint = 1) { ... };
```

### Method declaration

When implementing a trait method using default values for arguments, you don't have to rewrite them.

If you do, it will result in a compilation error.

```rust
trait World {
    fn split(&self, count: uint = 2);
}

struct MyWorld;

impl World for MyWorld {
    fn split(&self, count: uint) {
        // Split the world...
    }
}

struct BadWorld;

impl World for BadWorld {
    fn split(&self, count: uint = 3) {
        // Compile-time error, as you redefine the default value.
        // Note that this will result in an error even if the default value
        //  indicated here is the same that the one present in trait definition.
    }
}
```

### Function call

In function call, you can omit arguments with default value as if the function doesn't have these arguments, they will be added at compile-time.

> Note that you MUST set arguments which don't have default values.

You can set specific arguments by naming it and set a value with a `<name>: <value>` syntax, even those without default value.

```
call_args ::= call_arg (',' call_args)? ;
call_arg ::= expr_arg | named_arg ;
expr_arg ::= expr ;
named_arg ::= ident ':' expr ;
```

`Example:`

```rust
fn toto(a: uint) { ... }
toto(); // Compile-time error
toto(1);

fn toto(a: uint = 1) { ... }
toto(); // Resolved as toto(1)
toto(2);

fn toto(a: uint, b: uint = 2) { ... }
toto(1); // Resolved as toto(1, 2)
toto(1, 3);
toto(1, b: 5); // Resolved as toto(1, 5)
toto(b: 6, a: 2); // Resolved as toto(2, 6)

fn toto(a: uint, b: uint = 2, c: uint) { ... }
toto(1); // Throw an error, because c is not set here.
toto(1, 2, 3); // We set `b` so we can set `c` too.
toto(1, c: 22); // Resolved as toto(1, 2, 22)
toto(1, c: 22, 3); // Resolved as toto(1, 3, 22)
                   // Same as std::fmt format string definition.
toto(b: 123, c: 456, a: 789); // Resolved as toto(789, 123, 456)

fn create_window(title: &str = "My window", size: (uint, uint) = (800, 600)) { ... }
create_window(); // Resolved as create_window("My window", (800, 600))
create_window(size: (1024, 768)); // Resolved as create_window("My window", (1024, 768)

fn split_str(s: &str, sep: char = ' ') { ... }
split_str("hello world"); // Resolved as split_str("hello world", ' ')
split_str("hello,world", sep: ','); // Resolved as split_str("hello,world", ',')
```


## Function signature

The named arguments feature modify the way functions signature is determined, as the ABI shouldn't be changed a lot when modifying function declaration.

The function signature change when:

- Modifying arguments order, as before.
- Modifying arguments names, because of named arguments feature (as we have to keep arguments names in function signature)
- Modifying arguments default values, because of default arguments features (as we have to keep the default value in function signature)

The function ABI change only when modifying arguments order, as the arguments names and default values are "lost" when the function call is compiled.

## Effects in the standard library

With this feature, some functions in the Rust standard library could be rewritten, either by providing better usage or by deduplicating some functions (e.g. `split` functions)

- Merge `std::str::StrSlice::split` and `std::str::StrSlice::splitn` in one `split` method (idem for `rsplit`)
- Merge `std::str::StrVector::concat` and `std::str::StrVector::connect` in one `concat` method.
- And so on...There is many points in the standard library that can be rewritten in order to benefit this feature.

# Drawbacks

Some people have shown reticence to include those features in the language. Having different names for functions that have different signatures can improve readability.

Default and named arguments are maybe not enough to solve the binding problem, especially for libraries that use a lot of function overloading (? not sure about this).

# Alternatives

* Don't do this.
* Add only keyword arguments, or only default arguments.

# Unresolved questions

* Should we accept only last-arguments for default/named arguments, like C++ do?
This could forbid the following syntax:
```rust
fn toto(a: uint, b: uint = 2, c: uint) { ... }
```
* Should we accept non-static expression (limited to declaration scope) in default argument value?
This could allow the following syntax:
```rust
fn slice(&self, start: uint = 0, end: uint = self.len()) { ... }
// Allowed, self is in declaration-scope

fn slice(&self, start: uint = 0, end: uint = toto.len()) { ... }
// Forbidden, toto is not in declaration-scope
```

* Should we "sugarize" `Option`-typed arguments, like:
```rust
fn split(s: &str, sep: char = ' ', count: Option<uint> = None) { ... }

split("hello world"); // -> split("hello world", ' ', None)
split("hello world 42", count: 2); // -> split("hello world 42", ' ', Some(2))
```
> Note that this "sugar" could be generalized for all `Option`-typed values, but it's not the point here.

* For method declaration/implementation, why not letting the implementer struct defining default values, and leave the trait method definition with an "optional" argument (with a special syntax)
