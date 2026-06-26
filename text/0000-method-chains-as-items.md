- Feature Name: method-chains-as-items
- Start Date: 2026-06-25
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Introduce *method chains* as a native kind of Rust item: a function whose parameter list is
split across a sequence of named, dot-separated sections instead of a single parenthesized list.
Each section has a name and its own parameter list (which may be empty, or hold several
parameters at once) and only the last section carries a return type and a body. For example:

```rust
fn define_movie<'a>(name: &'a str)
    .released_in(release_year: usize)
    .directed_by(director_name: &'a str) -> Movie
{
    Movie {
        name: name.to_string(),
        release_year,
        director_name: director_name.to_string(),
    }
}
```

is called as a sequence of named method calls:

```rust
let movie = define_movie("The Lobster")
    .released_in(2015)
    .directed_by("Yorgos Lanthimos");
```

A method chain call is a single, atomic expression: only the complete chain, written out in
full, is ever a valid call (see Guide-level explanation). This RFC defines only that observable
behavior, not any particular implementation (see Reference-level explanation). Method chains can
be declared as free items, as associated functions inside `impl` blocks, or as required/provided
methods inside `trait` definitions.

This pattern is already available today, via code generation, as the third-party proc-macro
crate [`assemblist`](https://crates.io/crates/assemblist), which this RFC's author also
maintains. This RFC proposes turning the underlying call shape into a compiler-native item,
presenting it as a self-contained, additive alternative to the broader, long-debated proposal of
adding named and optional parameters to Rust.

## Motivation
[motivation]: #motivation

Rust deliberately has no named or optional parameters. Every call site is positional, every
parameter of a function must be supplied, and there is no syntax to label an argument with the
name of the parameter it fills. This is a long-standing, frequently revisited point of friction:
proposals to add named and/or optional parameters surface regularly in the Rust community, and
just as regularly stall. Part of the reason is that many separate sub-questions all need an
answer at once, and reviewers tend to disagree on each one individually: which parameters can be
named, whether they can be skipped, and in what order they can then be supplied. Whatever the
answer, it also has to respect two things that already hold today: an existing call like
`f(a, b, c)` has to keep meaning exactly what it already means, and a function name has to keep
resolving to exactly one item regardless of which arguments are passed, since Rust functions,
unlike some other languages' functions, are not overloaded by argument count or type.

In the absence of named and optional parameters, the idiomatic workaround is to hand-write a
chain of methods (commonly called a builder) that incrementally accumulates the values a
caller supplies before producing the final result. There is no single canonical shape for this:
authors are free to structure it however they like. Two broad strategies recur, though, each with
its own trade-off: mutating a single instance through methods callable in any order, which is
simple to write but only checked for completeness at runtime; or encoding which values have been
supplied so far into the type itself (often called the *typestate* pattern) which pushes that
check to compile time, at the cost of meaningfully more code to design and maintain. Whichever
strategy an author picks, or wherever they land between the two, real costs remain:

- **Boilerplate.** Every constructible type that needs this needs its own hand-written chain of
  methods (or a derive macro tying one to a specific struct's fields) designed and written from
  scratch.
- **A trade-off between simplicity and compile-time completeness.** As above: an author gets one
  property or the other, rarely both, out of a single hand-written type.
- **Documentation spread across pages that aren't individually interesting.** Whatever shape the
  builder takes, a consumer mostly cares about one thing: the complete sequence of calls and what
  they produce together. But that behavior is documented piecemeal, split across the builder
  type's own page and each of its methods', rather than presented as the single, cohesive thing it
  actually is.

Method chains avoid this trade-off entirely, while staying purely additive. They introduce a new
kind of item, they do not change the meaning of any existing function or call expression:

- They generate no boilerplate beyond the chain's own declaration: there is nothing to design or
  hand-write beyond the chain itself (no separate builder type, no type-level tracking of what's
  been supplied so far).
- It is checked for completeness entirely at compile time, with no runtime check and no extra
  type-level machinery needed to get there (see Guide-level explanation).
- The whole chain is documented as a single item, not fragmented across a builder type's page and
  its methods' pages (see Rationale and alternatives for why a macro-based implementation cannot
  achieve this on its own).

Because this RFC proposes a new kind of item rather than new call syntax for existing functions,
it sidesteps the two constraints described above that proposals about named/optional parameters
must consider. Nothing about how `f(a, b, c)` resolves today changes, because the syntax for method
chains just mimics an ordinary chain of method calls and does not require altering the syntax for
individual functions/methods. A method chain's own name still resolves to exactly one item, the same way an
ordinary function's does (see Reference-level explanation): nothing about it depends on which
arguments are passed, so no question of arity-based overloading ever comes up. This still addresses
the same underlying need: letting callers supply grouped, named, and conditionally-shaped sets of
parameters without writing or maintaining a hand-rolled builder.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Think of a method chain as a function whose argument list has been split into a sequence of
named sections. Instead of:

```rust
fn define_movie(name: &str, release_year: usize, director_name: &str) -> Movie { /* … */ }

define_movie("The Lobster", 2015, "Yorgos Lanthimos");
```

you write:

```rust
fn define_movie<'a>(name: &'a str)
    .released_in(release_year: usize)
    .directed_by(director_name: &'a str) -> Movie
{
    Movie {
        name: name.to_string(),
        release_year,
        director_name: director_name.to_string(),
    }
}
```

and call it the same way you'd call any chain of methods:

```rust
let movie = define_movie("The Lobster")
    .released_in(2015)
    .directed_by("Yorgos Lanthimos");
```

Each `.section_name(…)` looks like an ordinary method call, but the whole expression (from
`define_movie("The Lobster")` to the final `.directed_by("Yorgos Lanthimos")`) is really just
one call, spread across several dot-separated sections instead of one parenthesized list. Writing
only part of it, like `define_movie("The Lobster")` alone, or
`define_movie("The Lobster").released_in(2015)` alone, isn't a smaller, valid version of that
call: it's an incomplete one, and the compiler rejects it, naming the section it expected to come
next. There is no point during evaluation where "a movie chain with the name set, waiting for a
release year" exists as something you could store in a `let`, return from a function, or pass to
something else. The chain simply isn't a value at all until every section has been supplied.

This has direct, practical consequences:

- **You cannot skip a section, call sections out of order, call a section twice, or stop
  partway.** Only `define_movie(…).released_in(…).directed_by(…)`, written out in full, in
  that order, in one expression, is something the compiler recognizes as a call at all.
- **An incomplete chain is a compile error, not a runtime concern.** There is no "half-built
  movie" state to accidentally use as if it were finished, because there is no such state to
  begin with (see above).
- **The chain reads top to bottom as documentation of its own parameters.** A caller (or an IDE's
  autocomplete) sees `.released_in(`, gets a hint for `release_year: usize`, then sees
  `.directed_by(`, and gets a hint for `director_name: &str`. Every parameter arrives carrying
  the name its author gave it, without Rust needing named arguments inside a single `(…)` list.

A section's parameter list can hold any number of parameters, including none at all, the way
`.times()` does above. For instance:

```rust
fn begin(i: i32).next().finish(a: i32, b: i32) -> i32 {
    i + a + b
}

begin(1).next().finish(2, 3);
```

### Method chains as methods

A method chain can also be declared inside an `impl` block, optionally starting with a receiver,
exactly like an ordinary method:

```rust
struct PlayList { tracks: Vec<Track> }

impl PlayList {
    pub fn insert(&mut self, index: usize)
        .of_kind(kind: TrackKind)
        .titled(title: String) -> &mut Track
    {
        let track = Track::new(kind, title);
        self.tracks.insert(index, track);
        &mut self.tracks[index]
    }
}

playlist.insert(0).of_kind(TrackKind::Audio).titled("Intro".to_string());
```

Any receiver form an ordinary method accepts today (`self`, `&self`, `&mut self`, or an
explicit-type receiver such as `self: Box<Self>` or `self: Rc<Self>`) can appear as the chain's
receiver, with the one restriction that it must be the very first parameter of the chain's first
section. There's nothing about splitting the rest of the parameter list across sections that
requires restricting how `self` itself is taken.

### What doesn't change

A method chain is declared with the same `fn` keyword, the same visibility modifiers (`pub`,
`pub(crate)`, …), and can be marked `async`, `const`, or `unsafe` exactly like an ordinary function.
`async`, `const`, and `unsafe` apply to the chain as a whole. There's nothing to `.await` until
every section has been supplied and the chain's body actually runs. Reading and reviewing existing
Rust code is entirely unaffected:
nothing about ordinary functions, methods, or call sites changes. A method chain is something a
function's *author* opts into when declaring it; everything else in the language behaves exactly
as it does today.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Grammar

A method chain item has the shape:

```text
chain := [attrs] [vis] [«unsafe»] [«const»] [«async»] «fn» ident [generics] «(» [receiver «,»]
         params «)» ( «.» section )+ «->» ReturnType [where_clause] «{» body «}»

section := ident [generics] «(» params «)»
```

with the following constraints:

- `params`, in any section, may be empty or hold any number of parameters. There is no
  requirement that a section introduce exactly one.
- The optional `receiver` may appear only in the first section, but is otherwise unrestricted:
  any receiver form valid for an ordinary method today (`self`, `&'a self`, `&'a mut self`, or
  an explicit-type receiver such as `self: Box<Self>`) is valid here too, and any future
  extension to the set of receiver forms ordinary methods accept (e.g. further
  arbitrary-self-types work) applies equally to method chains with no separate decision required.
  The only chain-specific restriction is positional: a receiver can only be the chain's very
  first parameter.
- Each section's `[generics]` introduces type and lifetime parameters that are in scope for that
  section's own parameters and return type, and for every later section (exactly like a
  function's generics, but introduced incrementally). A later section's bounds may reference an
  earlier section's generic parameters (e.g. a section taking `n: T` can be followed by a section
  bounded on `T: Display`).
