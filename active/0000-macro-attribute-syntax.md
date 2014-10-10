- Start Date: 2014-10-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

- Change the syntax of macros to `@id(...)`, `@id{...}` or `@id[...]`,
  where `id` is an identifier.
- Change the syntax of attributes from `#[...]` and `#![...]` to
  `@...` and `@!...`, roughly speaking.
- Make it illegal to use a macro name in an attribute.
- Lay out a long-term -- but not detailed and not normative -- plan
  for stabilizing decorators and other user-defined syntax extensions

# Motivation

There are two motivations for introducing the change.

**Free the bang.** The first is to "free up" the `!` sign. This was
initially prompted by aturon's error-handling RFC, but even if we opt
not to act on that specific proposal, it's still worth trying to
reserve `!` and `?` for *something* related to error-handling. We are
very limited in the set of characters we can realistically use for
syntactic sugar, and `!` and `?` are valuable "ASCII real-estate".

Part of the reason for this is that `!` has a long history of being
the sigil one uses to indicate something dangerous or
surprising. Basically, something you should pay extra attention
to. This is partly why it was used for macros, but in truth macros are
not *dangerous*. They can be mildly surprising, in that they don't
necessarily act like regular syntax, but having a distinguished macro
invocation syntax already serves the job of alerting you to that
possibility. Once you know what a macro does, it ought to just fade
into the background (consider `@format`, perhaps one of the most
common macros).

**Decorators and macros.** Another strong motivation is that there is
macros and attributes are already fairly intertwined, and we'd like to
make that connection stronger in the future. Already the most common
attribute -- `deriving` -- is literally nothing more than a macro. The
only difference is that its "input" is the type definition to which it
is attached (there are some differences in the implementation side
presently -- e.g., deriving is based off the AST -- but as discussed
below that distinction should be erased eventually).

**Historical precedent.** The choice of `@` is due to its long history
of being used in "meta ways", typically attributes and
decorators. This makes it the obvious choice to replace
`#[...]`. Using `@` for macros as well follows from the previous two
points.

# Detailed design

## Syntax for macro invocations

Macro invocations would be written in one of the following forms (BNF):

```
MACRO = '@' ID '(' {TT} ')'
      | '@' ID '[' {TT} ']'
      | '@' ID '{' {TT} '}'
```

Here `ID` stands for "some identifier" and `TT` for a token tree.

## Syntax for attributes and inner attributes

Attributes and inner attributes would be written in one of the following forms (BNF):

```
ATTR       = '@' [!] META
META       = ID
           | ID '(' META_SEQ ')'
META_SEQ   = META_ITEM {',' META_ITEM}
META_ITEM  = META
           | ID '=' STRING_LITERAL
```

Here are some examples of legal syntax:

- `@inline`
- `@!inline`
- `@deprecated(message = "reason)`
- `@deriving(Eq)`

Note that some attributes which are legal today have no equivalent:

- `#[deprecated = "reason]` becomes `@deprecated(message = "reason")`

## Macro invocations at the top-level

To make them more consistent with other syntax, macro invocations that
use `()` outside of any fn body must be following a semicolon. This
changes existing practice somewhat, where a `;` is commonly omitted.

For example, the following macro use from the compiler would have to be
changed:

```rust
declare_lint!(WHILE_TRUE, Warn,
              "suggest using `loop { }` instead of `while true { }`")
```

The correct form is now one of the following:

```rust
@declare_lint(WHILE_TRUE, Warn,
              "suggest using `loop { }` instead of `while true { }`"); // <-- Note semicolon
@declare_lint { WHILE_TRUE, Warn,              
                "suggest using `loop { }` instead of `while true { }`" }
```

This RFC does not introduce guidelines as to which form should be
preferred.

The use of `;` here is consistent with other top-level declarations,
which always end in either `;` or `{...}`:

    use foo::bar;
    struct Foo(uint);
    
    struct Foo { field: uint }

## Applying attributes to expressions or blocks

To apply attributes to expressions and blocks, inner attributes can be
used:

    { @!foo ... }
    ( @!foo ... )
    
Using inner attributes also avoids questions of precedence and
resolves some potential ambiguities.

## Naming conflicts between attributes and macros

