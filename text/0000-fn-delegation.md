- Feature Name: (fill me in with a unique ident, `fn_delegation`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

This RFC proposes a syntactic sugar for delegating implementations of functions to other already
implemented functions.

There were two major delegation RFCs in the past, the first RFC in 2015
(<https://github.com/rust-lang/rfcs/pull/1406>) and the second one in 2018
(<https://github.com/rust-lang/rfcs/pull/2393>).

The second RFC was postponed by the language team in 2021
(<https://github.com/rust-lang/rfcs/pull/2393#issuecomment-816822011>).
We hope to revive that work again.

How this proposal is different from the previous ones:
- This proposal follows the "prototype first, finalized design later" approach, so it's oriented
towards compiler team as well, not just language team.
The prototyping is already [in progress](https://github.com/Bryanskiy/rust/tree/delegImpl) and we
are ready to provide resources for getting the feature to production quality if accepted.
- This proposal takes a more data driven approach, and builds the initial design on relatively
detailed statistics about use of delegation-like patterns collected from code in the wild. The
resulting design turns out closer in spirit to the
[original proposal](<https://github.com/rust-lang/rfcs/pull/1406>) by @contactomorph than to later
iterations.

# Motivation

This proposal falls under the [Efficient code reuse](https://github.com/rust-lang/rfcs/issues/349)
umbrella.

Rust doesn't have the sort of data inheritance common for object oriented languages, in which
derived data structure can inherit from some base data structure and automatically use its methods
that way.
In Rust this pattern is typically expressed through composition, in which the "base" data structure
is put into the "derived" data structure as a (possibly nested) field or some similar kind of
sub-object.
Newtypes (`struct Derived(Base);`) are an especially popular example of such pattern.

With composition methods that in other languages could be potentially inherited automatically need
to be implemented manually (possibly with help of macros).
Such trivial implementations may create a lot of boilerplate, and even prevent people from using
newtypes when it would be appropriate for type safety.

Some more motivating examples can be found in the two previous RFCs linked above.

This proposal aims to support a sugar that would allow to avoid such boilerplate in cases
similar to inheritance and newtypes, and in other cases too if they fit into the same basic
mechanism, while staying in limited syntactic budget.

# Data driven approach

To drive the design we implemented a compiler pass that collects some metrics about functions
calling other functions.
Based on a combination of such metrics we can detect calls that plausibly look like delegation
from the caller function to the callee function.

The pass uses linting mechanism in rustc and dumps the data into stderr.
Then a processing script parses those compiler logs and puts the resulting data into
[`pandas.DataFrame`](https://pandas.pydata.org/docs/reference/api/pandas.DataFrame.html) in which
they can then be aggregated and analyzed in different ways.
The data contains `file:line:column` locations, so it's possible to find examples of code with
given properties when necessary.
The code can be found [here](https://github.com/petrochenkov/rust/tree/deleglint2).

So far the pass was run on crates from rustc and standard library (`x.py build --stage 2`).
That's about 70-80 crates or different kind, both library and application code.
About 267800 calls was collected, and about 5300 of them (2%) looked like delegation.
We can run this on a larger subset of crates.io crates to collect more data, if requested.

At high level, the "looks like delegation" property is basically a combination of
- whether we can auto-generate it in the compiler,
- whether the inputs and outputs of the caller and callee are compatible enough, and
- the call doesn't already come from some sugar.

The number of delegation-like calls may be overestimated because we didn't consider compatibility
for things like generic parameters and predicates, or ABI.
Hopefully, this overestimation spreads evenly among different categories of delegation discussed
below.

The detailed condition can be found in `fn is_delegation` in the rustc branch linked above.
Some collected data will be described in more detail in the design discussion below.

If we ignore the "source" of the calls then we find that approximately half of the collected
delegation cases come from derives on newtypes (almost all derives was built-in on our sample):

| source        |   count |                                   |
|:--------------|---------|----------------------------------:|
| Source.Derive |    5161 | Generated by a `#[derive]` macro  |
| Source.User   |    4096 | Written directly by user          |
| Source.Bang   |    1198 | Generated by a `fn_like!()` macro |
| Source.Lang   |       1 | Generated by compiler             |

In practice, if you have a trait that provides a corresponding derive macro, and that derive is
sufficient for producing the whole trait impl, then it's preferable to use that derive.
Basically, `#[derive]` is the best delegation sugar, when it's available.
Better delegation support is needed for the remaining cases, not covered by derives like this.
So we remove the derive-generated delegation-like calls from our statistics and base the design on
the remaining cases.

# Syntactic budget

We should fit delegation into some syntax that is no more complex than `use` imports.

```rust
// Import item
#[attrs]
pub(vis) use prefix::{a, b, c as d};

// Delegation item
#[attrs]
pub(vis) delegation_keyword prefix::{a, b, c as d} { target_expr_template }
```

Our goal is to cover the maximum amount of cases that occur in practice within the given syntax
budget.

The specific syntactic shape may be rehashed later after getting sufficient user experience,
although the `use`-like shape seems fine as is.
What is important to stay in the budget is to not add additional knobs to it, like pre-processing
or post-processing closures for inputs and outputs, or function signatures, because in that case
delegated functions will get close to full function implementations in wordiness, while staying
worse than them in readability.

# Design decisions

## Front matter

The usual item harness, attributes and visibility, should be supported on delegation items, like on
all other items.
Some attributes may be added (`inline`?) or inherited from the callee, which exactly is an open
question (no statistics were collected about this).

Unsafety, asyncness, constness and ABI will most likely be inherited from the callee function
(no statistics were collected about these).

## Renaming

Renaming `c as d` should be supported, like in imports.

In the collected data about 40% of delegation instances involve renaming.

| same name      |   count |
|:---------------|--------:|
| SameName.TRUE  |    3135 |
| SameName.FALSE |    2159 |

The renaming feature is trivial, so complexity of the support is negligible compared to the gains.

## Multiple names

Multiple names in a list `delegation_keyword a, b, c` should be supported, like in imports.

About 35% delegation items in the collected data are not alone in their impl (or other parent),
there are other delegation items there too.
<details>
  <summary>Details</summary>

|   N delegation items |   N of imps with this number of delegation items |
|-------------------:|---------------:|
|                  1 |           5124 |
|                  2 |            279 |
|                  3 |             97 |
|                  4 |             72 |
|                  5 |             49 |
|                 10 |             29 |
|                  6 |             28 |
|                  8 |             17 |
|                  7 |             16 |
|                 14 |              8 |
|                 12 |              4 |
|                  9 |              3 |
|                 13 |              2 |
|                ... |            ... |

</details>

The feature should not be hard to support compared to the gains, unless there are some issues with
cloning `target_expr_template`'s IR or something. In any case, the mechanism will be necessary if
we are going to support higher level sugar like delegation all functions in a trait impl at once
with "glob delegation".

## Parent context

Functions delegation should be supported inside both trait impls and inherent impls.
Only about 60% of delegation instances correspond to items in trait impls.

It should also be supported in other contexts too (free functions, default methods in traits),
because why not - all these cases fit into our code generation scheme (see below) and occur in
practice.
Free function support can be sacrificed if it stops fitting into the scheme, though.
A typical free function delegation case collected from stdarch looks like this.
```rust
#[target_feature(enable = "some_feature")]
pub fn foo(arg: u32) { submodule::foo(arg) }

// In delegation form
#[target_feature(enable = "some_feature")]
pub delegation_keyword submodule::foo;
```
Without the attribute difference such "delegation" could just be replaced with a simple reexport
`pub use submodule::foo;`.

| caller parent             |   count |
|:--------------------------|--------:|
| CallerParent.TraitImpl    |    3124 |
| CallerParent.InherentImpl |    1601 |
| CallerParent.Other        |     462 |
| CallerParent.Trait        |     107 |

## First argument transformation aka "target expression"

First argument (often `self`) should supports arbitrary transformations, rather than just
projection to a field (`self.field`).
In other words, the target expression should be an arbitrary expression.

| arg0 preproc         |   count |                     |
|:---------------------|---------|--------------------:|
| Arg0Preproc.No       |    1734 | `arg0`              |
| Arg0Preproc.Field    |    1582 | `arg0.field`        |
| Arg0Preproc.Other    |    1270 | anything else       |
| Arg0Preproc.Getter   |     583 | `arg0.get_field()`  |
| Arg0Preproc.RefField |     136 | `&(mut) arg0.field` |

`self.field` is not even the most common transformation, usually the first argument is not
transformed at all, although delegation for static methods and free functions skews the statistics
in favor of "no transformation" quite a bit.
Getters (method calls without arguments, `self.get_field()`) are also quite common, and other
target expressions are common too.

If all static methods and free functions are filtered away, then half of `Arg0Preproc.No`s go away
and the statistics look like this, with `self.field` getting to the top, but still representing
only 36% of cases.

| arg0 preproc         |   count |                     |
|:---------------------|---------|--------------------:|
| Arg0Preproc.Field    |    1582 | `self.field`        |
| Arg0Preproc.Other    |    1223 | anything else       |
| Arg0Preproc.No       |     833 | `self`              |
| Arg0Preproc.Getter   |     576 | `self.get_field()`  |
| Arg0Preproc.RefField |     136 | `&(mut) arg0.field` |

When reading the previous RFCs, I considered support for arbitrary target expressions a no-brainer,
because correctly supporting just fields is likely not any easier, from an implementation point of
view (that's something the implementation experience may clarify).
However, in the feedback to the original RFC it was suggested to limit delegation support to fields,
@contactomorph disagreed with that suggestion and I disagree with it as well so far.

## Return type compatibility

Return types should initially be the same in the caller and the callee, then we should try
extending the condition to "same up to `Self` type".

| ret match                 |   count |                                                      |
|:--------------------------|---------|-----------------------------------------------------:|
| RetMatch.Same             |    4969 | `fn get_u8() -> u8 { other_get_u8() }`               |
| RetMatch.SameUpToSelfType |     317 | `fn foo(&self) -> Newtype { Newtype(self.0.foo()) }` |
| RetMatch.Coerced          |       8 | `fn get_ref_u8() -> &u8 { get_ref_mut_u8() }`        |

| ret postproc      |   count |                                                      |
|:------------------|---------|-----------------------------------------------------:|
| RetPostproc.FALSE |    4977 | no result post-processing generally allowed          |
| RetPostproc.TRUE  |     317 | but `SameUpToSelfType` requires some post-processing |

### Newtype support

`RetMatch.SameUpToSelfType` is a case that must be supported in some way, because supporting it
means a proper newtype support.
The number of `RetMatch.SameUpToSelfType` cases found in the collected data is also sizable
(if we didn't filter away delegation cases from `#[derive]`s it would take the first place).

It usually corresponds to operations like this.
```rust
impl Clone for Newtype {
    fn clone(&self) -> Newtype {
        Newtype(self.0.clone())
    }
}
```

It's very much plausible to consider this case a delegation, but it is a case in which the return
value post-processing is necessary, but we don't want that post-processing to be written by user.
It is likely that it can be done automatically, without growing the syntax budget (like adding
result post-processing closures).
To support this case we need to figure out how to transform `Self` in return types when inheriting
signatures, and how to add the corresponding value post-processing to bodies.

Perhaps such post-processing can be done through a `From`-like trait that will be implemented
automatically for all newtypes, or at least will be derivable.
If the same trait is implementable manually, then we'll be able to support delegation even for
less matching return types without introducing post-processing closures.

### Trait impls and coercions

If the caller is a function in a trait impl, then we can take the function signature from the trait
instead of inheriting it from the callee.
In that case implicit coercion for the return value (`RetMatch.Coerced`) will also make sense, even
if it's not very common - we generate code without any post-processing, and then type checker fills
in the coercion automatically.

In non-trait-impl cases signatures have to be inherited from the callee (no other place to inherit
them from), so the types will match exactly by construction and no coercions will be supported.

## Non-first argument transformations

Non-first arguments should initially have the same type in the caller and the callee, then we
should try extending the condition to "same up to `Self` type".

| args match                 |   count |                                                          |
|:---------------------------|---------|---------------------------------------------------------:|
| ArgsMatch.Same             |    4913 | `fn take_u8(&self, a: u8) { self.other_take_u8(a) }`     |
| ArgsMatch.SameUpToSelfType |     321 | `fn foo(&self, other: &Self) { self.0.foo(&other.0) }`   |
| ArgsMatch.Coerced          |      60 | `fn take_vec(&self, a: &Vec<u8>) { self.take_slice(a) }` |

| args preproc         |   count |                     |                                                     |
|:---------------------|---------|---------------------|----------------------------------------------------:|
| ArgsPreproc.No       |    4987 | `arg`               | no result pre-processing generally allowed          |
| ArgsPreproc.Other    |     226 | anything else       | but `SameUpToSelfType` requires some pre-processing |
| ArgsPreproc.RefField |      39 | `&(mut) arg0.field` |                                                     |
| ArgsPreproc.Getter   |      34 | `arg.get_field()`   |                                                     |
| ArgsPreproc.Field    |      19 | `arg.field`         |                                                     |

### Newtype support

`ArgsMatch.SameUpToSelfType` is a case that is desirable to support for a proper newtype support.
It usually corresponds to binary operations like this.
```rust
impl Ord for Newtype {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}
```

It's quite plausible to consider this case a delegation, but it is a case in which argument
pre-processing is necessary, but we don't want that pre-processing to be written by user.
It is possible that it can be done automatically, without growing the syntax budget (like adding
argument pre-processing closures).
To support this case we need to figure out how to transform `Self` in argument types when inheriting
signatures, and how to add the corresponding argument pre-processing to bodies.

Perhaps such pre-processing can be done through a `Deref`-like trait that will be implemented
automatically for all newtypes, or at least will be derivable (possibly the same one discussed in
the return type compatibility section above).
If the same trait is implementable manually, then we may be able to support delegation even for
less matching argument types without introducing pre-processing closures.

### Trait impls and coercions

If the caller is a function in a trait impl, then we can take the function signature from the trait
instead of inheriting it from the callee.
In that case implicit coercions for argument types (`ArgsMatch.Coerced`) will also make sense, even
if they are not very common - we generate code without any pre-processing, and then type checker
fills in the coercions automatically.

In non-trait-impl cases signatures have to be inherited from the callee (no other place to inherit
them from), so the types will match exactly by construction and no coercions will be supported.

## Postponed features

Extending the delegation feature from functions to associated types/constants to be able to
delegate all items in a trait impl in one go, or the whole trait impl, is not proposed right now.

Whole trait impl delegation is a second layer of sugar on top of the basic delegation for
individual functions, at least from implementation point of view.
You cannot delegate a whole impl without generating individual delegated functions.

The second layer can be supported later when the base layer is ready.
It should be useful, delegatable functions in trait impls represent 60% of all delegation cases
after all.
In our collected data about 48% of trait impls in which at least one function was delegatable
could be delegated entirely, 38% contained associated types or constants but all functions in them
could be delegated, and 14% could only be delegated partially.
We need to make sure that the selected delegation syntax can be extended to this "glob delegation"
case.

# Rejected features

Complex post-processing of the returned value (`RetMatch.Different`/`RetPostproc.TRUE`) is not
supported, it needs something like an output post-processing closure, which doesn't fit into the
syntax budget.

Complex pre-processing of non-first arguments (`ArgsMatch.Different`/`ArgsPreproc.Other`) is not
supported, it needs something like argument pre-processing closures, which doesn't fit into the
syntax budget.

Delegating to functions with a different number of arguments (`ArgsMatch.DifferentCount`) is not
supported either.

# Generated body

So, the generated function body for a delegation item

```rust
#[attrs]
pub(vis) delegation_keyword name as rename { target_expr_template }
```

will look like this (the first argument may be `self`).
```rust
#[attrs]
pub(vis) fn rename(arg0: Arg0, arg1: Arg1, ..., argN: ArgN) {
    resolved_callee(target_expr_template(arg0), arg1, ..., argN)
}
```

How exactly the `resolved_callee` is obtained is a large separate question, it is discussed below.
Substitution of the `target_expr_template` is also discussed below.

The code generation here is supposed to be "macro-like", that means we generate the code above
(at some high enough IR level), but it is possible that it won't type check and will report errors.
The type checking may also insert implicit code like auto-refs, auto-derefs, or coercions.
The generation cannot happen literally at macro expansion level though, because it needs to inherit
the signature part from the callee (or from some trait), and signatures are not yet ready this
early.

Support for the `SameUpToSelfType` case (limited automatable pre-processing for non-first arguments
and post-processing for the output) is not yet added to this example.

# Implementation and design details

## How the signature is inherited

### Trait impls, refinement, and coercions

If our delegation item is in a trait impl `impl Trait for Type { /*delegate foo*/ }` we have two
opportunities:
- Inherit the signature from `Trait::foo`.
- Inherit the signature from the callee, e.g. `SomeFieldType::foo`.

Right now the signature is always required to match the trait anyway, so there's not much
difference, an error will be reported if the inherited signature doesn't match the trait.

However, with the ["Refined trait implementations" RFC](https://github.com/rust-lang/rfcs/pull/3245)
accepted the situation becomes more complex.
Signature inherited from the callee may be different from the trait, but that could still be fine
due to refinement.

- Inheriting signature from the trait means not supporting refinements.
- Inheriting signature from the callee means not supporting coercions
(`RetMatch.Coerced`, `ArgsMatch.Coerced`, see above).

The `#[refine]` attribute proposed by RFC 3245 could potentially be used for changing the behavior
from one to another. We suggest inheriting signatures from the trait by default.

If our delegation item has any other parent than a trait impl, then we have no other choice than to
inherit the signature from the callee.

### `Self` mapping

If the signature is inherited from the callee, then we need to change the callee's `Self` type to
the caller's `Self` type when copying the signature.

This needs to happen for the `self` parameter at least, but for other parameters and return type
as well if we aim to support the `SameUpToSelfType` cases.

### `self` parameter

Whether the first parameter of a function is `self` is a part of signature (or at least part of
public interface), because only functions with `self` can be called using method call syntax.
Multiple cases are possible when inheriting `self`-ness of the first parameter from the callee.

- If we are in a trait impl and the trait requires `self`, then `self` it is.
- If we are in a trait impl and the trait requires no `self`, then a regular non-self parameter is
used.
- If we are in an inherent impl or a trait, then either `self` or a regular parameter, depending on
whether the callee's `arg0` is `self`.
- If we are in a free function, then a regular parameter is used.

Alternatively, we just could report `self`-ness mismatches as errors.

Note, that the `self`'s *type* is always inherited (modulo `Self` mapping), regardless of the
`self`-ness property (except perhaps in cases when we are in a trait impl and the signature is
inherited from the trait). That is required to inherit complex self types like `Pin` correctly.

### Open questions

Signature inheritance is a source of a few questions that we cannot answer right now, and that will
need to be answered during implementation.

- In particular, how are predicates inherited?
  - If the callee signature refers to generic parameters defined by the callee's parent item, what do
we do with them exactly?

- Are signatures to inherit available early enough?
  - Ideally we would generate delegation items as HIR, but it is not possible at the moment - all HIR
is generated at once and we cannot add more HIR later.
Generating HIR per-item is probably possible, and it's a goal for people working on incremental
compilation, but it may require some larger infrastructure changes in the compiler.
  - Are the signatures available at the astconv time at least?
  - If the callee comes from a trait impl `impl Trait for Type { fn foo() }`, then can we take the
`foo`'s signature from that specific impl, rather than from the `Trait`, early enough?
That would mean supporting trait impl refinements on the callee side.
  - Passes like variance inference are global and require all signatures to be available at once.
Can delegated items be exempted from this global pass and variance-checked later?
Can it cause infinite cycles with delegation item requiring some other items' signature, and that
other item in its turn requiring the delegation item's signature?

- Specific mechanism of the `Self` mapping discussed above is not yet clear an needs to also be
figured out during implementation.

This all will likely need input from people from the types team, and people who implemented
`impl Trait`, which encountered similar issues.

## How `resolved_callee` is determined

We should support delegation in two flavors, that differ in the way they determine the callee
function.

With one flavor the callee path would be specified explicitly, and with another it would be
inferred from `target_expr_template`.
These two flavors can be expressed like this.
```rust
// Explicit paths
delegation_keyword module::name { target_expr_template }
delegation_keyword Type::name { target_expr_template }
delegation_keyword <Type as Trait>::name { target_expr_template }

// Inferred from `target_expr_template`
delegation_keyword name { target_expr_template }
```

Doing this makes sense at least from the implementation staging point of view, but it likely makes
sense from the language point of view as well, because in some cases the callee cannot be inferred
from `target_expr_template` (that includes any delegation to free functions in particular, but
that's not the only case).

### Callee is specified explicitly

This case is simpler for the compiler.
Path to the callee is known, so it is just used in the generated body.

```rust
fn name(arg0: Arg0, arg1: Arg1, ..., argN: ArgN) {
    callee_path(target_expr_template(arg0), arg1, ..., argN)
}
```

### Callee is inferred from `target_expr_template`

#### The proposed algorithm

- We take the expression `target_expr_template.name()` and turn it into a body for type checking.
- Every `self` in it is assigned the type `Self` and treated as `value_of::<Self>()`, i.e. as
"`self` by value".
  - That means target expressions involving `*self` are not supported for callee inference
(unless `Self` itself implements `Deref`). Inferring callees having `Pin` in their `self` will
seemingly not work either. Explicit paths will need to be used in these situations.
- Then the body is type checked with the sole purpose of determining the resolution of `name`.
  - The resolution doesn't depend on arguments passed to the `.name()` call, or even on their number,
so we can resolve such method call without passing any arguments.
  - I think we can resolve static methods this way too, right now `rustc` does it for diagnostics
(if the resolved method doesn't have `self` then an error is reported), but we should be able to
guarantee the precise behavior here too.
This is useful if we want to delegate a list of both static and non-static methods in one go (or to
"glob delegate" the whole trait impl, see postponed features), with inference from
`target_expr_template`.
- Once we have the callee resolution this body is thrown away, type checking results from this
temporary body are not reused in any way when we start type checking the generated code.

The body is then generated in the same way as with explicit paths, and type checked again.
```rust
fn name(arg0: Arg0, arg1: Arg1, ..., argN: ArgN) {
    resolved_callee(target_expr_template(arg0), arg1, ..., argN)
}
```
The only difference is that adjustments on `target_expr_template(arg0)` should be added as if it
was a method receiver, rather than a regular call argument. Otherwise you may have to write
something like `&self.field` in your `target_expr_template` instead of just `self.field`, which is
not good.

#### The questionable alternative

Alternatively, we could generate and type check the body like this
```rust
fn name(arg0: Arg0, arg1: Arg1, ..., argN: ArgN) {
    target_expr_template(arg0).name(arg1, ..., argN)
}
```
, but then `name` could actually resolve to a different function `resolved_callee2`!
(Given smart enough combination of `target_expr_template` and callee signature).
That doesn't seem good, given that the signature is still inherited from the first
`resolved_callee`.

#### What if the inferred callee is a static method?

Another question arises if `resolved_callee` turns out to be a static method.
A simple example:
```rust
// Delegation
impl Newtype { delegation_keyword static_method { self.field } }

// Generated body
fn static_method(arg: u32) {
    FieldType::static_method(arg.field)
}
```
But that's typically not what we want at all!
`self.field` here is used only for inferring the callee, but in the generated body we want to throw
it away and generate this instead.
```rust
fn from_u32(arg: u32) {
    FieldType::static_method(arg)
}
```

The solution is to generate a different body if the combination of these two factors happen:
- Delegation flavor with callee inference is chosen
- The inferred callee doesn't actually have a `self` parameter

Then we generate the alternative body instead
```rust
fn name(arg0: Arg0, arg1: Arg1, ..., argN: ArgN) {
    resolved_callee(arg0, arg1, ..., argN)
}
```
, with `target_expr_template` being thrown away.
It's hard to estimate how common this pattern will be by collecting data from existing code,
because `expr.static_method()` doesn't currently compile and produces and error.

I don't think this case breaks the general rule (`arg0` is transformed using `target_expr_template`), it
rather extends it.
If you need to delegate to a static method, but still transform `arg0`, then the delegation flavor
with explicit path can be used instead.
`arg0` does get transformed occasionally when delegating to functions without `self`
| arg0 preproc       |   count |
|:-------------------|--------:|
| Arg0Preproc.No     |     879 |
| Arg0Preproc.Other  |      47 |
| Arg0Preproc.Getter |       7 |

, so the general scheme may still be useful for this case.

### Going from `target_expr_template` to generated code

As mentioned above, when inferring the callee, all `self`s in `target_expr_template` are
effectively replaced with `value_of::<Self>()`.

Similarly, when generating the final body, all `self`s in `target_expr_template` are replaced with
`arg0` of the generated function (which is often also `self`).

If `arg0` is not transformed, then we could actually skip the target expression part in the
delegation syntax entirely.
```rust
// Explicit path
// For static methods and free functions this looks better than the full form with `{ self }`.
delegation_keyword prefix::name;

// Inferred callee
// Doesn't make sense semantically because `self.name()` will just infinitely recurse.
delegation_keyword name;
```

# Implementation steps

- 1. Support delegation flavor with explicit paths for the callee, and only for paths that can be
resolved early in `rustc_resolve` (that means `module::name` or `Trait::name`).
This will help us to focus on issues with signature inheritance, and get a working feature while
completely putting aside the questions about callee inference.
The callee signature will also be available very early, before even AST -> HIR lowering.
- 2. Same as above, but with support for type-relative explicit paths `Type::name`.
Now we'll need to be able to generate inherited signatures after some amount of type checking.
- 3. Support delegation flavor with inferred callee.
Need larger amount of type checking before being able to generate inherited signatures.
- 4. Support `RetMatch.SameUpToSelfType` cases.
- 5. Support `ArgsMatch.SameUpToSelfType` cases.
- 6. Consider supporting secondary layers of sugar like "glob delegation".

Support for a list of multiple delegated functions in a single delegation item can be done in
parallel with any step after `1`.

# Literature

- https://en.wikipedia.org/wiki/Delegation_(object-oriented_programming)
- https://kotlinlang.org/docs/delegation.html
- https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/newtype_deriving.html
- https://github.com/rust-lang/rust/issues/7773 (delegation of entire trait implementations)
- https://github.com/rust-lang/rust/issues/8353 (delegation of entire trait implementations, automatically derive traits on newtypes)
- https://github.com/rust-lang/rust/issues/9912 (implementation inheritance via traits)
- https://github.com/rust-lang/rust/issues/19597 (syntax extension to derive traits for a newtype)
- https://github.com/rust-lang/rfcs/pull/186 (newtype keyword)
- https://github.com/rust-lang/rfcs/issues/261 (newtype deriving)
- https://github.com/rust-lang/rfcs/issues/292 (delegation of entire trait implementations)
- https://github.com/rust-lang/rfcs/issues/299 (implementation inheritance via traits)
- https://github.com/rust-lang/rfcs/issues/349 (efficient code reuse)
- https://github.com/rust-lang/rfcs/issues/479 (delegation of entire trait implementations, automatically derive traits on newtypes)
- https://github.com/rust-lang/rfcs/pull/508 (newtype deriving)
- https://github.com/rust-lang/rfcs/pull/949 (castable newtypes)
- https://github.com/rust-lang/rfcs/pull/1406 (delegation of implementation) - main RFC
- https://github.com/rust-lang/rfcs/pull/1546 (fields in traits)
- https://github.com/rust-lang/rfcs/pull/2242 (semantic newtypes)
- https://github.com/rust-lang/rfcs/pull/2375 (inherent trait implementation)
- https://github.com/rust-lang/rfcs/pull/2393 (delegation) - rework of main RFC
- https://github.com/rust-lang/rfcs/pull/2429 (reserve delegate keyword)
- https://github.com/rust-lang/rfcs/pull/2874 (trait enums)
- https://github.com/rust-lang/rfcs/issues/3108 (delegation dup)
- https://github.com/rust-lang/rfcs/issues/3133 (delegation another dup)
- https://internals.rust-lang.org/t/syntactic-sugar-for-delegation-of-implementation/2633 (preface to main rfc)
- https://internals.rust-lang.org/t/3-weeks-to-delegation-please-help/5742 (preface to main rfc rework)
- https://internals.rust-lang.org/t/new-rfc-for-delegation-anyone-interested-in-contributing/6644 (preface to main rfc rework 2)
- https://internals.rust-lang.org/t/potential-rfc-delegation/13831 (dup)
- https://internals.rust-lang.org/t/pre-rfc-forwarding/13836 (dup)
- https://internals.rust-lang.org/t/adding-true-oo-capabilities-to-rust/16691 (tangentially related)
- https://internals.rust-lang.org/t/is-it-still-possible-to-make-progress-on-postponed-features-the-case-of-delegation/16868 (status update on 2 main RFCs N years later)
- https://internals.rust-lang.org/t/strong-type-aliases-type-copy/17754 (dup)
- https://crates.io/crates/delegate (fn delegation, lots of features and syntax, post- and pre-processing, alive)
- https://crates.io/crates/delegate-attr (fn delegation, a bit simpler than delegate, dead)
- https://crates.io/crates/ambassador (traits only, trait prototype registering, not very alive)
- https://crates.io/crates/enum_delegate (trait <-> enum polymorphism specifically, trait prototype registering, alive)
