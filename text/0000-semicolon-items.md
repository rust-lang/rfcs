# Semicolon Items

- Feature Name: `semicolon_items`
- Start Date: 2018-06-17
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

The semicolon (`;`) is now legal, but not recommended, where an item is
expected, permitting a user to write `struct Foo {};`, among other things.
The `item` fragment specifier in a `macro_rules` matcher will match `;`.
To retain a recommended and uniform style, the tool `rustfmt` will remove any
extraneous `;`. Furthermore, the compiler will fire a warn-by-default lint when
extraneous `;` are encountered, whether they be inside or outside an `fn` body.

# Motivation
[motivation]: #motivation

## Leftover semicolons when refactoring

Various common refactorings often leave behind extraneous semicolons, such as:

* Replacing a closure with a `fn` item:
  
  ```rust
  let add_two = |x| x + 2;
  
  // becomes:
  fn add_two(x: u32) -> u32 {
      x + 2
  };
  ```
  
* Adding a field to what was formerly a unit struct:

  ```rust
  struct UnitExpr;
  
  // becomes:
  struct UnitExpr {
      span: Span,
  };
  ```

* Changing a tuple struct into a braced struct:

  ```rust
  struct Foo(String, usize);

  // becomes:
  struct Foo {
      bar: String,
      baz: usize,
  };
  ```
 
The error emitted by the compiler in these circumstances is never
indicative of a bug, but nonetheless disturbs the flow of writing
and causes the user to have to wait and recompile.
During that time, the user's train of thought can easily be lost.

## Improving consistency with items in `fn` bodies

[compiletest_rs]: https://github.com/laumann/compiletest-rs/blob/master/src/runtest.rs#L2585-L2660

Incidentally, these extraneous semicolons are currently permitted on items
defined inside of `fn` bodies, as a consequence of the fact that `;` in
isolation is a valid *statement* (though not an item).
The following, slightly modified example is from [compiletest_rs].

```rust
fn read2_abbreviated(mut child: Child) -> io::Result<Output> {
    use std::mem::replace;
    use read2::read2;

    const HEAD_LEN: usize = 160 * 1024;
    const TAIL_LEN: usize = 256 * 1024;

    enum ProcOutput {
        Full(Vec<u8>),
        Abbreviated {
            head: Vec<u8>,
            skipped: usize,
            tail: Box<[u8]>,
        }
    };

    impl ProcOutput {
        ...
    };

    ...
}
```

By permitting semicolons as items outside of `fn` bodies,
a modicum of consistency in Rust's grammar can be gained.

## A frequent editing mistake

For the authors of this RFC, who have written Rust code for many years,
it still sometimes happens that they mistakenly place a semicolon after
`struct Name {}`. As previously mentioned, such trivialities needlessly
disrupt the editing process.

## Habits from other languages

Languages like C and C++ require that you write:

```cpp
struct Foo {
    int x;
    ...
}; // <-- You can't omit the ;
```

Since you can't omit the semicolon from the definition above,
that becomes habit. For people who use many languages, or are transitioning to
Rust, not being hindered by such trivialities can help in learning as well
as improving productivity.

## Retaining a uniform style

To retain as uniform of a style possible in the Rust community,
this RFC proposes that `rustfmt`, Rust's code formatting tool,
should remove extraneous `;` since they are most likely left over
while editing or as a frequently made mistake.
No possibility of configuring this behaviour of `rustfmt` is proposed at this time.

A warn by default lint will also be added to the compiler to further
discourage against extraneous `;`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Simply put, the token `;` is now accepted as an item.
This means that you are allowed to write the following:

```rust
struct Foo {
    bar: Baz
}; // <-- NOTE
```

You can also put a `;` after `enum`, `trait`, `impl`, `fn`, `union`,
and `extern` items.

A `macro_rules` matcher using the `item` fragment will now also accept
`;` as an item. For example, given:

```rust
macro_rules foo! {
    ($x: item) => { .. }
}
```

you may write:

```rust
foo!(;)
```

It's important to note that while `;` is now accepted where `item`s are,
this is not intended as the recommended style, but only to improve the
consistency in Rust's grammar, as well as writing flow.

## A warn-by-default lint

Similarly to how the compiler fires warn-by-default lints in against
unused variables, unused `mut`, dead code, the compiler will also, by default,
emit warnings when it encounters extraneous `;`.

The proposed name of this lint is `redundant_semicolon` and will be fired
whenever a semicolon is unnecessary, *except* in the following cases:

```rust
return;
return expr;

break;
break 'label;
break 'label expr;

continue;
continue 'label;
```

where `expr` is some expression and `label` is some label.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

1. The token `;` is accepted as a valid item, both in the language syntax
and by the `item` macro 'fragment specifier'. As an example, `struct Foo {};`
is therefore in the language accepted by a Rust compiler.

2. `rustfmt` will remove any extraneous `;` items.

3. The compiler will provide a warn-by-default lint named `redundant_semicolon`
   which will fire whenever a semicolon is extraneous (can be removed without
   altering the semantics and the well-formedness of a program) except when
   it is placed after a `return`, `break`, or `continue` expression, including
   legal labeled or valued versions of those constructs.

# Drawbacks
[drawbacks]: #drawbacks

The language accepted by a Rust parser is made somewhat more complicated;
this can have some minor effect on parsing performance and will also
complicate the language users will need to understand.
However, we believe that nothing new is actually necessary to be learned
as a redundant `;` in an example such as `struct Foo {};` should already
be intuitive for someone who knows `struct Foo {}`.