In preparation for allowing users to define decorator macros in the
future, it will become an error to use a macro name in attribute
position. Hence, the following examples are illegal:

    @format("abc")
    struct Foo { ... }
    
    @macro_rules a_macro { ... }
    
    @a_macro
    struct Foo { ... }

## Resolving ambiguities

When encountered in a module body or in a block in statement position,
`@foo(...)` could potentially represent either a macro invocation or
an attribute on the following statement or item. This ambiguity is
resolved in a fairly straightforward fashion by examining the next
token:

- If the next token is an `@` (beginning of another attribute or
  macro), then `@foo(...)` is interpreted as an attribute. The
  following construction must be a series of attributes attached to an
  item or statement.
- If the next token is one of the following keywords, then the
  `@foo(...)` is resolved to an attribute on the following
  item/view-item:
  - `use`
  - `extern`
  - `static`
  - `const`
  - `fn`
  - `unsafe`
  - `mod`
  - `type`
  - `enum`
  - `trait`
  - `impl`
  - `struct`
- In statement position, if the next token is one of the following
  keywords, then the `@foo(...)` is resolved to an attribute on the
  following statement:
  - `let`
  - one of the semi-colon-free control-flow keywords:
    - `if`, `match`, `while`, `loop`, or `for`
- Otherwise, `@foo(...)` is parsed as a macro invocation in expression
  position.

`@foo{...}` and `@foo[...]` are *always* interpreted as
macro invocations.

In practice, these rules mean that `@foo(...)` macros must be used in
a similar way to function calls (which seems natural). Here are some
examples showing various macros/attributes. `@macro()` is used when
the call would be interpreted as a macro, and `@attr()` otherwise:

    // Attributes may be attached to macro invocations.
    @attr(...)
    @attr(...)
    @macro(....); // Macro invocations as an item terminate with `;`.
    
    @attr(....)
    @macro { ... } // But one can also use `{ .. }`.
    
    fn example_fn() {
        // Attributes may be attached to statements:
        @attr(...)
        let x = @macro(...);
        
        // Here `if` is in statement role (no trailing semicolon),
        // and hence an attribute may be attached:
        @attr(...)
        if some_cond == @macro(...) {
            @!attr(...)
            
            @attr(...)
            @attr(...)
            fn foo() { ... }
            
            let x = (@!attr(...) foo * bar) + zed;
            
            @macro(...);
            @macro(...);
            @macro(...)
        } else {
            @!attr(...)
        }
    }

## A non-normative plan for stabilizing syntax extensions

Right now attributes and macros are quite distinct, but looking
forward it makes sense for them to move closer together overtime. The
following is a high-level plan. This RFC only describes the general
direction: individual steps will require further RFCs to hammer out
the details.

The overall goal is to create a stable interface that allows for
syntax extensions like `@format` as well as decorators like
`@deriving` to be pulled out of the compiler source and into external
packages. This plan has several parts:

1. Centralize on token-trees as the input/output format for both
   macro-rules macros and syntax extensions.
2. Allow attributes to take token trees as input, rather than just
   meta-items.
3. Allow macro-rules to be used to define decorator-style macros.

Here we go into more details on each of these points.

**Token trees as the interface to rule them all.** Currently,
decorators like `deriving` are implemented as a transform from one AST
node to some number of AST nodes. Basically they take the AST node for
a type definition and emit that same node back along with various
nodes for auto-generated impls.  This is completely different from
macro-rules macros, which operate only on token trees. The plan is to
centralize on token-trees as the interface for both. There are two
reasons for this. The first is that we are simply not prepared to
standardize the Rust compiler's AST in any public way (and have no
near term plans to do so). The second is that it allows a single
interface for syntax extensions, whether those extensions are used in
decorator position (in which case the input should always be parsable
as an AST) or if as a macro invocation (in which case the input can be
an arbitrary token tree).
    
Syntax extensions that wish to accept Rust ASTs can easily use a Rust
parser to parse the token tree they are given as input. This could be
a cleaned up version of the `libsyntax` library that `rustc` itself
uses, or a third-party parser module (think Esprima for JS). Using
separate libraries is advantageous for many reasons. For one thing, it
allows other styles of parser libraries to be created (including, for
example, versions that support an extensible grammar). It also allows
syntax extensions to pin to an older version of the library if
necessary, allowing for more independent evolution of all the
components involved.

