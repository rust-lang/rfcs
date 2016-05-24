- Feature Name: eager_expand_macro
- Start Date: 2016-05-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes eager expansion of macros. The primary motivation is providing
a mechanism for creating new identifiers in macros (e.g., by concatenating
identifiers). It is not currently practical to do this because macros cannot be
used in identifier position.

# Motivation
[motivation]: #motivation

A major use case for macros is syntactic abstraction, using macros to
create many, similar items. In current Rust this is impeded by the inability to
create new identifiers. There exists a `concat_idents` macro ([docs](https://doc.rust-lang.org/std/macro.concat_idents!.html)),
but since macros cannot be used in identifier position, it cannot be used when
creating new items.

A straightforward solution would be to allow macros in identifier position.
However, this has some problems, it is ugly and confusing: e.g, `fn foo!(x)(x:
u32)` (the double sets of parentheses are hard to read); since an identifier is
an expression, a macro in expression position can be interpreted as either an
identifier or an expression; and it would make compiler code around identifiers
more complex.

Eager expansion solves these issues by allowing macro uses inside macro bodies
to be expanded before the enclosing macro. Eagerly expanded macros can therefore
be used anywhere inside a macro, including identifier position. This also
restricts usage to macro definitions.

# Detailed design
[design]: #detailed-design

Within a macro definition, the syntax `$!foo(...)` denotes an eagerly expanded
macro use of `foo` (c.f., `foo!(...)`).

The brackets used and the arguments passed to the macro follow the same rules as
other macros. The macro is located by name in the same way as other macros. The
macro may be procedural or by example. In short, `$!foo` behaves in the same way
as `foo!`, modulo expansion order.

Using eager expansion syntax outside a macro is an error, similar to using macro
argument syntax.

## Example

Let's start by considering an example using `concat`, a macro for concatenating
two identifiers to make a new one (similar to today's `concat_idents`). To be
clear, this example macros is not part of this RFC.

Let's imagine we have a struct with a bunch of fields and we want to make
getters and setters for them all because we are feeling nostalgic for Java
Beans. This would make a lot of boilerplate, so instead we'll make an
`accessors` macro to make the getters and setters for us:

```
macro accessors {
    ($x: ident, $t: ty) => {
        pub fn $!concat(get_, $x)(&self) { self.$x }
        pub fn $!concat(set_, $x)(&mut self, a: &t) { self.$x = a; }
    }
}

impl Foo {
    accessors!(name, String);
    accessors!(age, String);
}

fn f(f: &mut Foo) {
    foo.set_name("Peter".to_owned());
    println!("age: {}", foo.get_age());
}
```


## Expansion

An example macro (with no eager expansion):

```
macro foo() {
    ($x: ident) => {
        fn bar($x: i32) {
            baz!($x);
        }
    }
}
```

and macro use `foo!(y)`.

When the parser finds the use, it first looks up the macro and selects an arm by
pattern matching (trivial in this case). Expansion then takes the text from that
arm, replaces arguments, parses the result (which may trigger further expansion),
and splices the resulting AST fragment into the AST, replacing the macro use
(`foo!(y)`).

Text from the example:

```
fn bar($x: i32) {
    baz!($x);
}
```

substitute arguments,

```
fn bar(y: i32) {
    baz!(y);
}
```

parsing causes `baz!(y)` to be expanded (y is parsed, then `baz` is looked up
and expanded),

```
fn bar(y: i32) {
    ...;
}
```

The parsed code is then inserted into the AST in place of `foo!(y)`.

Eagerly expanding macro uses are expanded after substitution of arguments, but
before the macro body is parsed. Arguments to eager macro uses are parsed to do
pattern matching (i.e., before the body is expanded). The eagerly expanded macro
is not parsed until the rest of the (enclosing) macro is expanded.

Example, 

```
macro foo() {
    ($x: ident) => {
        fn bar($x: i32) {
            $!baz($x);
        }
    }
}
```

starting text:

```
fn bar($x: i32) {
    $!baz($x);
}
```

substitute arguments,

```
fn bar(y: i32) {
    $!baz(y);
}
```

eager expand `baz` (no parsing),

```
fn bar(y: i32) {
    ...;
}
```

then parse and splice into the AST.


## Details

### Ordering

Non-nested, eagerly expanding macro uses are expanded in the same order as other
macro uses, i.e., the order in which they are encountered by the parser.

Non-eager macro uses created by eager expansion are not immediately expanded,
and are expanded when the enclosing macro is parsed.

Eager macro uses created by eager expansion are expanded once the first round of
eager expansion is finished. Expansion continues breadth-first until there are
no unexpanded eager macro uses.

Eager macros should never be created by non-eager expansion.

Non-eager macro uses in arguments to eager macro uses are not expanded when the
arguments are parsed. They will naturally be expanded at the same time as other
macro uses in the enclosing macro body. Eager macro uses in macro arguments will
be expanded before the outer macro use, even if the outer macro use is also
eager. I.e., eager macro expansion progresses inside out.

Examples:

`foo!($!bar())` outside a macro is a syntax error.

`foo!($!bar())` inside a macro, `bar` is expanded first, `foo` is expanded later
when the macro body is parsed.

`$!foo(bar!())` `foo` is expanded first, `bar` is expanded at the same time as
other macro uses in the enclosing macro.

`$!foo($!bar())` `bar` is expanded first, then `foo`, then the rest of the enclosing macro.


### Hygiene

This section depends to some extent on rules for item hygiene which haven't yet
been formally proposed. An RFC for that should appear soon.

An eagerly expanding macro has a hygiene context somewhere between expanding the
macro at the enclosing macro's use site and expanding it at the actual use site.
The effect should be that eager expansion has very similar hygiene properties to
regular expansion. A little more precisely, it takes 'macro' scoping information
from the enclosing use site, but lexical scoping information from the actual use
site.

When looking up a macro to be eagerly expanded, we use the enclosing macro's
context. Specifically, the very start of the enclosing macro, not the location
of the actual use.

Examples:

```
macro foo {
    () => {
        macro bar { ... }  // bar 1

        $!baz();
    }
}

macro bar { ... }  // bar 2

macro baz() {
    () => {
        bar!();
    }
}

foo!();
```

Expansion of this program results in the body of `bar 2` because `bar 1` gets
marked by the expansion of `foo`, but the use of `bar` from the expansion of
`baz` does not.


```
macro foo {
    () => {
        mod a {
            $!bar();
        }
    }
}

macro bar { ... }

foo!();
```

which expands without error to

```
mod a {
    ...
}
```

because we look up `bar` in the context of `foo`'s definition.

### Hygiene implementation

This section builds assumes a [sets of scopes macro hygiene implementation](http://ncameron.org/blog/sets-of-scopes-macro-hygiene/).
Very briefly, each identifier is given a set of scopes, where a scope is an
opaque marker, sometimes representing a span of source code. To find a binding
for a name we first match names, then find the binding with the largest subset
of scopes.

When a macro use is expanded, the tokens in the macro body are assigned an
introduction scope. That scope is unique for each expansion. When the expanded
macro is parsed, we apply the set of pending scopes for the macro definition. As
we parse, we add scopes due to constructs in the macro definition.

Scopes from the source code have two components - called inside and outside edge
scopes. If an identifier is written in a scope it gets both edge scopes. If it
is expanded into a scope, it gets only the inside edge scope. Example,

```
macro foo {
    () => { a::bar!(); }
}

mod a {
    macro bar {
        () => { let b = 42; b }
    }
}

fn main() {
    foo!();
}
```

There is a 'global' scope due to the enclosing crate. The following identifiers
have scope sets:

* `foo: {in_crate, out_crate}`
* `a: {in_crate, out_crate}`
* `bar: {in_crate, out_crate, in_a, out_a}`
* `main: {in_crate, out_crate}`
* `foo!: {in_crate, out_crate, in_main, out_main}`

For the macro definitions, the pending set is given by the set of scopes on the
macro name.

Looking up the binding for `foo!` is trivial. After one step of expansion we get

```
fn main() {
    a::bar!();
}
```

`a` has scope set `{in_crate, out_crate, intro_foo1, in_main}`, so looking it up is
straightforward. We then use `a`'s scopes to look up bar.

After the next step of expansion we get

```
fn main() {
    let b = 42;
    b
}
```

Both uses of the name `b` have the same set of scopes:
`{in_crate, out_crate, in_a, out_a, intro_bar, in_main, in_b, out_b}`.

For the eager expansion case, there is a step before expansion - we must find
the macro to expand. Since we haven't parsed the enclosing macro, we can't use
the hygiene information from the eager macro use site. That means we look up the
eager macro in the context of the enclosing macro.

When we expand the eager use, we still use the pending set of scopes for the
macro being expanded, but not for the enclosing macro. We also add introduction
scopes for the macro being expanded, but not the enclosing macro. In these ways
eager expansion mimics regular expansion (i.e., the resulting sets of scopes are
similar). When we parse, we add scopes as normal. Note that since identifiers
are expanded before parsing, they will get both inside and outside edge
scopes (c.f., regular expansion). Finally, because we can't parse the eagerly
expanded macro until we have expanded the enclosing macro, we must keep the
pending scope set for the eagerly expanded macro around longer than usual.
Example:

```
macro foo {
    () => { $!a::bar(); }
}

mod a {
    macro bar {
        () => { let b = 42; b }
    }
}

fn main() {
    foo!();
}
```

The starting scope sets are the same as before. The first expansion step is to
expand `$!a::bar()`. We get something like

```
macro foo {
    () => { let b = 42; b }
}
```

Where the `b`s have the `intro_bar` scope and we keep the pending scopes for
`bar` around. Then we expand `foo!()`, we get

```
fn main() {
    let b = 42;
    b
}
```

with `b: {in_crate, out_crate, in_a, out_a, intro_bar, in_main, in_b, out_b}`.
The final scope set is the same as for regular expansion.


# Drawbacks
[drawbacks]: #drawbacks

Adds another moving part to the macro system.

Since we try to keep behaviour as close as possible to regular expansion, it may
not be obvious when to use either flavour.


# Alternatives
[alternatives]: #alternatives

We could special case `concat_idents`, that is the primary motivation for eager
expansion. It is not clear that eager expansion pulls its weight without
`concat_idents`. The special case would probably be more ergonomic, however,
it is also less flexible and I expect uses might arise in the future.

Expansion order: it might be reasonable to expand eager macro uses outside in.


# Unresolved questions
[unresolved]: #unresolved-questions

Is there a better syntax for this?

The current `concat_idents` macro should be replaced, it is not flexible enough
with regards to hygiene. I leave a proper specification for another RFC.