- Only the *last* section carries `-> ReturnType { body }`; every earlier section is purely a
  parameter-list extension of the same underlying call.
- A `where` clause, if present, appears once for the whole chain, after the final section's
  return type and before its body, exactly like an ordinary generic function's. It can bound any
  generic parameter introduced by any section, including ones introduced well before the final
  section. There is no per-section `where` clause, and one cannot be written between two
  sections: collecting every bound at the end, rather than next to the section that introduced
  the corresponding generic parameter, keeps a chain's `where` clause exactly where a reader
  already expects to find one.
- `vis`, `unsafe`, `const`, and `async` apply once, to the chain as a whole, exactly like an
  ordinary function's; there is no per-section visibility, safety, constness, or asyncness in
  this proposal (see Future possibilities for the branching extension, where each alternative
  reintroduces its own `fn` and, with it, its own modifiers). An `unsafe` chain requires an
  `unsafe` block at the call site and runs its final section's body in an unsafe context, the
  same as an ordinary `unsafe fn`. A `const` chain is held to ordinary `const fn` rules over the
  single fused call described in Semantics; the one caveat is that a later section's `where`
  clause bounding an earlier section's generic parameter (e.g. `T: Display`) is, for a `const`
  chain, subject to whatever rules `const fn` already imposes on generic trait bounds today
  (`~const` bounds), independently of this proposal.

