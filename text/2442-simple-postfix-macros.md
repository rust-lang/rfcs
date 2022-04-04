- Feature Name: `simple_postfix_macros`
- Start Date: 2018-05-12
- RFC PR: [rust-lang/rfcs#2442](https://github.com/rust-lang/rfcs/pull/2442)
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

However, unlike macro syntax, postfix syntax can *only* be used by language
extensions; it's not possible to experiment with postfix syntax in macro-based
extensions to the language.  Language changes often start through
experimentation, and such experimentation can also result in sufficiently good
alternatives to avoid requiring language extensions. Having a postfix macro
syntax would serve both goals: enabling the prototyping of new language
features, and providing compelling syntax without having to extend the
language.

Previous discussions of method-like macros have stalled in the process of
attempting to combine properties of macros (such as unevaluated arguments) with
properties of methods (such as type-based or trait-based dispatch). This RFC
proposes a minimal change to the macro system that allows defining a simple
style of postfix macro, inspired specifically by `try!(expr)` becoming `expr?`
and `await!(expr)` becoming `expr.await`, without blocking potential future
extensions. In particular, this RFC does not in any way preclude a future
implementation of postfix macros with full type-based or trait-based dispatch.

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
some variations and not in others. For instance, a macro `do_thing` could allow
both of the following:

```rust
do_thing!(some_expression());
some_other_expression().do_thing!().further_expression().do_thing!();
```

This method-like syntax allows macros to cleanly integrate in a left-to-right
method chain, while still making use of control flow and other features that
only a macro can provide.

A postfix macro may accept `self` by reference or mutable reference, by using a
designator of `&self` or `&mut self` in place of `self`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When expanding a postfix macro, the compiler will effectively create a
temporary binding (as though with `match`) for the value of `$self`, and
substitute that binding for each expansion of `$self`. This stands in contrast
to other macro arguments, which get expanded into the macro body without
evaluation. This change avoids any potential ambiguities regarding the scope of
the `$self` argument and how much it leaves unevaluated, by evaluating it
fully.

For example, given the following macro definition:

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

The invocation in `main` will expand to the following:

```rust
    match value("hello") {
        _internal1 => match ({
            eprintln!("{}:{}: {}: {:?}", "src/main.rs", 14, "value", _internal1);
            _internal1
        }
        .len())
        {
            _internal2 => {
                eprintln!("{}:{}: {}: {:?}", "src/main.rs", 14, "len", _internal2);
                _internal2
            }
        },
    };
```

The first `match` represents the expansion of the first `log_value!`; the
second `match` represents the expansion of the second `log_value!`. The
compiler will generate unique symbols for each internal variable.

The use of `match` in the desugaring ensures that temporary lifetimes last
until the end of the expression; a desugaring based on `let` would end
temporary lifetimes before calling the postfix macro.

A designator of `&self` becomes a match binding of `ref _internal`; a
designator of `&mut self` becomes a match binding of `ref mut _internal`.

Note that postfix macros cannot dispatch differently based on the type of the
expression they're invoked on. The internal binding the compiler creates for
that expression will participate in type inference as normal, including the
expanded body of the macro. If the compiler cannot unambiguously determine the
type of the internal binding, it will produce a compile-time error. If the
macro wishes to constrain the type of `$self`, it can do so by writing a `let`
binding for `$self` with the desired type.

Macros defined using this mechanism follow exactly the same namespace and
scoping rules as any other macro. If a postfix macro is in scope, Rust code may
call it on any object.

A macro may only be called postfix if it is directly in scope and can be called
unqualified. A macro available via a qualified path does not support postfix
calls.

Even though `$self` represents an internal temporary location provided by the
compiler, calling `stringify!` on `$self` will return a stringified
representation of the full receiver expression. For instance, given
`a.b()?.c.m!()`, `stringify!($self)` will return `"a.b()?.c"`. This allows
postfix macros to provide functionality such as `dbg!` or `assert!` that wants
to show the receiver expression.

If passed to another macro, `$self` will only match a macro argument using a
designator of `:expr`, `:tt`, or `:self`.

Using the `self` or `&self` or `&mut self` designator on any macro argument
other than the first will produce a compile-time error.

Wrapping any form of repetition around the `self` argument will produce a
compile-time error.

If the `$self:self` argument does not appear by itself in the macro argument
list (`($self:self)`, with the closing parenthesis as the next token after
`$self:self`), then it must have a `,` immediately following it, prior to any
other tokens. Any subsequent tokens after the `,` will match what appears
between the delimiters after the macro name in its invocation.

A macro may attach the designator `self` (or `&self` or `&mut self`) to a
parameter not named `$self`, such as `$x:self`. Using `$self:self` is a
convention, not a requirement.

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

Rather than writing `expr.macroname!()`, we could write `expr.!macroname()` or
similar, placing the `!` closer to the `.` of the method call. This would place
greater attention on the invocation, but would break the similarity with
existing macro naming that people have grown accustomed to spotting when
reading code. This also seems more likely to get confused with the prefix unary
`!` operator.

In the syntax to define a postfix macro, we could use just `$self` rather than
`$self:self`. `$self` is not currently valid syntax, so we could use it for
this purpose without affecting any existing valid macro. This would make such
declarations look closer to a method declaration, which uses `self` without a
type. However, macros do currently allow `self` as the name of a macro argument
when used with a designator, such as `$self:expr`; this could lead to potential
confusion, and would preclude some approaches for future extension.

We could omit support for `&self` and `&mut self` and only support `self`.
However, this would make `some_struct.field.postfix!()` move out of `field`,
which would make it much less usable.

# Prior art
[prior-art]: #prior-art

The evolution of `try!(expr)` into `expr?`, and the evolution of `await!(expr)`
into `expr.await`, both serve as prior art for moving an important macro-style
control-flow mechanism from prefix to postfix.

# Unresolved questions
[unresolved]: #unresolved-questions

- Is the desugaring of `&self` and `&mut self` correct? Is there another
  desugaring that would work better? What happens if the type is already a
  reference?

# Future work

- We may also want a means of creating postfix proc macros.