# Rationale and alternatives
[alternatives]: #rationale-and-alternatives

## Do nothing

As always, if we conclude that the motivation is not enough,
we can elect to do nothing.

## Improve error messages further

[issue #51603]: https://github.com/rust-lang/rust/issues/51603
[@joshtriplett]: https://github.com/joshtriplett

An idea due to [@joshtriplett] is to improve the error messages
given by `rustc` instead of accepting redundant `;`s.

The current error message when writing `struct Foo {};` is:

```rust
error: expected item, found `;`
 --> src/main.rs:1:14
  |
1 | struct Foo {};
  |              ^ help: consider removing this semicolon
```

This error message is already quite good, giving the user actionable
information. However, the specific case could be improved by recognizing it
specially and saying something like *"don't put a `;` after a `struct { ... }`"*
(same for `union`s and `enum`s). This idea is being discussed in [issue #51603].

However, this does not solve the problem when refactoring,
and neither does it enhance writing flow nor make the grammar more consistent.

## Make `struct F {};` a non-fatal hard error

Another alternative is for the parser to accept `;` as an item, but make it a
hard error after parsing. All the errors of the following form will
be emitted at once, and not one by one:
> "error: expected item, found `;`"

Thereby, the fewer edit-compile cycles will be required and thus the flow
is disturbed less, but still to a degree.

## Add an `--fpermissive` mode to the compiler

Similar to how `gcc` provides the flag `-fpermissive` which treats *all*
"sufficiently recoverable" errors as warnings and continues compilation until
final lib/exe is produced, a Rust compiler could also provide a similar flag.
This could also be done by writing:

```rust
#[allow(non_fatal_errors)]
```
in the source code.

However, such flags have a few drawbacks:
* Flags are less likely to be seen as a requirement of the language
  specification itself. As a consequence, future alternative implementations
  of Rust will have less of an imperative to support such a flag,
  making it more likely to develop language dialects, which we've heretofore
  avoided.

* Not all recoverable warnings are equal, and some may be more indicative of a
  logic error than a redundant semicolon is. Being able to lint against `;`
  specifically and configure that is a more limited change.

* The hassle of remembering to use `-fpermissive` or something like it outdoes
  any gains made by it.

## Do not lint

A more permissive option is to not provide any lint, or make it an
allow-by-default lint. We reject this because:

1. The following unlikely, but problematic, mistake becomes difficult to spot:

   ```rust
   #[my_attr]; // <-- hard to spot!
   fn foo() { ... }
   ```

   Here, the attribute applies to the semicolon, not the `fn`, but that is
   not easy to spot at all.

2. We lint today against superfluous punctuation in the form of parenthesis in
   type and expression contexts. Linting against extraneous semicolons is
   therefore more consistent.

## Interpret `;` as whitespace at the top level

This solution would simply ignore `;` at the module level or inside `impl`s.
This will however have too far reaching negative implications on `;` as a
separator in macros which it is frequently used as.

## Allow an optional `;` after all items

A different technical solution that nonetheless achieves the set out motivations
of this RFC is to allow syntax like: `struct F {};` specifically. This is then
parsed as a single item rather than `struct F {}` and `;` as two separate items.

This is less consistent with the treatment of `;` inside `fn` bodies as well as
in other languages. However, this aspect is not *that* important since most code
will not have `;`s floating around in random places. Therefore, this alternative
also accounts for most problems discussed in the motivation.

In addition, the following macro:

```rust
macro_rules! m {
    ($($item: item);*) => {}
}
```

which today accepts the invocation:

```rust
m!(fn f() {}; fn g() {});
```

would no longer do so if `fn f() {};` is interpreted as one `item`,
because `m!` is expecting a semicolon `;` after an `item`. That semicolon
has now been gobbled, and so the invocation will result in an error.

## Allow extraneous `;` wherever `item`s are allowed

Another technical solution is to allow extraneous `;` wherever `item`s are
allowed. This is different than making `;` an `item` since it affects how
macros work in a slightly different way. Specifically, the token `;` is not
matched by the `item` macro fragment specifier.

# Prior art
[prior-art]: #prior-art

Rust is unusual in that it is a semicolon-terminated language,
yet it has very few kinds of statements that are not expressions in disguise.
This makes direct comparisons to other languages difficult.  

There is however some prior art:

* C++11 [added support](http://en.cppreference.com/w/cpp/language/declarations)
for `;` as an "empty declaration," where prior specifications of the language did not.

* GHC, the Glasgow Haskell Compiler, accepts `;` at the top level.
For example, the following is accepted by GHC:

  ```haskell
  module Foo where
  ;
  ```

+ Java will accept redundant semicolons such as in the following example:
  ```java
  class Main {
      public static void main(String[] args) {
          System.out.println("Hello world!");
          ;;
      }
      
      ;;;
  };;
  ```

+ so too will JavaScript:
  ```javascript
  console.log("foobar");;;
  ```

+ as well as Ruby:
  ```ruby
  require 'sinatra';

  set :protection, :except => :frame_options;;;
  set :bind, '0.0.0.0';;
  set :port, 8080;;;;

  get '/' do ;;
      'Hello world!'
      ;;
  end
  ;;
  ```

# Unresolved questions
[unresolved]: #unresolved-questions

* Does this create parsing ambiguities anywhere?

* Should `;` be an `item` or should some other mechanism be used to achieve
  the same intended effect?

* In which cases will the warn-by-default lint fire
  and in which cases will it not?

* What should the lint in question be called?

* If `;` is an `item`, what is the effect of a doc-comment or an attribute
  on a `;`?