### Name resolution

A method chain's first section's identifier lives in the same namespace as an ordinary function,
associated function, or method name. Declaring two method chains (or a method chain and an
ordinary function/method, since an ordinary function is just a method chain with a single
section) with initial sections under the same name in the same scope is exactly as much a
duplicate-definition error as declaring two ordinary functions under that name is today.
Method chains introduce no overloading: a name still resolves to exactly one item, regardless
of how many sections that item's chain has.

### Semantics

A method chain invocation `ident(args0).section1(args1)….sectionN(argsN)` is a single call
expression spanning every section from the first to the last. The language only defines a
meaning for the invocation in this complete form:

- Writing only a prefix of a chain's sections (`ident(args0)` alone, `ident(args0).section1(args1)`
  alone, and so on up to, but not including, the full chain) is not a valid expression in any
  position: not as a statement, not as the right-hand side of a `let`, not as a function
  argument, not as a return value. There is no point during resolution or evaluation at which
  "the result so far" exists as a value of any type, nameable or not, that a program could refer
  to, store, or pass around. A chain that is not completed in one expression is a compile-time
  error naming the section the compiler expected to come next.
- A complete chain invocation behaves exactly as if it were a single call to a function taking
  the concatenation of every section's parameters (plus the receiver, if any) in declared order,
  and running the final section's body. There are accordingly no auto-trait, `Copy`/`Clone`, or
  drop-timing questions to answer for "the result of a non-final section": no such value exists
  for any such property to apply to. The only types that matter to a method chain are each
  section's own parameter types and the chain's final return type, exactly as for an ordinary
  function.