Roughly speaking, a use of `deriving` like the following:

```rust
@deriving(Eq)
struct Bar { ... }
```

would be translated into an invocation equivalent to:

```rust
@deriving { (Eq), (struct Bar { ... }) }
```

**Attributes taking token trees.** Currently, attribute syntax is
specialized so that attributes only accept "meta items". Eventually we
would like to accept a broader class of inputs. [Issue 3740][3740],
for example, describes a reason to embed type names within an
attribute; there are also motivating reasons to include expressions,
field names, and so forth. The natural generalization is to allow
attributes to accept arbitrary token trees rather than just the
current meta-items.

**Allow decorators to be defined using macro-rules.** Currently,
decorators like `@deriving` must be defined in syntax extensions. Once
decorators are defined in terms of syntax trees, however, it becomes
relatively straightforward to allow them to be defined using
macro-rules as well. This lowers the burden to declaring and creator
decorators, and also helps in cross-compilation scenarios, because the
interpreted nature of macro-rules declarations doesn't require
intermediate object files. The plan is not to allow any macro-rules
macro to be used in decorator position, but rather for macros to "opt
in" to use as a decorator as part of their declaration. This helps to
avoid confusing errors when a missing semicolon causes an a
macro-invocation to be interpreted incorrectly as an attribute.

# Drawbacks and history to this RFC

A [previous RFC][PR] introduced roughly the same plan, though with
some important differences:

1. There was no restriction against in-scope macro identifiers from
   being used as attributes.
2. There was no discussion of block and expression attributes.
3. The rules for resolving ambiguities were expressed differently,
   although the spirit was the same.
   
At the time there were two major objections raised:

1. Macros using `!` feels very lightweight, whereas `@` in prefix
   position feels more intrusive.
2. There is an inherent ambiguity since `@id()` can serve as both an
   attribute and a macro.
   
The first point is equally true (or untrue) with the current RFC.
Clearly, it is a subjective opinion. It is also somewhat at odds with
the historical use of `!` as a means of drawing attention to dangerous
things. In general, it seems that using a light color when performing
syntax highlighting may go a long way towards ensuring that the `@`
feels lightweight.

The second point regarding potential ambiguities is largely addressed
by the rule prohibiting in-scope macro identifiers from being used in
attribute position, as well as careful coverage of the ambiguity
rules. It is also worth noting that the goal of these rules in general
is to make macro invocations behave *more* like existing language
constructs with regard to `;` and other punctuation, not less.

# Alternatives

Here are some alternatives that were considered:

1. Use `@foo` for attributes, keep `foo!` for macros (status quo-ish).
2. Use `@[foo]` for attributes and `@foo` for macros (a compromise).

Option 1 is roughly the status quo, but moving from `#[foo]` to `@foo`
for attributes (which seemed universally popular in the
[previous RFC][PR] discussion). The obvious downside is that we lose
`!` forever and we also miss an opportunity to unify attribute and
macro syntax. We can still adopt the model where decorators and macros
are interoperable, but it will be a little more strange, since they
look very different.

Option 2 is to introduce `[]` to differentiate attributes from macros.
The avoids the ambiguities and makes a clearer syntactic distinction
between attributes and macro usage. The main downside is that
`@deriving(Eq)` and `@inline` follow the precedent of other languages
more closely and arguably look cleaner than `@[deriving(Eq)]` and
`@[inline]`. Given that accidentally using an attribute where a macro
was expected (or vice versa) leads to an error, this strict syntactic
separation was deemed unnecessary.

# Unresolved questions

There are some questions about how strict we should be in terms of
where attributes and macros may appear.

- Can an attribute be attached to an `if` or `while` etc that is *not*
  in statement position? For example: `let x = @attr() if cond {
  .... } else { ... }`. There is no particular ambiguity in parsing
  such an attribute, but allowing it could lead to confusion since
  only expressions that begin with a keyword would permit attributes.
  (Due to ambiguities around `@attr[]`.)

The author is vaguely of the opinion that we ought to be stricter.

[PR]: https://github.com/pcwalton/rfcs/blob/unify-attributes-and-macros/active/0000-unify-attributes-and-macros.md
[3740]: https://github.com/rust-lang/rust/issues/3740
