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

If a postfix macro calls `stringify!($self)`, it will get a stringified
representation of the full receiver expression. For instance, given
`a.b()?.c.m!()`, `stringify!($self)` will return `"a.b()?.c"`. This allows
postfix macros to provide debugging functionality such as `dbg!` or `assert!`
that wants to show the receiver expression.

A postfix macro may accept `self` by value, by reference, or by mutable
reference; the compiler will automatically use the appropriate type of
reference, just as it does for closure captures. For instance, consider a
simple macro that shows an expression and its value:

```rust
macro_rules! dbg {
    ($self:self) => { eprintln!("{}: {}", stringify!($self), $self) }
}

some_struct.field.dbg!(); // This does not consume or copy the field
some_struct_ref.field.dbg!(); // This works as well
```

Or, consider a simple postfix version of `writeln!`:

```rust
macro_rules! writeln {
    ($self:self, $args:tt) => { writeln!($self, $args) }
    ... // remaining rules for the non-postfix version
}

some_struct.field.writeln!("hello world")?; // This does not consume the field
some_struct.field.writeln!("hello {name}")?; // So it can be called repeatedly
some_struct.field.write_all(b"just like methods can")?;
```

This makes the `.writeln!(...)` macro work similarly to a method, which uses
`&mut self` but similarly can be called on an expression without explicitly
writing `&mut`. This allows postfix macros to call methods on the receiver,
whether those methods take `self`, `&self`, or `&mut self`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When expanding a postfix macro, the compiler will effectively create a
temporary binding (as though with `match`) for the value of `$self`, and
substitute that binding for each expansion of `$self`. This stands in contrast
to other macro arguments, which get expanded into the macro body without
evaluation. This change avoids any potential ambiguities regarding the scope of
the `$self` argument and how much it leaves unevaluated, by evaluating it
fully.

In the following expansion, `k#autoref` represents an internal compiler feature
within pattern syntax (which this RFC does not propose exposing directly), to
invoke the same compiler machinery currently used by closure captures to
determine whether to use `ref`, `ref mut`, or a by-value binding.

Given the following macro definition:

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
        k#autoref _internal1 => match ({
            eprintln!("{}:{}: {}: {:?}", "src/main.rs", 14, "value", _internal1);
            _internal1
        }
        .len())
        {
            k#autoref _internal2 => {
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

The use of `k#autoref` in the desugaring allows a postfix macro to work in
contexts such as `some_struct.field.mac!()` (in which the macro must accept
`&self` by reference to avoid moving out of the struct field), as well as in
contexts that must take ownership of the receiver in order to function.

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

A macro may only be called as a postfix macro if it is directly in scope as an
unqualified name. A macro available via a qualified path (for instance,
`path::to::macro`) does not support postfix calls. If an imported macro uses
`as` to rename it (for instance, `use path::to::macro_name as other_name`), it
supports postfix calls using the new name, not the original name.

Even though `$self` represents an internal temporary location provided by the
compiler, calling `stringify!` on `$self` will return a stringified
representation of the full receiver expression. For instance, given
`a.b()?.c.m!()`, `stringify!($self)` will return `"a.b()?.c"`. This allows
postfix macros to provide debugging functionality such as `dbg!` or `assert!`
that wants to show the receiver expression.

If passed to another macro, `$self` will only match a macro argument using a
designator of `:expr`, `:tt`, or `:self`.

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

A single macro may define multiple rules, some that use `$self:self` and some
that do not, which allows it to be invoked either as a postfix macro or as a
non-postfix macro. Rules specifying `$self:self` will only match postfix
invocations, and rules not specifying `$self:self` will only match non-postfix
invocations.

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

In the syntax to define a postfix macro, rather than using
```
($self:self, $arg:expr) => (...)
```
we could use
```
$self:self.($arg:expr) => (...)
```
. This would have the advantage of looking more similar to the invocation, but
the disadvantage of looking much different than method definition syntax (in
which `self` appears within the function arguments).

In the syntax to define a postfix macro, rather than using
```
($self:self, $arg:expr) => (...)
```
we could use
```
($self:self. $arg:expr) => (...)
```
or
```
($self:self $arg:expr) => (...)
```
. Using a `.` might be evocative of method syntax, but would be unusual and
harder to remember. Using no delimiter at all would reflect that the `,` does
not actually match a literal `,` in syntax, and might be clearer when using
macros whose arguments use unusual syntax substantially different than method
arguments (e.g. `expr.mac!(:thwack => boing | :poit <= narf)`) but would be
confusing and error-prone for macros intended to resemble method calls (e.g.
`expr.mac!(arg1, arg2, arg3)`). (Making the `,` optional seems even more
confusing and prone to ambiguity.)

We could omit the `k#autoref` mechanism and only support `self`. However, this
would make `some_struct.field.postfix!()` move out of `field`, which would make
it much less usable.

We could omit the `k#autoref` mechanism in favor of requiring the macro to
specify whether it accepts `self`, `&self`, or `&mut self`. However, this would
prevent writing macros that can accept either a reference or a value, violating
user expectations compared to method calls (which can accept `&self` but still
get called with a non-reference receiver).

Rather than requiring a macro to explicitly declare that it works with postfix
syntax using a `:self` specifier, we could allow calling existing macros using
postfix syntax, and automatically pass the receiver as the first argument. This
would work similarly to [Uniform Function Call
Syntax](https://en.wikipedia.org/wiki/Uniform_Function_Call_Syntax), as well as
similarly to the ability to call `Type::method(obj, arg)` rather than
`obj.method(arg)`. Working with all existing macros would be both an advantage
and a disadvantage. This RFC proposes to require macros to explicitly opt-in to
postfix syntax, so that the macro has control over whether it makes sense in
postfix position, and so that the macro can determine what syntax makes the
most sense in postfix position. An explicit opt-in also seems appropriate
given the pre-evaluation behavior of the `:self` specifier, which seems
sufficiently different to warrant a different specifier rather than existing
specifiers like `:expr`.

We could allow postfix macros to omit the delimiters entirely when they have no
arguments. For instance, instead of `a.b()?.c.mac!()`, we could allow writing
`a.b()?.c.mac!`. This would work well when using a macro as a substitute for a
postfix keyword (similar to `.await` or `?`). The `!` would remain, to indicate
the invocation of a macro. However, some macros may prefer to look like a
zero-argument method call (`.mac!()`). Allowing *both* `.mac!` and `.mac!()`
introduces potential ambiguities. If we want to allow `.mac!` in the future, we
could provide an opt-in syntax for the macro to use, which would allow omitting
the delimiter; if, in the future, we determine that we can unambiguously allow
a macro to support *optional* parentheses, we could allow opting into both.
This RFC proposes that we always require the delimiter for now, for simplicity
and to avoid ambiguity.

# Prior art
[prior-art]: #prior-art

The evolution of `try!(expr)` into `expr?`, and the evolution of `await!(expr)`
into `expr.await`, both serve as prior art for moving an important macro-style
control-flow mechanism from prefix to postfix.

# Future work

- We may want to add postfix support to some existing macros in the standard
  library. This RFC does not make any specific proposals for doing so, but once
  the capability exists, the standard library may wish to use it.
- We may also want a means of creating postfix proc macros.
- We *may* want to expose `k#autoref` for other purposes. We may also want to
  use it in the definition of other syntax desugaring in the future.