- An ordinary function's bare name is itself a usable value: it can be passed wherever a matching
  `Fn`/`FnMut`/`FnOnce` value is expected just by writing the function's name, with no explicit
  closure required. A method chain's first section's identifier has no such standalone meaning.
  There is no path expression (`ident.section1.section2`, or anything else) that refers to "the
  chain as a callable value," and the bare identifier `ident` on its own, not immediately followed
  by a call that completes the whole chain, is covered by the previous point: it is not a valid
  expression. A caller who wants to pass a method chain somewhere that expects a closure has to
  write one explicitly: `|t, y, d| define_movie(t).released_in(y).directed_by(d)` rather than
  `define_movie` or some hypothetical `define_movie.released_in.directed_by`.

This RFC defines only this observable behavior, and deliberately imposes no implementation
strategy beyond it. A compiler may lower a method chain to a single physical function (the most
direct strategy, since nothing about its semantics requires any intermediate value to exist), to a
sequence of single-method generic types the way today's `assemblist` macro does, or to anything
else that reproduces the same observable behavior. Whichever strategy is used, the language
defines no name for any such intermediate construct: it is never shown to `rustdoc`, never
participates in name resolution, and is not something a Rust programmer is ever expected to
reason about.

The reason for forbidding any use of "the result of a non-final section" (rather than, say,
giving it some real but unnameable type that could still be bound to a `let`) is specifically to
leave the compiler this freedom. If a half-finished chain had to behave as a real, droppable
value with auto-trait properties of its own, a compiler would be forced to actually materialize
*something* to represent it, even when the entire chain is written as one continuous expression
and nothing was ever asked to outlive it. Forbidding any such use entirely means a compiler never
has to materialize anything beyond what an ordinary, single function call already requires, while
still leaving it free to do so internally if that happens to be the simplest way to implement a
particular case.

### Method chains in traits

A `trait` may declare a method chain as a required or provided associated item, in exactly the
shape this RFC otherwise allows, minus a body on the final section for a required one:

```rust
trait Configurable {
    type Output;

    fn configure(&mut self)
        .with_retries(n: u32)
        .with_timeout(d: Duration) -> Self::Output;
}
```

An implementation must restate the same section names and parameter lists (including the
receiver on the first section), exactly as it must restate an ordinary trait method's signature
today, and supplies the body for the final section:

```rust
impl Configurable for Client {
    type Output = ConfiguredClient;

    fn configure(&mut self)
        .with_retries(n: u32)
        .with_timeout(d: Duration) -> ConfiguredClient
    {
        /* … */
    }
}
```

