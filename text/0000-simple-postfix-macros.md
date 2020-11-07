- Feature Name: `simple_postfix_macros`
- Start Date: 2018-05-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow simple postfix macros, of the form `expr.ident!()`, to make macro
invocations more readable and maintainable in left-to-right method chains.

# Motivation
[motivation]: #motivation

The transition from `try!` to `?` doesn't just make error handling more
concise; it allows reading expressions from left to right. An expression like
`try!(try!(try!(foo()).bar()).baz())` required the reader to go back and forth
between the left and right sides of the expression, and carefully match
parentheses. The equivalent `foo()?.bar()?.baz()?` allows reading from left to
right.

The introduction of `await!` in RFC 2394 brings back the same issue, in the
form of expressions like `await!(await!(await!(foo()).bar()).baz())`. This RFC
would allow creating a postfix form of any such macro, simplifying that
expression into a more readable `foo().await!().bar().await!().baz().await!()`.

Previous discussions of method-like macros have stalled in the process of
attempting to combine properties of macros (such as unevaluated arguments) with
properties of methods (such as type-based or trait-based dispatch). This RFC
proposes a minimal change to the macro system that allows defining a simple
style of postfix macro, designed specifically for `await!` and for future cases
like `try!` and `await!`, without blocking potential future extensions. In
particular, this RFC does not in any way preclude a future implementation of
postfix macros with full type-based or trait-based dispatch.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When defining a macro using `macro_rules!`, you can include a first argument
that uses a designator of `self` (typically `$self:self`). This must appear as
the first argument, and outside any repetition. If the macro includes such a
case, then Rust code may invoke that macro using the method-like syntax
`expr.macro!(args)`. The Rust compiler will expand the macro into code that
receives the evaluated `expr` as its first argument.

```rust
macro_rules! log_value {
    ($self:self, $msg:expr) => ({
        eprintln!("{}:{}: {}: {:?}", file!(), line!(), $msg, $self);
        $self
    })
}

fn value<T: std::fmt::Debug>(x: T) -> T {
    println!("evaluated {:?}", x);
    x
}

fn main() {
    value("hello").log_value!("value").len().log_value!("len");
}
```

This will print:

```
evaluated "hello"
src/main.rs:14: value: "hello"
src/main.rs:14: len: 5
```

Notice that `"hello"` only gets evaluated once, rather than once per reference
to `$self`, and that the `file!` and `line!` macros refer to the locations of
the invocations of `log_value!`.

A macro that accepts multiple combinations of arguments may accept `$self` in
some variations and not in others. For instance, `await!` could allow both of
the following:

```rust
await!(some_future());
some_other_future().await!().further_future_computation().await!();
```

This method-like syntax allows macros to cleanly integrate in a left-to-right
method chain, while still making use of control flow and other features that
only a macro can provide.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When expanding a postfix macro, the compiler will effectively create a
temporary binding for the value of `$self`, and substitute that binding
for each expansion of `$self`. This stands in contrast to other macro
arguments, which get expanded into the macro body without evaluation. This
change avoids any potential ambiguities regarding the scope of the `$self`
argument and how much it leaves unevaluated, by evaluating it fully.

The `await!` macro, whether defined in Rust code or built into the compiler,
would effectively have the following two cases:

```rust
macro_rules! await {
    ($e:expr) => ({
        // ... Current body of await! ...
    })
    ($self:self) => (
        await!($self)
    )
}
```

Note that postfix macros cannot dispatch differently based on the type of the
expression they're invoked on. This includes whether the expression has type
`T`, `&T`, or `&mut T`. The internal binding the compiler creates for that
expression will participate in type inference as normal, including the expanded
body of the macro. If the compiler cannot unambiguously determine the type of
the internal binding, it will produce a compile-time error.

Macros defined using this mechanism follow exactly the same namespace and
scoping rules as any other macro. If a macro accepting a `$self:self` argument
is in scope, Rust code may call it on any object.

Since `$self` represents an internal temporary location created by the
compiler, calling `stringify!` on `$self` will just return `"$self"`. If passed
to another macro, `$self` will only match a macro argument using a designator
of `:expr`, `:tt`, or `:self`.

Using the `self` designator on any macro argument other than the first will
produce a compile-time error.

Wrapping any form of repetition around the `self` argument will produce a
compile-time error.