This requires no new conceptual machinery beyond what ordinary trait methods already need. Per
Semantics, `client.configure().with_retries(3).with_timeout(d)` is equivalent to calling a single
trait method `configure_with_retries_with_timeout(&mut self, n: u32, d: Duration) ->
Self::Output`. A trait containing a method chain is therefore object-safe under exactly the same
conditions an equivalent, flattened single-method trait would be: for instance, `Configurable`
above already isn't callable through a plain `dyn Configurable`, but only because it returns the
unconstrained associated type `Self::Output`. That is exactly the restriction that would apply to
any ordinary trait method with that same return type. Splitting the parameter list into sections
introduces no restriction of its own.

### Documentation

This RFC does not prescribe `rustdoc`'s exact rendering, only that a method chain is indexed and
rendered as a single item rather than fragmented across one page per section (see Rationale and
alternatives). One possible rendering of `define_movie`'s page, sketched as a page layout rather
than literal markup, might look something like this:

```text
my_crate :: define_movie . released_in . directed_by         [src]

Method chain define_movie.released_in.directed_by

    «the chain's declared signature, exactly as written, wrapped
     across as many lines as it needs»

    «the chain's doc comment, rendered as markdown, same as today»
```

The title spells out every section's name, not just `define_movie`, because (per Semantics)
`define_movie` alone never identifies a usable item on its own: the thing being documented is the
complete chain. This is purely a rendering choice, though: per Name resolution, only
`define_movie` itself participates in Rust's namespace (it is what a `use` import names, and what
cannot collide with another item). Nothing else about the page is unusual: no "Methods" section,
no entry for `released_in` or `directed_by`, nothing below the description at all.

### What is explicitly out of scope for this proposal

- Calling sections out of order, skipping a section, or supplying a default for a section that
  wasn't called: a method chain is exactly as complete-or-nothing as a single function call with
  all its arguments. See Rationale and alternatives.
- Storing, naming, or otherwise using the result of any section but the last in any way other
  than immediately calling the next section. See Semantics above.
- Branching a section into multiple named alternative continuations, and optional or repeated
  sections. See Future possibilities.

Every example in the Guide-level explanation above is a complete chain invocation in this sense:
`define_movie("The Lobster").released_in(2015).directed_by("Yorgos Lanthimos")` is one call,
equivalent to calling a single function over `name`, `release_year`, and `director_name`
together; `playlist.insert(0).of_kind(TrackKind::Audio).titled("Intro".to_string())` is one call
equivalent to calling a single method over `index`, `kind`, and `title` together with the
`&mut self` receiver.

## Drawbacks
[drawbacks]: #drawbacks

- **A new kind of item to learn and to support.** Method chains add a new way to declare
  something that reads, syntactically, like an extended function signature but behaves like
  neither a function nor an ordinary sequence of method calls. `rustfmt`, `rust-analyzer`,
  `clippy`, and `rustdoc` all need dedicated support to parse, format, navigate, lint, and render
  it; until they do, working with method chains will feel rougher than working with ordinary
  functions.
- **The compiler needs a dedicated diagnostic for incomplete or misordered chains.** Because an
  incomplete or out-of-order chain is a dedicated kind of error rather than an instance of
  ordinary method-not-found resolution, the compiler needs new diagnostic code specifically for
  method chains. This is new compiler-internals work, even though the result (a message naming
  the exact section expected next) can plausibly end up clearer than what today's macro-based
  errors produce.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design's main advantage is that it does not have to reopen, or win, the broader
named/optional-arguments debate to be worth doing: it solves a narrower problem (grouping and
naming a function's parameters, with compile-time-enforced completeness) using only syntax Rust
already has (ordinary method-call chaining), with no new call-site sigil and no change to how an
existing `f(a, b, c)` call is resolved. Whether full named and/or optional arguments are ever
added to Rust, and what shape they would take if so, is a question this RFC deliberately does not
revisit: every attempt at it has stalled for reasons of its own (covered in Prior art) that are
largely orthogonal to whether *this* narrower need is real. The alternatives considered below are
accordingly variations of the method-chain idea itself, not competing solutions to that larger
problem.

### Could a non-final section return a real, unnameable value instead of being atomic?

The most direct alternative to this RFC's atomic-call rule (see Semantics) is to make each
non-final section an ordinary method call in every sense: it would return a real value, of some
unnameable-but-otherwise-ordinary type (much like the type of a closure), that a caller could
bind to a `let`, pass to a sufficiently generic function, or simply drop without ever calling the
next section.

This has a genuine advantage: it behaves exactly like every other method call already does, with
none of the new "this expression isn't valid on its own" carve-out this RFC introduces. It is
also closer to how `assemblist` already behaves today.

Its drawback is exactly what motivated moving away from it: it forces a compiler to actually
generate, and a chain's intermediate calls to actually produce, a real value of a real type for
every non-final section, even when, as in every example in this RFC, the whole chain is written
out in one continuous expression and nothing was ever going to inspect, store, or outlive that
intermediate value. Committing to that as part of the language's observable behavior also commits
to answering, for every method chain, questions this RFC currently avoids needing to answer at
all: what auto traits that intermediate value has, when it is dropped, whether it is `Send`, and
so on, all for a value whose only legitimate use is to immediately call the next section.

### Could sections be reorderable at the call site?

A chain could allow its sections to be called in any order, rather than the single declared order
this RFC requires. This would make method chains read more like the keyword/named-argument call
syntax of #152, #257, #805, and #2964 (see Prior art), and would run into the same problem those
proposals did: #257's review had to invent a dedicated disambiguation rule purely to keep
reordered, partially-named calls unambiguous ("if you alter the declaration parameter ordering in
the middle of a function invocation, you are forced to specify the parameter names for the rest
of the invocation"). This RFC's fixed order avoids needing any such rule: the order at the call
site simply *is* the order at the declaration site. It also keeps the door open for the branching
and optional/repeated-section extensions sketched in Future possibilities, which would be
substantially harder to design and to reason about if sections could additionally be taken in any
order.

### Doing nothing

The status quo remains available regardless of this RFC's fate: the hand-written builder pattern,
in whichever of its shapes (see Motivation), remains the first-party idiom, and third-party crates like
[`assemblist`](https://crates.io/crates/assemblist) remain the only way to get method chains at all today.
Actually, if this already feasible via a crate, why make it a language item at all?

The strongest reason is documentation. Generating a method chain from a proc-macro requires
generating one intermediate type per section, because that is the only way a macro can express
"the result of calling this section is something that exposes exactly one further method, with
this specific signature." That is an implementation necessity, not something the author of a
chain asked for, and not something a caller should need to know or care about. But `rustdoc` has
no way to tell the difference: it documents whatever items the macro expanded to, so every
section of every method chain shows up in generated documentation as its own struct, with its own
page, and its own single inherent method, fragmenting one logical declaration into as many
documented items as it has sections, in a part of the documentation a consumer never needed to
read.

As a native item, a method chain has no such constraint: this RFC defines only the chain's
observable behavior (see Reference-level explanation), and whatever a compiler does internally to
implement it (whether or not that involves generating any per-section types) is never part of
the language's surface. There is nothing for `rustdoc` to fragment a chain's declaration into,
because there is nothing to fragment: the whole chain is documented and indexed as a single item,
the same way a multi-argument function is. This is also the experience callers already get when
reading the chain's declaration or its call sites.

This also sidesteps a smaller, unrelated rough edge of the macro-based implementation: because
`assemblist`'s generated methods are nested inside a generated `impl` block, the macro must rename
the original receiver to `self_` inside the body to avoid colliding with the generated method's
own `self`. Nothing in this RFC requires a compiler-native implementation to reproduce that
workaround; whatever internal representation it chooses, the receiver can be named `self`
throughout, exactly as written.

## Prior art
[prior-art]: #prior-art

The need this RFC responds to is not new: it has been brought to `rust-lang/rfcs` repeatedly,
under several different designs, and every attempt has stalled or been closed.

- **[#152, "optional parameters"](https://github.com/rust-lang/rfcs/pull/152) (2014).** The
  earliest attempt, asking for general optional/default parameters. It carried no specific
  technical objection; it was closed as postponed before 1.0, with the team's position being that
  optional arguments, while a frequently requested feature, are "non-essential" and could always
  be added backwards-compatibly later.
- **[#191, "Currying RFC"](https://github.com/rust-lang/rfcs/pull/191) (2014).** Proposed that
  every function and closure be automatically curried, so that calling it with too few arguments
  would yield a partially-applied closure instead of an error. This is the closest existing
  precedent to splitting a parameter list across several call sections. It was closed specifically
  because reviewers objected to *implicit* currying happening on every call by default: several
  commenters said they would rather have explicit, opt-in currying syntax, and the team closed it
  stating there was no appetite for automatic currying as a language default. The objection that
  sank it was about implicitness, not about the idea of spreading a call across several sections.
- **[#257, "Default and positional arguments"](https://github.com/rust-lang/rfcs/pull/257)
  (2014).** Proposed default values together with named-argument call syntax allowing reordering
  (e.g. `toto(1, c: 22, 3)`). Discussion mostly bikeshed the call-site sigil (`:` vs `=` vs `=>`),
  its ambiguity with a hypothetical future type-ascription syntax, and how default/named arguments
  would interact with the C ABI for `extern` functions. A concern that recurred without
  resolution: once a parameter's name is part of what callers can write, renaming it becomes a
  breaking change, unlike today's freely renameable positional parameters. Postponed before 1.0,
  unmerged.
- **[#805, "Keyword arguments"](https://github.com/rust-lang/rfcs/pull/805) (2015).** A narrower
  keyword-arguments-only proposal. It drew direct pushback that the ergonomic gain was small
  relative to already-available alternatives (newtypes, struct literals, and, explicitly named in
  the discussion, the builder pattern), and that adding new call-site syntax for it risked feature
  creep. Postponed pending further design post-1.0.
- **[#2443, "Reserve `f(a = b)` in Rust 2018"](https://github.com/rust-lang/rfcs/pull/2443)
  (2018).** Not a parameter-naming proposal itself, but an attempt to reserve the `f(a = b)` call
  syntax in the 2018 edition for a future named-arguments feature. It was closed not because named
  arguments were rejected, but because the lang team rejected speculatively reserving syntax ahead
  of any concrete, accepted design: a future proposal could just as easily settle on a different
  syntax, or none at all. This illustrates that even after a concrete design is chosen, claiming
  new call-site syntax for it is its own, separate fight.
- **[#2964, "RFC: Named arguments"](https://github.com/rust-lang/rfcs/pull/2964) (2020).** The
  most recent and most thoroughly discussed attempt (86 review comments), proposing a
  `.name: Type` declaration syntax and `.name = value` call syntax. Recurring objections included
  how to mark a parameter as nameable at the definition site without later renames becoming
  breaking changes, and whether trait methods could gain named parameters without invalidating
  existing implementations. The RFC was formally closed by the lang team with a statement that is
  the single most relevant piece of prior art for this proposal: the team said they were "not
  entirely opposed to the concept of better argument handling" and had been following several
  proposals around "structural types, anonymous types, better ways to do builders, and various
  other approaches," but wanted the underlying problem discussed as a whole, through an MCP,
  rather than accepting one incremental syntactic solution, and no such MCP was being solicited
  that year.

Two threads run through all six discussions. First, every design that introduced new call-site
syntax (`:`, `=`, `=>`, or reserving any of them) spent much of its review re-litigating that
syntax rather than the underlying problem, and in one case (#2443) was closed purely over the
*reservation* of syntax, independently of the feature it was meant to support. Second, marking a
parameter as nameable, and keeping that compatible with later renames, recurred as an unresolved
definition-site design problem across the proposals that got furthest (#257, #2964). Method chains
sidestep both: they reuse ordinary method-call syntax with no new sigils, and a chain section's
name is unambiguously part of the chain's declaration from the start, with no separate opt-in
marker needed. They also align directly with the one concrete opening the lang team has stated
explicitly in #2964's closing comment (interest in "better ways to do builders") rather than
reopening the named-arguments problem space the team declined to revisit piecemeal.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should individual sections be allowed their own doc comments (as opposed to one doc comment for
  the whole chain, on its head `fn`), and if so, how should `rustdoc` weave them into the
  single-item rendering proposed in Reference-level explanation (Documentation)?
- What exact wording and span should the compiler use for an incomplete or misordered chain's
  diagnostic (e.g. "expected `.released_in(…)` to continue this chain")? This is
  implementation/polish work that does not need to be resolved before this RFC is accepted, but
  should be resolved before stabilization.
- Should `rustfmt` define specific conventions for formatting multi-line method chain
  declarations (e.g. how `.section(…)` lines align relative to the head `fn`), and should
  `clippy` add lints specific to method chains (for instance, flagging a single-section chain
  that has no reason not to just be an ordinary function)?
- Can a trait method chain have a *provided* (default) body, and if a default body exists, can an
  implementation override only some of the chain's sections, or must it restate the whole chain
  as a unit the same way overriding an ordinary provided method requires restating its whole
  body? This RFC's trait example only shows a required method chain.

## Future possibilities
[future-possibilities]: #future-possibilities

### Branching method chains

`assemblist` also supports declaring *alternatives*: a section in a chain can branch into several
named continuations, each of which can itself be a further chain, and each branch is free to
return its own type. For example, a single chain starting with a common prefix could let a caller
choose between `.as_get()` and `.as_post()`, where only the latter then requires a body:

```rust
fn new_http_request_to(url: Uri)
    .from<'a>(user_agent: &'a str)
    .with_authorization(authorization: HttpAuthorization).{

    fn as_get() -> GetHttpRequest { /* … */ }

    fn as_post().{
        fn with_text(body: String) -> PostHttpRequest { /* … */ }
        fn with_json(json: JsonValue) -> PostHttpRequest { /* … */ }
    }
}
```

This is deliberately left out of the current proposal to keep its scope minimal: a method chain,
as proposed here, is a single linear sequence of sections with one final return type. Branching
adds real design surface of its own (how branches are named, how exhaustiveness and documentation
work for a tree of continuations rather than a single path, how this interacts with the rest of
the type system) that is independent of the core case for turning linear method chains into a
native item. If this RFC is accepted, a follow-up RFC could extend method chains with branching
continuations along the lines sketched above.

### Optional and repeated sections

This RFC requires every section to be called exactly once. A natural further extension, worth
considering together with branching since both relax the "exactly one fixed linear sequence"
shape, is to let a section be marked optional or repeatable, with the body receiving the
corresponding `Option<T>` or slice of every value the caller supplied:

```rust
fn f(i: i32).g(date: Date)?.h(name: &str)*.build() -> String {
    let optional_date: Option<Date> = date;
    let many_names: &[&str] = name;
    /* … */
}
```

Here `.g(date: Date)?` could be called zero or one times before `.build()`, with `date` bound as
`Option<Date>` in the body; `.h(name: &str)*` could be called any number of times, including
zero, with `name` bound as `&[&str]` collecting every call's argument in call order. Like
branching, this adds real design surface (interaction with per-section generics, what ordering
rules apply between an optional/repeated section and the ones around it, how `rustdoc` and
diagnostics describe a section that doesn't have to be called) that is independent of, and not
required for, accepting linear, exactly-once method chains first.