If the `$self:self` argument does not appear by itself in the macro argument
list (`($self:self)`, with the closing parenthesis as the next token after
`$self:self`), then it must have a `,` immediately following it, prior to any
other tokens. Any subsequent tokens after the `,` will match what appears
between the delimiters after the macro name in its invocation.

A macro may attach the designator `self` to a parameter not named `$self`, such
as `$x:self`. Using `$self:self` is a convention, not a requirement.

A postfix macro invocation, like any other macro invocation, may use any form
of delimiters around the subsequent arguments: parentheses (`expr.m!()`),
braces (`expr.m!{}`), or square brackets (`expr.m![]`).

# Drawbacks
[drawbacks]: #drawbacks

Creating a new kind of macro, and a new argument designator (`self`) that gets
evaluated at a different time, adds complexity to the macro system.

No equivalent means exists to define a postfix proc macro; this RFC
intentionally leaves specification of such means to future RFCs, for future
development and experimentation. A postfix macro can trivially forward its
arguments to a proc macro.

Macros have historically not interacted with the type system, while method
calls (`expr.method()`) do type-based dispatch based on the type or trait of
the expression. In introducing postfix macros that look similar to method
calls, this proposal does not attempt to introduce type-based dispatch of
macros at the same time; an invocation `expr.m!()` does not in any way depend
on the type of `expr` (though the expansion of the macro may have expectations
about that type that the compiler will still enforce). A future RFC may
introduce type-based dispatch for postfix macros; however, any such future RFC
seems likely to still provide a means of writing postfix macros that apply to
any type. This RFC provides a means to implement that subset of postfix macros
that apply to any type, enabling a wide range of language experimentation
(particularly in readability and usability) that has previously required
changes to the language itself.

# Rationale and alternatives
[alternatives]: #alternatives

Rather than this minimal approach, we could define a full postfix macro system
that allows processing the preceding expression without evaluation. This would
require specifying how much of the preceding expression to process unevaluated,
including chains of such macros. Furthermore, unlike existing macros, which
wrap *around* the expression whose evaluation they modify, if a postfix macro
could arbitrarily control the evaluation of the method chain it postfixed, such
a macro could change the interpretation of an arbitrarily long expression that
it appears at the *end* of, which has the potential to create significantly
more confusion when reading the code.

The approach proposed in this RFC does not preclude specifying a richer system
in the future; such a future system could use a new designator other than
`self`, or could easily extend this syntax to add further qualifiers on `self`
(for instance, `$self:self:another_designator` or `$self:self(argument)`).

We could define a built-in postfix macro version of `await!`, without providing
a means for developers to define their own postfix macros. This would address
the specific issue with `await!`, but would not help developers create
solutions for similar future issues. This would perpetuate the problem of
requiring changes to the language and compiler to solve such problems, rather
than allowing developers to experiment with solutions in the broader Rust
ecosystem.

We could define a new postfix operator for `await!`, analogous to `?`. This
would require selecting and assigning an appropriate symbol. This RFC allows
fitting constructs that affect control flow into method chains without
elevating them to a terse symbolic operator.

Rather than writing `expr.macroname!()`, we could write `expr.!macroname()` or
similar, placing the `!` closer to the `.` of the method call. This would place
greater attention on the invocation, but would break the similarity with
existing macro naming that people have grown accustomed to spotting when
reading code. This also seems more likely to get confused with the prefix unary
`!` operator.

We could do nothing at all, and leave `await!` in its current macro form, or
potentially change it into a language keyword in the future. In this case, the
problem of integrating `await` and similar constructs with method chains will
remain.

In the syntax to define a postfix macro, we could use just `$self` rather than
`$self:self`. `$self` is not currently valid syntax, so we could use it for
this purpose without affecting any existing valid macro. This would make such
declarations look closer to a method declaration, which uses `self` without a
type. However, macros do currently allow `self` as the name of a macro argument
when used with a designator, such as `$self:expr`; this could lead to potential
confusion, and would preclude some approaches for future extension.

# Prior art
[prior-art]: #prior-art

The evolution of `try!` into `?` serves as prior art for moving an important
macro-style control-flow mechanism from prefix to postfix. `await!` has similar
properties, and has already prompted discussions both of how to move it to
postfix and how to integrate it with error handling using `?`.

# Unresolved questions
[unresolved]: #unresolved-questions

- Should we define a means of creating postfix proc macros, or can we defer that?
- Does evaluating `$self` create any other corner cases besides `stringify!`?
