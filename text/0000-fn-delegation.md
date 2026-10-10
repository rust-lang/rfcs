
- Feature Name: (fill me in with a unique ident, `fn_delegation`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC proposes a design for _delegation_: syntactic sugar for ergonomically forwarding function calls.

## Motivation

### Forwarding to a subobject

Rust [deliberately](https://doc.rust-lang.org/book/ch18-01-what-is-oo.html#inheritance-as-a-type-system-and-as-code-sharing) does not provide the kind of data inheritance common in object-oriented languages where a derived type automatically inherits methods from a base type. Instead, Rust typically expresses this pattern through composition: the "base" type is embedded inside the "derived" type as a field (possibly nested) or another form of subobject. With composition, methods that would be inherited automatically in other languages must instead be implemented manually, often with the help of macros. Consider a common pattern [found](https://github.com/rust-lang/rust/blob/ad2e756c7093149e25f67a747e579a49b7e6976e/library/core/src/iter/adapters/flatten.rs#L55-L104) throughout real Rust codebases:

<!-- compile-fail: incomplete -->
```rust
impl<I: Iterator, U: IntoIterator, F> Iterator for FlatMap<I, U, F>
where
    F: FnMut(I::Item) -> U,
{
    fn next(&mut self) -> Option<U::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn advance_by(&mut self, n: usize) -> Result<(), NonZero<usize>> {
        self.inner.advance_by(n)
    }

    fn count(self) -> usize {
        self.inner.count()
    }

    fn last(self) -> Option<Self::Item> {
        self.inner.last()
    }
    ...
}
```

The `Iterator` implementation simply forwards multiple method calls to a field that already implements that trait. The pattern is particularly common with newtypes, which often need to reintroduce many of the inner type's inherent methods or trait implementations.

This situation highlights a gap in Rust’s ergonomics: while Rust provides powerful mechanisms for defining abstractions through traits and generics, it offers comparatively little support for reusing existing behavior.

This RFC aims to address this limitation by introducing a delegation feature. With delegation, the forwarding implementation above could be rewritten as follows:

<!-- compile-fail: not for all of the `Iterator` methods delegation is currently supported on nightly -->
```rust
impl<I: Iterator, U: IntoIterator, F> Iterator for FlatMap<I, U, F>
where
    F: FnMut(I::Item) -> U,
{
    type Item = I::Item;
    reuse Iterator::* { self.inner }
}
```

Delegation has long been discussed by the Rust community: it has motivated two prior RFCs ([rust-lang/rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406), [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393)), multiple conversations, and several macro crates. See [_Prior art_](#prior-art) for an overview of these efforts. This proposal seeks to revive that work.

### Generalization

While forwarding to subobject methods remains the main motivating scenario, a sufficiently general mechanism for forwarding function calls can support other scenarios as well.
- An inherent method on a type forwarding to a method from a trait implementation on the same type.
- A "reexport on steroids" that adds attributes to an existing function definition.
  - For example, target feature attributes, like it often happens in [stdarch](https://github.com/rust-lang/stdarch).
- Any other scenario that takes the general form of a function calling another function with limited argument transformation.

This part of the motivation is a lesson drawn directly from the two prior attempts at delegation. Both [rust-lang/rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) restricted delegation in some form, leaving multiple possible delegation patterns as future extensions. In both cases, the forward-compatibility concerns were never addressed. Therefore, in this proposal we want to explore the design space more thoroughly.

## How to read this RFC

This RFC is quite long, and a few kinds of cross-references recur throughout it, so it's worth spelling out the convention up front:

- A ([?](#anchor)) link points to a rationale subsection under “Rationale and alternatives” explaining why a design decision was made the way it was. These are asides: skipping them does not affect your understanding of the feature itself, only of the reasoning behind a specific choice.
- A [_text in italics_](#anchor) link points to another section of the RFC: material the current paragraph depends on.
- A plain [text](url) link points to a resource outside this RFC, such as a pull request, issue, comment, crate, or page in the Rust Reference.

### Terminology

The following terminology is frequently used in this proposal:

- _delegation item_ - a new item kind introduced by this proposal, declared with the `reuse` keyword, that generates a function or method that forwards its arguments to the specified callee.
- _target block_ - an optional block expression whose trailing expression transforms some of the generated function's arguments before those arguments are forwarded to the callee; usually, the transformed argument is the method receiver.
- _target expression_ - the trailing expression of the target block, if it exists.
- _parent context_ - the parent item in which the delegation item appears. This can be a module or block (for free functions), a trait implementation, an inherent implementation, or a trait definition (for associated functions).
- _desugaring_ - transformation of a delegation item into a regular function definition with a signature and a body.
- _renaming_ - the ability to give the generated function a name that differs from the callee's name.
- _delegation pattern_ - a piece of code that can potentially be rewritten using a delegation item.
- _delegation resolution_ - a function definition from which the signature is copied during desugaring.

## Implementation experience

This RFC draws on the experimental implementation tracked in [rust-lang/rust#118212](https://github.com/rust-lang/rust/issues/118212).

Many of the examples in this proposal can be tried on nightly Rust.

The nightly implementation is [feature-complete](https://en.wikipedia.org/wiki/Software_release_life_cycle#Feature-complete) and may even accept more code than this RFC describes, since its primary purpose was experimentation.
Different parts of the implementation may have different levels of design maturity and polish. If the feature is stabilized, stabilization will definitely take place in multiple stages.

Some delegation subfeatures, such as delegation to inherent methods, may work in a limited way, since supporting them properly would require compiler reengineering to avoid query cycles. Some of these limitations are discussed throughout the proposal.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Suppose you're writing a `BTreeSet<T>` type as a wrapper around `BTreeMap<T, ()>`, which is, incidentally, close to how the standard library's own `BTreeSet` is built (the real `BTreeSet` also carries an allocator parameter, elided here for simplicity).

```rust
pub struct BTreeSet<T> {
    map: BTreeMap<T, ()>,
}
```

Here is the forwarding implementation of the `Hash` trait for `BTreeSet`:

```rust
impl<T: Hash> Hash for BTreeSet<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.map.hash(state)
    }
}
```

With delegation, the same implementation could look like this:

```rust
impl<T: Hash> Hash for BTreeSet<T> {
    reuse Hash::hash { self.map }
}
```

The `reuse` item is a delegation item that desugars into a function definition. `Hash::hash` is the callee to which the delegation item forwards, and `{ self.map }` is the target block: a small block whose trailing expression is applied to some of the callee’s arguments, usually the receiver.

> [!NOTE]
>
> This example and some examples below can be expressed more efficiently using `derive` because they use traits from the standard library, but here we are using delegation for them to demonstrate what is possible.

### Paths and callee disambiguation

One might ask why we need to specify the path `Hash::hash` instead of simply writing `hash` in the previous example. The reason is that the callee does not have to be a method of the wrapped type, so the name alone would not tell which function is meant. For example, the `IntoIterator` implementation of `BTreeSet` simply calls the `iter` inherent method of `BTreeSet` itself:

```rust
impl<'a, T> IntoIterator for &'a BTreeSet<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}
```

With delegation, the `into_iter` implementation may be replaced as follows:

```rust
impl<'a, T> IntoIterator for &'a BTreeSet<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    reuse BTreeSet::<T>::iter as into_iter { self }
}
```

Here, the `as into_iter` part gives the generated function the name the trait requires (see [_Renaming a delegated method_](#renaming-a-delegated-method) below). The target block `{ self }` just passes the receiver through unchanged.

Paths unambiguously identify the function to which we are forwarding. When delegating to type-relative paths, as with `BTreeSet::<T>::iter` above, you currently need to specify the type's generic arguments. This limitation could be removed in the future.

### Other parent contexts

Once delegation to any kind of function is allowed, whether an inherent method, a trait method, or a free function, there is no need to restrict the parent context either. For example, `BTreeMap` implements `len` as an inherent method:

```rust
impl<T> BTreeSet<T> {
    pub fn len(&self) -> usize {
        self.map.len()
    }
}
```

and `reuse` can forward it just as easily:

```rust
impl<T> BTreeSet<T> {
    reuse BTreeMap::<T, ()>::len { self.map }
}
```

Note also that the parent context and the callee are independent of each other. As explained in [_Paths and callee disambiguation_](#paths-and-callee-disambiguation), you can delegate from any kind of function to any kind of function. For example, a free function can delegate to an inherent method.

### Delegating several methods at once

The main advantage of delegation comes with the ability to delegate multiple items at once. For example, listing `is_empty`, `clear`, and `len` as three separate `reuse` items would still be three lines whose only real difference is the method name. List delegation collapses them into one:

```rust
impl<T> BTreeSet<T> {
    reuse BTreeMap::<T, ()>::{len, is_empty, clear} { self.map }
}
```

Each generated method gets the receiver its callee needs: `clear` needs to mutate the map, so the method generated by `reuse` takes `&mut self`, while `len` and `is_empty` only need to read it, so those take the shared reference `&self`. The target block `{ self.map }` is the same in each case. You do not have to write the references by hand; autoref and autoderef apply according to their usual rules.

### Renaming a delegated method

Sometimes the callee's name isn't the name you want on your own type. `contains_key` reads naturally on a map, but for a set, `contains` is clearer. Adding `as new_name` after the callee renames the generated method:

```rust
impl<T> BTreeSet<T> {
    reuse BTreeMap::<T, ()>::contains_key as contains { self.map }
}
```

You can see that the syntax of `reuse` items is generally modeled after `use` items.

### Omitting the block expression

In the [_Paths and callee disambiguation_](#paths-and-callee-disambiguation) we used the target block `{ self }`. In such cases, it carries no information and can be omitted entirely, with the item ending in a semicolon instead:

```rust
impl<'a, T> IntoIterator for &'a BTreeSet<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    reuse BTreeSet::<T>::iter as into_iter;
}
```

This is purely syntactic sugar: `reuse path;` stands for `reuse path { self }`.
Although in some cases, when the delegated function has no `self` argument, an explicit target block is not allowed, but the semicolon form will work.

### Methods without a receiver

So far, the target block has been applied only to the callee’s receiver, while the remaining arguments (such as the `state` argument of `Hash::hash`) have been passed through unchanged. But not every forwarded method has a receiver. `Default::default` has no arguments at all: the `Default` implementation of `BTreeSet` just calls the inherent function `new`:

```rust
impl<T> Default for BTreeSet<T> {
    fn default() -> BTreeSet<T> {
        BTreeSet::new()
    }
}
```

With delegation, the same implementation could look like this:

```rust
impl<T> Default for BTreeSet<T> {
    reuse BTreeSet::<T>::new as default;
}
```

The target block must be omitted here because the delegated function has no parameters.

### Delegating binary operators

Binary operators are also different in that both operands have the same type as the receiver. Here is the forwarding implementation of the `PartialEq` trait for `BTreeSet`:

```rust
impl<T: PartialEq> PartialEq for BTreeSet<T> {
    fn eq(&self, other: &BTreeSet<T>) -> bool {
        self.map.eq(&other.map)
    }
}
```

With delegation, the same implementation could look like this:

<!-- compile-fail: nightly doesn't support `Self` identification in generic parameter defaults yet -->
```rust
impl<T: PartialEq> PartialEq for BTreeSet<T> {
    reuse PartialEq::eq { self.map }
}
```

`BTreeMap::eq` compares two maps, so the target block must be applied not only to `self`, but also to `other`.

### Delegating methods that return the wrapper

The conversion also works the other way around. Here is the forwarding implementation of the `Clone` trait for `BTreeSet`:

```rust
impl<T: Clone> Clone for BTreeSet<T> {
    fn clone(&self) -> Self {
        BTreeSet { map: self.map.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.map.clone_from(&source.map);
    }
}
```

Both methods can be delegated using the list syntax shown above:

<!-- compile-fail: nightly currently hardcodes the field name to `0`, but BTreeSet has a named field called `map` -->
```rust
impl<T: Clone> Clone for BTreeSet<T> {
    reuse Clone::{clone, clone_from} { self.map }
}
```

Both methods share the target block `{ self.map }`. The value returned by `Clone::clone` is wrapped, the target block is applied to the `source` argument of `Clone::clone_from`, and neither transformation has to be spelled out.

### Delegating a whole trait

`Clone` has only these two methods, so instead of listing them we can delegate all of them with a glob:

<!-- compile-fail: nightly currently hardcodes the field name to `0`, but BTreeSet has a named field called `map` -->
```rust
impl<T: Clone> Clone for BTreeSet<T> {
    reuse Clone::* { self.map }
}
```

A glob delegation item behaves as if all the methods of the trait were listed, including those with default implementations. `clone_from` is such a method. A glob delegation can also be written as follows:

<!-- compile-fail: nightly currently hardcodes the field name to `0`, but BTreeSet has a named field called `map` -->
```rust
reuse impl<T: Clone> Clone for BTreeSet<T> { self.map }
```

This form is purely syntactic sugar for the previous form.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Syntax

This proposal introduces two new [item kinds](https://doc.rust-lang.org/reference/items.html), function delegation items and impl delegation items:

```diff
Item →
    OuterAttribute* ( VisItem | MacroItem )

    VisItem →
    Visibility
    (
        Module
      | ExternCrate
      ...
+     | FnDelegation
+     | ImplDelegation
```

From here on, function delegation items are referred to simply as "delegation items".

Delegation items are accepted syntactically and semantically in all contexts where functions with bodies are accepted semantically.
That means modules and blocks, traits, and implementations, but not `extern` blocks. In `extern` blocks, delegation items are rejected syntactically ([?](#why-can-delegation-items-be-declared-in-any-position)). Like other items, delegation items may be annotated with a visibility modifier ([?](#why-is-visibility-manually-added-instead-of-being-copied-from-the-callee)) and may have attributes applied to them ([?](#why-are-attributes-manually-added-instead-of-being-copied-from-the-callee)).

Delegation items have the form:
```diff
+ FnDelegation →
+     reuse DelegationPaths ( BlockExpressionNoInnerAttributes | ; )
+
+ DelegationPaths →
+     PathExpression ( as IDENTIFIER )?
+     ( PathExpression | QualifiedPathType ) :: { ( PathIdentSegment ( as IDENTIFIER )? )* , ? }
+     ( PathExpression | QualifiedPathType ) :: *
```

The grammar is generally modeled after `use` items, with two major differences: qualified paths and generic arguments in paths are supported, and nested lists and globs are not supported.

A delegation item starts with the `reuse` keyword ([?](#why-reuse)) and consists of a path prefix, which may be either simple or qualified, a suffix, and an optional block expression. Their roles are discussed in the following sections.

Suffixes come in three flavors: individual delegation, list delegation ([?](#why-is-list-delegation-supported)), and glob delegation ([?](#why-is-glob-delegation-supported)). The optional `as IDENTIFIER` clause allows the delegated function to be defined with a different name ([?](#why-is-renaming-supported)).

A delegation item intentionally does not provide syntax for introducing its own generics ([?](#why-doesnt-a-delegation-item-provide-syntax-for-introducing-its-own-generics)). A delegation item intentionally does not provide syntax for argument or return-value transformations, besides the target block ([?](#why-doesnt-a-delegation-item-provide-syntax-for-argument-or-return-value-transformations)).

> [!NOTE]
>
> Expression path syntax (with mandatory turbofish) is used for consistency with other value paths, but it is not technically necessary. Type path syntax (with optional turbofish) could be supported later if necessary.

Impl delegation items have the form:
```diff
+ ImplDelegation ->
+    `reuse` `unsafe`? `impl` GenericParams? `!`? TypePath `for` Type
+    WhereClause?
+     ( BlockExpression | ; )
```

This is the same syntax as for regular `impl` items, except that the block containing associated items is replaced with a block expression.
Impl delegation items are accepted in all contexts where regular impl are accepted.

### List, glob, and impl delegation

List, glob, and impl delegations are three kinds of higher-level syntactic sugar that expand to individual function delegations at macro expansion time.

#### List delegation

List delegation defines several items at once from a shared path prefix. It desugars to one individual delegation item per name.

Target blocks, generic arguments, and other components are copied at the token-stream level, making list delegation a macro feature.

```rust
reuse prefix::<Args>::{a, b, c} { target }
```
expands to
```rust
reuse prefix::<Args>::a { target }
reuse prefix::<Args>::b { target }
reuse prefix::<Args>::c { target }
```

If a target block or a generic argument contains something with an identity, such as an item or a closure, it is also copied as tokens. As a result, multiple distinct, independent items or closures are created ([extended rationale](https://github.com/rust-lang/rfcs/pull/3530#issuecomment-2020869823)).

<!-- compile-fail: some unresolved names -->
```rust
reuse prefix::{a, b} {
    use some::import; // import
    self.field.map(|x| x.y) // closure
}

// Desugars to
reuse prefix::a {
    use some::import; // import 1
    self.field.map(|x| x.y) // closure 1
}
reuse prefix::b {
    use some::import; // import 2
    self.field.map(|x| x.y) // closure 2
}
```

Empty list delegations are currently prohibited (see [_Future possibilities: Empty list delegation_](#empty-list-delegation)).

#### Glob delegation

Glob delegation allows all methods of a trait to be delegated at once. It desugars to one individual delegation item per "glob-imported" name.

Glob delegations are semantically allowed only inside implementations, and the path prefixes in glob delegations can only refer to traits ([?](#why-are-glob-delegations-restricted)). Note that there are no similar restrictions on list delegations.

The set of names for which individual delegation items are produced is determined as follows:
- The full set of names defined by the target trait in all namespaces is considered.
- Names already explicitly defined inside the glob delegation's parent context (the impl) are filtered out ([?](#why-can-names-from-glob-delegations-be-overridden-by-explicit-items)). "Explicitly" here means not by another glob delegation.
- If any of the remaining names refers to an associated type or constant, an error is reported for future compatibility with associated type and const delegation (see [_Future possibilities: Support for delegating types and consts_](#support-for-delegating-types-and-consts)).
- Note: the above rules mean that a glob delegation can only be expanded after all macro invocations in its target trait and its parent impl have been expanded, except perhaps other glob delegations.

Note that individual delegations are still generated for functions with default bodies in the target trait definition.
This ensures that manual implementations of functions with default bodies are correctly forwarded to ([?](#why-are-methods-with-default-bodies-included-in-glob-delegation)).

As with list delegations, target blocks, generic arguments, and other components are copied at the token-stream level, making glob delegation a macro feature.

Empty glob delegations are currently prohibited (see [_Future possibilities: Empty list delegation_](#empty-list-delegation)).

<details>

<summary> Example: desugaring of glob delegation.</summary>

<!-- compile-fail: things are omitted for brevity -->
```rust
trait Trait<Args> {
    fn a() {} // has default body
    fn b();
    fn c();
}
impl Trait<Args> for Type {
    fn c() {} // explicitly defined name
    reuse Trait::<Args>::* { target }
}
```
expands to
```rust
impl Trait<Args> for Type {
    fn c() {} // explicitly defined name, not delegated
    reuse prefix::<Args>::a { target } // delegated, despite the default body
    reuse prefix::<Args>::b { target }
}
```

</details>

#### Impl delegation

Impl delegation is a second layer of syntactic sugar that makes it convenient to write a trait impl containing a glob delegation to the same trait.

<!-- compile-fail: some unresolved names -->
```rust
reuse impl Trait<Args> for Type { target }
```
expands to
```rust
impl Trait<Args> for Type {
    reuse Trait::<Args>::* { target };
}
```

All restrictions on regular glob delegations also apply to glob delegations produced by impl delegations.

### Paths and name resolution

Delegation reuses the existing path mechanism and does not introduce a new approach to name resolution.
Paths allow delegation items to unambiguously identify callable items they forward to, including trait methods, trait implementation methods (with qualified paths used to specify `Self`), inherent methods, and free functions ([?](#why-are-qualified-paths-used-for-call-disambiguation)). Also see [_Future possibilities: Name-based resolution as sugar_](#name-based-resolution-as-sugar).

Delegation items can also refer to other delegation items. If a cycle is encountered in such a chain of recursive delegations, an error is reported.

Delegation paths are resolved in the value namespace, and if the path doesn't refer to a function or associated function, an error is reported.
In particular, delegation for associated types and constants is not currently supported (see [_Future possibilities: Support for delegating types and consts_](#support-for-delegating-types-and-consts)).

Type-relative paths are also supported, although support for them on nightly rustc is currently limited.

> [!NOTE]
>
> Delegation to inherent methods is particularly complex to implement. From the perspective of name resolution, paths in Rust may be classified as follows:
> - A path to a free function (e.g., `module::func`).
> - A reference to an associated item defined in a trait (e.g., `<Vec<T> as Clone>::clone`), where the `Self` type may also be omitted.
> - A type-relative path (e.g., `<T>::default`).
>
> Lowering a delegation item into a real function requires knowing the callee's signature, including its generics, the number of arguments, and whether and how it takes a `self` argument. With this information, a signature can be synthesized for the new item. Paths in the first two categories can be resolved early enough to expose that information. Type-relative paths generally cannot: their resolution is not known until type checking, by which point the delegation item's signature is already needed.
>
> Currently, to work around this limitation, we use simple name-based resolution during [AST lowering](https://rustc-dev-guide.rust-lang.org/hir/lowering.html): we only resolve to an inherent method with the specified name. This does not work in more complex cases, such as when a type-relative path resolves to a trait method:
>
>  ```rust
> pub struct Struct {}
>
> trait Trait {
>     fn to_string(x: &Struct) -> String {
>         // default impl
>     }
> }
>
> impl Trait for Struct {}
>
> impl Struct {
>     pub fn get_string(x: &Struct) -> String {
>         Struct::to_string(f)
>     }
> }
> ```
>
> `Struct::to_string` resolves to `Trait::to_string`. However, since we cannot perform trait selection during lowering, we report an error. This limitation could potentially be addressed in the future, the current workaround is to use `Trait::to_string`. See [_Future possibilities: Supporting type-relative paths_](#supporting-type-relative-paths).

### Desugaring of individual delegation: signature

All other delegation forms desugar into individual delegations as specified above. Here, we describe how individual delegations desugar further into regular functions.

Individual delegation defines exactly one new function item that forwards to exactly one callee identified by a path.

The function from which the delegated item's signature is copied is called the _delegation resolution_.
For delegations defined in a trait implementation, the delegation resolution is the corresponding trait method ([?](#why-is-the-delegation-resolution-the-corresponding-trait-method-in-trait-implementations)). In all other cases, it is the item resolved by the path (see [_Paths and name resolution_](#paths-and-name-resolution) for details on how the path is resolved).

The generated function header for an individual delegation has the following form:

<!-- compile-fail: some unresolved names -->
```rust
#[attrs]
pub(vis) reuse path as name { target_expr }
```
desugars to
```rust
#[attrs]
pub(vis) FunctionQualifiers fn name<GenericParams>(..., argN: ArgN, ...) -> FunctionReturnType
WhereClause
{
    /* body */
}
```

- Outer attributes (`#[attrs]`) are exactly those specified by the user at the delegation site, if any, plus [_default attributes_](#default-attributes).
- Visibility (`pub(vis)`) is exactly as specified by the user at the delegation site. Also see [_Unresolved questions: Should the visibility of the delegation item be restricted?_](#should-the-visibility-of-the-delegation-item-be-restricted)
- Function qualifiers (`FunctionQualifiers`: safety, constness, asyncness, and ABI) are copied unchanged from the delegation resolution.
  - None of these qualifiers can be overridden ([?](#why-are-function-qualifiers-copied-unchanged)).
- If the delegation item has an `as name` clause, then `name` is used as the generated function's name; otherwise, the final segment of `path` is used.
- Generic parameters (`GenericParams`) and where clauses (`WhereClause`) are copied from the delegation resolution and remapped as described in [_Generics remapping_](#generics-remapping).
- Function parameters (e.g., `argN: ArgN`) are copied from the delegation resolution:
  - Generic parameters appearing in the function arguments are remapped as described in [_Generics remapping_](#generics-remapping).
  - The `self` parameter turns into a regular first parameter if the parent context is not an impl or trait.
  - Delegating to C-variadic functions is not supported ([?](#why-is-delegation-of-variadic-functions-not-supported)).
- The return type (`FunctionReturnType`) is copied from the delegation resolution:
  - Generic parameters appearing in the return type are remapped as described in [_Generics remapping_](#generics-remapping).

> [!NOTE]
>
> Desugaring happens mainly during [AST lowering](https://rustc-dev-guide.rust-lang.org/hir/lowering.html). This is because once [HIR](https://rustc-dev-guide.rust-lang.org/hir.html) construction is complete, the crate becomes immutable and code modification is no longer possible at that stage. The types and generics are then processed further during HIR -> Ty lowering.

#### Default attributes

An `#[inline]` attribute is implicitly added to the generated function unless the delegation item already has an `inline` attribute.

A `#[must_use]` attribute is implicitly copied to the generated function if the delegation resolution has one, unless the delegation item already has a `must_use` attribute (perhaps with a different message).
It is not currently possible to make the generated function non-`must_use` if the delegation resolution is `must_use`.

Also see [_Unresolved questions: Which attributes should be added by default?_](#which-attributes-should-be-added-by-default)

#### Path elaboration

Generic arguments in the delegation path are "elaborated" in the same way as in other paths in expression contexts.
This means that generic arguments not specified explicitly are filled in with their defaults, filled in with `'static` for elided lifetimes, or replaced with inference placeholders (`_` for types or `'_` for lifetimes), depending on the context.

The generics remapping process below uses generic arguments from delegation paths in their elaborated form.

#### Generics remapping

As mentioned earlier, a delegation item cannot explicitly introduce its generic parameters. Instead, they are copied from the delegation resolution.

However, after copying, we need to remap the generics so that the copied signature and where clauses remain semantically equivalent to what would be written by hand (e.g., the callee's parent parameters don't automatically make sense once copied into a different scope).

The delegation resolution's signature may contain:
- Own generic parameters - generic parameters defined directly by the delegation resolution item.
- Parent generic parameters, including `Self` - generic parameters defined by the delegation resolution's parent trait or impl.
  - After copying, all parent parameters start as *unsubstituted*. Replacing the parameter's uses with the provided arguments makes it substituted.

The following procedure is used for remapping each own parameter:
- The generic argument corresponding to the parameter is identified in the last segment of the elaborated callee path.
- If the generic argument is an inference placeholder (`_` or `'_`), then both the generic parameter definition and its uses stay in place ([?](#why-are-inference-placeholders-allowed-in-paths)).
  - Nested inference placeholders are not allowed ([?](#why-are-nested-inference-placeholders-not-allowed-in-paths)).
- If the generic argument is not an inference placeholder, then the generic parameter's definition is eliminated from the generated function and all its uses are replaced with that argument ([?](#why-might-own-generic-parameters-need-to-be-substituted)).

The following procedure is used for remapping the `Self` parent parameter:
- If the parent context is an impl or a trait, then all the parameter's uses are replaced with the impl's or trait's self type.
  - The `Self` argument in the callee path is ignored, even if it is specified ([?](#why-is-the-self-type-not-substituted)).
- Otherwise, the generic argument corresponding to the `Self` parameter is identified in the elaborated qualified callee path.
- If the generic argument is an inference placeholder, then uses of `Self` stay in place and remain unsubstituted.
  - Nested inference placeholders are not allowed.
- If the generic argument is not an inference placeholder, then all uses of the parameter are replaced with that argument.

The following procedure is used for remapping each non-`Self` parent parameter ([?](#why-might-parent-parameters-need-to-be-substituted)):
- If the parent context is a trait impl (`impl Trait<Args> for ...`), then all the parameter's uses are replaced with the corresponding argument in `Args`.
  - Any matching arguments in the callee path are ignored.
    - This is because the generated signature must match the corresponding trait method, while the delegation path may refer to a different item whose generic parameters do not necessarily correspond to those of the trait method.
- Otherwise, if the callee's parent is a trait, the generic argument corresponding to the parameter is identified in the trait segment of the elaborated callee path.
- Otherwise, if the callee's parent is an inherent impl, the corresponding parameter value is determined from results of the type-relative path resolution.
    - When a type-relative path `Type<TypeArgs>::assoc` is resolved, it is tied to a specific impl block `impl<ImplParams> Type<ImplTypeArgs>` (the callee's parent) with a specific list of substitutions for `ImplParams`, even if that the `ImplParams` list does not *directly* correspond to the `TypeArgs` list. If the substitution is the parameter definition itself, then we treat is as an inference placeholder below.
- If the generic argument is an inference placeholder, then uses of the parameter stay in place and remain unsubstituted.
  - Nested inference placeholders are not allowed.
- If the generic argument is not an inference placeholder, then all uses of the parameter are replaced with that argument.

If any parent parameters in the signature or where clauses remain unsubstituted, an error is reported ([?](#what-happens-if-unsubstituted-parent-parameters-remain-after-substitution)).

<details>

<summary> Example: generics remapping with step-by-step desugaring.  </summary>

Consider this example, in which an inherent impl delegates to a trait:
```rust
trait Trait<T> {
    fn method<U>(&self, arg1: &T, arg2: &U);
}

impl Wrapper {
    reuse Trait::method { self.inner }
}

```

Step 0, immediately after desugaring but before remapping, looks like this:
Copied but not yet substituted parameters (`T`) are written as `?T`, the same notation is used in the rationale sections as well.

```rust
impl Wrapper {
    fn method<?U>(self: &?Self, arg1: &?T, arg2: &?U) {
        Trait::<_>::method::<_>(&self.inner, arg1, arg2)
    }
}
```

`?U` is one of the function's own parameters. It is copied together with its scope (`method<?U>`) as part of the function itself, so it can always be substituted with its new definition if necessary.
`?T` and `?Self` were defined in the parent scope, so immediately after copying, they are unsubstituted.

After remapping, this becomes:
```rust
impl Wrapper {
    fn method<U>(self: &Wrapper, arg1: &?T, arg2: &U) {
        Trait::<_>::method::<_>(&self.inner, arg1, arg2)
    }
}
```

Thus, `?T` remains unsubstituted, and this will produce an error.
However, if we wrote `Trait::<u8>::method` instead of just `Trait::method`, then `?T` would be substituted with `u8` and the code would be legal.

</details>

Also see [_Future possibilities: More sophisticated inference of generic parameters_](#more-sophisticated-inference-of-generic-parameters)

#### Effective self type identification

To increase the usefulness of delegation and provide better support for newtypes, we need to identify types that are "actually `Self`" in method signatures, not just for the `self` parameter but also for other parameters and the return type ([?](#why-are-effective-self-types-identified-beyond-the-receiver)).

For example, the occurrences of `Struct` in `other: Struct` and `-> Struct` in the following impl are "actually `Self`".

```rust
trait Trait {
    fn method(self, other: Self) -> Self;
}

impl Trait for Struct {
    fn method(self, other: Struct) -> Struct { unimplemented!() }
}
```
Let's call such types in signatures "effective self types".

The following procedure is used for detecting effective self types.

If the delegation resolution is a trait method, then the `Self` parameter occurrences in that trait method (before substitution) are considered effective self types in the generated function.
In situations like
```rust
trait BinOp<Rhs = Self> {
    fn bin_op(&self, rhs: &Rhs);
}
```
we also need to treat `Rhs` as an effective self type if it was obtained from the `Self` parameter default; otherwise, newtype conversions won't work correctly when delegating standard binary operators.

If the delegation resolution is an inherent method with `self`, then its corresponding type is considered an effective self type in the generated function.

In all other cases, types in signatures are not considered effective self types. In particular, effective self types are *not* detected by tracking uses of the `Self` type alias in impls (as opposed to the `Self` parameter in traits) or by checking type equality ([?](#why-are-effective-self-types-not-detected-through-type-aliases-or-type-equality)).

```rust
impl Struct {
    fn method(self, other1: Self, other2: Struct) {}
}

// Types of `other1` and `other2` are not considered effective self types here.
reuse Struct::method;
```

If a function parameter's type is an effective self type, possibly wrapped in one of the references or smart pointers mentioned in [items.associated.fn.method.self-ty](https://doc.rust-lang.org/reference/items/associated-items.html#r-items.associated.fn.method.self-ty), then let's call it an "effective self parameter" ([?](#why-are-specific-smart-pointers-used-for-detecting-effective-self-parameters)).
If the function's return type is an effective self type, without any additional wrapping, let's call it a "self return type" ([?](#why-are-self-return-types-limited-to-bare-self-type)).

In the generated function body, effective self parameters are converted using the delegation's target block, and self return types are converted using newtype wrapping.
See the [_body desugaring chapter_](#desugaring-of-individual-delegation-body) for details.

### Desugaring of individual delegation: body

#### Target block

The target block is an optional [block expression](https://doc.rust-lang.org/beta/reference/expressions/block-expr.html) ([?](#why-is-the-target-block-a-block-expression)) that is used to transform the delegation item's effective self parameters before they are forwarded to the resolved callee. There are no restrictions on the expressions that can be used inside the target block ([?](#why-is-the-target-block-unrestricted)).

Inside that block, `self` refers to an effective self parameter that will be transformed.
If the generated function has no effective self parameters, then it's an error to specify the target block, unless the current delegation item was defined as part of a list or glob delegation in which at least one other item has an effective self parameter ([?](#why-can-target-blocks-be-allowed-without-effective-self-parameters)).

If the target block is omitted, then no parameter transformations happen, and delegated functions without effective self parameters also work.

The target block contains zero or more statements and an optional trailing expression, called the target expression.
If the trailing expression is omitted, then `self` is implicitly used as the trailing expression ([?](#why-is-self-the-implicit-target-expression)).

#### Body desugaring

Suppose that the generated function has `N` effective self parameters. Then the target block is "disassembled" and its statements and trailing expression are inserted into the generated body `N` times, as shown in the following example ([?](#why-are-statements-not-passed-to-the-call)).

<!-- compile-fail: incomplete code -->
```rust
// target block
{
    target_stmt_1(self);
    ...
    target_stmt_M(self);
    target_expr(self)
}

// generated body, param1..paramN are the effective self parameters
(..., param1: Type1, ..., paramN: TypeN) {
    target_stmt_1(param1);
    ...
    target_expr_M(param1);
    ...
    target_stmt_1(paramN);
    ....
    target_expr_M(paramN);
    callee_path(..., ADJ(target_expr(param1)), ..., ADJ(target_expr(paramN)), ...)
}
```

Under these rules, statements with side effects (e.g., `dbg!(&self);`) execute once per effective self parameter.

<details>

<summary> Example: debug logging any effective self parameter </summary>

```rust
trait BinOpLike {
    fn bin_op(&self, rhs: &Self);
}

reuse impl BinOpLike for Wrapper {
    dbg!(self);
    self.inner
}

// Desugaring
impl BinOpLike for Wrapper {
    fn bin_op(&self, rhs: &Wrapper) {
        dbg!(self);
        dbg!(rhs);
        BinOpLike::bin_op(self.inner, rhs.inner)
    }
}
```

</details>

`ADJ` denotes the same set of adjustments as for an ordinary [method call](https://doc.rust-lang.org/reference/expressions/method-call-expr.html) receiver: a sequence of autoderefs, an optional autoref, and coercions ([?](#why-are-method-call-adjustments-applied-to-effective-self-parameters)). The difference is that the callee has already been resolved through the path, so these adjustments are not needed for method resolution. Instead, they are applied to the arguments to make them match the callee's signature.

<details>

<summary> Example: Automatic adjustments, glob and list delegations</summary>

```rust
trait Trait {
    fn static_f();
    fn by_value(self);
    fn by_ref(&self);
    fn by_mut_ref(&mut self);
}

struct X<T>(T);
impl<T: Trait> X<T> {
    // List delegation.
    // Target expression is automatically adjusted for each function
    // and removed for function without receiver.
    reuse <T as Trait>::{static_f, by_value as new_name} { self.0 }
}

struct X2<T>(T);
impl<T: Trait> X2<T> {
    // Glob delegation.
    // Target expression is automatically adjusted for each function
    // and removed for function without receiver.
    reuse <T as Trait>::* { self.0 }
}
```

</details>

All remaining parameters, which are not effective self parameters, are forwarded unmodified. Method-call adjustments are not applied to them, although implicit coercions may still apply.

The path `callee_path` is exactly as specified by the user, except that the generated function's own generic parameters are substituted as arguments to the final segment, rather than left to inference ([?](#why-are-the-delegation-resolutions-own-generic-parameters-substituted-as-arguments-to-the-final-segment)).

In a trait implementation, the delegation resolution and the actual callee may be different functions.
If any of the transformed or untransformed parameters are incompatible with the callee's signature, a type-checking error will be reported.

#### Return type wrapping

If the generated function's return type is a self return type, then the `callee_path` expression is additionally wrapped in a struct literal to perform "newtype wrapping".

```rust
Self { _: callee_path(...) }
```

`_` in this case denotes the single field of `Self` ([?](#why-is-return-type-wrapping-restricted-to-single-field-structs)).
If `Self` is not a struct (or union) with a single field, then an error will be reported.
An error will also be reported if that field is inaccessible from the delegation's definition site (e.g., because of its visibility).

<details>

<summary> Example: Self type identification and wrapping of the return value</summary>

```rust
trait MyAdd {
    fn add(self, other: Self) -> Self;
}

impl MyAdd for usize {
    fn add(self, other: usize) -> usize { self + other }
}

struct W(usize);
reuse impl MyAdd for W { self.0 }

// Desugaring:
fn add(self: W, arg1: W) -> W {
    // We detect that arguments are of type `Self` so we
    // apply target expression and adjustments to all of them,
    // next we wrap return value into a newtype.
    W { 0: MyAdd::add(self.0, self.0) }
}
```

</details>

## Drawbacks
[drawbacks]: #drawbacks

Many cases of delegation require more than simple forwarding (e.g., transforming arguments or return values). This feature only handles the simple cases, leaving complex transformations to manual coding or macros. This might limit the feature's usefulness, but the rationale for this is given in the [_syntax budget section_](#rule-1-stay-within-the-syntax-budget).

The delegation feature could potentially be implemented as a third-party library with compile‑time [_reflection_](#reflection) (if and when that becomes available).

TODO: collect remaining drawbacks from the rationale sections.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Design guiding principles

A recurring question throughout this RFC is whether a particular delegation pattern should be supported. The following guiding principles inform individual design decisions.

### Rule 1: stay within the syntax budget

Delegation items should use syntax that is no more complex than that of `use` imports.

<!-- compile-fail: some unresolved names -->
```rust
// Import item
#[attrs]
pub(vis) use prefix::{a, b, c as d};

// Delegation item
#[attrs]
pub(vis) reuse prefix::{a, b, c as d} { target_expr }
```

The motivation here is to avoid more complex features such as argument or return-value transformations, which would require preprocessing or postprocessing closures. With additional bells and whistles like that, a delegation item becomes as verbose as a full forwarding function implementation, but less readable. These transformations can instead be written manually or expressed using a macro (see [_Prior art_](#prior-art)).

### Rule 2: prefer generality over special casing

If a pattern fits within the proposal's syntax budget and can be expressed by a single, uniform desugaring rule, support it, even when it is expected to be rare in practice, rather than limit the support to what appears to be the common case.

This approach allows us to explore the design space more thoroughly, as discussed in the [_Motivation_](#generalization) section.

This is a default, not an absolute rule; exceptions may be made when there is a sufficiently strong reason to do so.

### Design decisions outlined in this RFC

#### Why can delegation items be declared in any position?

Delegation fundamentally forwards function calls. A regular function in Rust may be a trait method, a method in a trait implementation, an inherent method, or a free function. We can form different combinations based on the position of a caller and a callee:

<details>

<summary> Example: delegating from a trait implementation to an implementation of the same trait.</summary>

[example link](https://github.com/rust-lang/rust/blob/752b9bf8798c2ffc1d3fe2b804c04454366fc6d6/library/alloc/src/string.rs#L3635-L3641)

```rust
pub struct Drain<'a> {
    iter: Chars<'a>,
}

impl Iterator for Drain<'_> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        self.iter.next()
    }
}
```

</details>

<details>
<summary> Example: delegating from a trait implementation to an implementation of another trait. </summary>

[example link](https://github.com/rust-lang/rust/blob/752b9bf8798c2ffc1d3fe2b804c04454366fc6d6/library/core/src/iter/adapters/zip.rs#L74-L84)

```rust
trait ZipImpl<A, B> {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}

impl<A, B> Iterator for Zip<A, B>
where
    A: Iterator,
    B: Iterator,
{
    type Item = (A::Item, B::Item);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        ZipImpl::next(self)
    }
}
```

</details>

<details>
<summary> Example: delegating from an inherent method to a trait implementation. </summary>

[example link](https://github.com/rust-lang/rust/blob/752b9bf8798c2ffc1d3fe2b804c04454366fc6d6/library/std/src/collections/hash/set.rs#L149-L151)

```rust
impl<T> HashSet<T, RandomState> {
    pub fn new() -> HashSet<T, RandomState> {
        Default::default()
    }
}
```

</details>

<details>
<summary> Example: delegating from a free function to an inherent method. </summary>

[example link](https://github.com/rust-lang/rust/blob/752b9bf8798c2ffc1d3fe2b804c04454366fc6d6/compiler/rustc_ast_pretty/src/pprust/mod.rs#L99-L101)

```rust
pub fn to_string(f: impl FnOnce(&mut State<'_>)) -> String {
  State::to_string(f)
}
```

</details>

etc.

All these combinations appear in real-world code through regular calls, and each represents a potential use case for the delegation feature. Choosing which combinations to support is a design decision driven by multiple factors: the function call resolution algorithm, the available syntax budget, the frequency of the use case, and the extensibility to other cases.

Generality is particularly relevant in light of the existing prior art. The two previous delegation RFCs, [rust-lang/rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393), deliberately limited delegation to trait methods. Other proposals like [rust-lang/rfcs2375](https://github.com/rust-lang/rfcs/pull/2375) and [rust-lang/rfcs#3591](https://github.com/rust-lang/rfcs/pull/3591) address other use cases through different language mechanisms.


As established in the name resolution section, the callee may resolve to any of these kinds of functions. We see no reason to restrict the caller either (see [_guiding principles_](#design-guiding-principles)). Accordingly, this proposal supports every combination, rather than special-casing only the most common ones.

↩ [_Syntax_](#syntax)

#### Why is visibility manually added instead of being copied from the callee?

A delegation item is a distinct item whose behavior may deliberately differ from that of its callee. This also avoids ambiguity for users about whether omitting a visibility modifier makes the delegation item private or causes it to inherit the callee's visibility.

Also see [_Unresolved questions: Should the visibility of the delegation item be restricted?_](#should-the-visibility-of-the-delegation-item-be-restricted)

↩ [_Syntax_](#syntax)

#### Why are attributes manually added instead of being copied from the callee?

Attributes may affect diagnostics, linking, documentation, or the item's public API contract. A delegation item is a distinct item whose behavior may deliberately differ from that of its callee. Automatically inheriting attributes would also mean a delegation item's behavior could change silently whenever the callee's attributes change, with no corresponding edit at the delegation site.

Also see [_Unresolved questions: Which attributes should be added by default?_](#which-attributes-should-be-added-by-default)

↩ [_Syntax_](#syntax)

#### Why `reuse`?

The delegation syntax is generally modeled after `use` items to make it familiar and keep it within the syntax budget.
The keyword is similar to `use` for the same reason: the callee function is not used directly, as with imports, but reused to create a new function.

Alternative options like `delegate` or `forward` could also be considered, but they would benefit less from users' familiarity with `use` items.

↩ [_Syntax_](#syntax)

#### Why is list delegation supported?

The syntax cost of supporting it is negligible compared with the benefit. Specifically:

1. Individual delegation is very close to a regular function call in terms of the amount of code written and is not particularly useful on its own. One of the main benefits of delegation comes from being able to delegate multiple items at once, avoiding repetitive declarations.
2. It is not a new concept in Rust, as `use` declarations already support lists.
3. Some form of it appears in many prior attempts at delegation, demonstrating users' interest in this capability:
   1. `use expression for name_1, name_i` in [rust-lang/rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406)
   2. `delegate fn name_1, fn name_i to expression` in [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393)
   3. `export path . { sel_1, ..., sel_n }` in [Scala 3](https://docs.scala-lang.org/scala3/reference/other-new-features/export.html)


↩ [_Syntax_](#syntax)

#### Why is glob delegation supported?

The syntax cost of supporting it is negligible compared with the benefit. Specifically:

1. Individual delegation is very close to a regular function call in terms of the amount of code written and is not particularly useful on its own. One of the main benefits of delegation comes from being able to delegate multiple items at once, avoiding repetitive declarations.
2. It is not a new concept in Rust, as `use` declarations already support globs.
3. Some form of it appears in many prior attempts at delegation, demonstrating users' interest in this capability:
   1. `use expression` in [rust-lang/rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406)
   2. `delegate * to expression` in [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393)
   3. The `by` clause forwards an entire interface in one declaration in Kotlin.
   4. `#[delegate(Trait)]` delegates every method of `Trait` in [crates.io/ambassador](https://crates.io/crates/ambassador).
   5. `export name.*` in [Scala 3](https://docs.scala-lang.org/scala3/reference/other-new-features/export.html)

↩ [_Syntax_](#syntax)

#### Why is renaming supported?

The syntax cost of supporting it is negligible compared with the benefit. Specifically:

1. This will allow delegation from a trait implementation to a function that is not a method of the trait and has a different name from those defined in the trait.
    <details>

    <summary> Example: renaming in a trait implementation.</summary>

   ```rust
    impl<T> Default for BTreeSet<T> {
        reuse BTreeSet::<T>::new as default;
    }
   ```

   </details>
2. It is not a new concept in Rust, as `use` declarations already support renaming.
3. Some form of it appears in many prior attempts at delegation, demonstrating users' interest in this capability:
   1. `#[call(name)]` attribute in [crates.io/delegate](https://crates.io/crates/delegate)
   2. In [rust-lang/rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393), renaming is a possible extension.
   3. `export A as B` in [Scala 3](https://docs.scala-lang.org/scala3/reference/other-new-features/export.html)

↩ [_Syntax_](#syntax)

#### Why doesn't a delegation item provide syntax for introducing its own generics?

Consider the example:

```rust
pub fn to_vec<T: ConvertVec, A: Allocator>(s: &[T], alloc: A) -> Vec<T, A> {
    T::to_vec(s, alloc)
}
```

In principle, we could support this delegation pattern with syntax such as `reuse<T: ConvertVec, A: Allocator> T::to_vec;`. However, this would exceed our syntax budget (see [_guiding principles_](#design-guiding-principles)).

↩ [_Syntax_](#syntax)

#### Why doesn't a delegation item provide syntax for argument or return-value transformations?

There are several transformations one might reasonably want from the delegation feature:
- Return value: converting the callee's return value using `.into()`, unwrapping a `Result`/`Option` with `.unwrap()`, awaiting a future returned by the callee with `.await`, etc.
- Input arguments: reordering arguments or calling a method like `.as_ref()`, etc.

To support these transformations in their most general form, delegation items would need something closer to preprocessing and postprocessing closures. We do not support these in the RFC, in accordance with our [_guiding principles_](#design-guiding-principles).

↩ [_Syntax_](#syntax)

#### Why are glob delegations restricted?

Glob delegations are only allowed in implementations, but not in modules or blocks.
Modules can contain imports, both glob and single, and both make it harder to determine the set of names that would need to be "filtered out" from the glob delegation during its expansion.
Also, the motivation for glob delegations in modules is not as strong as the motivation for glob delegations in impls, which can be used to delegate whole trait impls, so the additional complexity doesn't pull its weight.

Glob delegations can only refer to traits, but not to modules.
Modules can contain imports, both glob and single, and both make it harder to determine the set of names that would be produced by the glob delegation during its expansion.
Modules typically contain other items rather than just functions, and delegating to them would result in errors with the current rules.
Similarly, the motivation for glob delegation from modules is not very strong, and the complexity also doesn't pull its weight.

List delegations list all their names explicitly, so they don't need any similar restrictions.

↩ [_Glob delegation_](#glob-delegation)

#### Why can names from glob delegations be overridden by explicit items?

TODO: explain the rationale.

↩ [_Glob delegation_](#glob-delegation)

#### Why are methods with default bodies included in glob delegation?

TODO important: explain the rationale. <br>
TODO drawback: discuss semver hazards from using glob delegations to trait items default bodies. <br>
TODO: standard derives tend to not generate impls for methods with default bodies (Clone, Hash, Ord, PartialEq, PartialOrd - no defaults, Eq - some defaults).

↩ [_Glob delegation_](#glob-delegation)

#### Why are qualified paths used for call disambiguation?

Rust distinguishes between two kinds of function invocation. The first kind consists of [method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html), which have the form `receiver.method(args...)`. They are resolved to associated methods that take a receiver argument. Resolution in that case requires additional analysis by the compiler: the receiver may be automatically dereferenced, borrowed, or coerced. If more than one method is applicable, the compiler emits an error. The second kind consists of [fully qualified calls](https://doc.rust-lang.org/reference/expressions/call-expr.html#r-expr.call.desugar), which can be used to resolve such ambiguities.


From the perspective of delegation, the alternatives can be categorized as follows:

1. Resolve the callee from the method name alone.

    [rust-lang/rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) suggested using only the method name to resolve the callee. This covers the most common scenario: delegating a trait implementation to another implementation of the same trait. However, this syntax does not generalize naturally to other caller/callee combinations ([?](#why-can-delegation-items-be-declared-in-any-position)) since it can lead to ambiguities in a similar way to method calls.

2. Resolve the callee from the fully qualified path.

   This approach covers every possible caller/callee combination ([?](#why-can-delegation-items-be-declared-in-any-position)) without ambiguity, but it requires more verbose and explicit syntax.

3. Use keywords as disambiguators.

    One of the suggestions in [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) is to use keywords (`trait`/`impl`/`fn`) to disambiguate the callee, for example, `reuse trait TraitName { expression }`. However, this approach doesn't generalize well to generic contexts. For example, it cannot distinguish between multiple generic implementations of the same trait.


The second option has been chosen for this proposal:

1. The first reason is that fully qualified paths already provide a uniform and well‑understood mechanism for disambiguation. Reinventing a separate keyword‑based approach (or any other alternative) would add unnecessary complexity.
2. The second reason is that the first option has already been proposed twice, in [rust-lang/rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). Rather than attempt the same approach a third time, this proposal comes at the problem from a different angle: name-based resolution can be reintroduced later as pure syntactic sugar layered on top of that mechanism. That keeps the door open to the first option in a forward-compatible way.

<details>

<summary> Example: disambiguation of methods without a receiver.</summary>

Consider a trait method without a receiver. In regular Rust code, calling such a method requires specifying the particular implementation of the trait, for example:

```rust
trait Trait { fn foo(); }

impl Trait for Inner { fn foo() {} }

impl Trait for Outer {
    fn foo() { Trait::foo(); } // ERROR
}

impl Trait for Outer {
    fn foo() { <Inner as Trait>::foo(); } // OK
}
```

The path `Trait::foo` alone is insufficient because multiple implementations may provide the same method. The same principle applies to delegation:

```rust
impl Trait for Outer { reuse Trait::foo; } // ERROR
```

Allowing `Self` in delegation paths makes this possible:

```rust
impl Trait for Outer { reuse <Inner as Trait>::foo; } // OK
```

</details>

<details>

<summary> Example: disambiguation of generic methods.</summary>

Consider multiple implementations of the same trait with generic parameters. Rust code calling a method of such a trait requires specifying the particular generic arguments, for example:

```rust
trait Trait<T> { fn foo(&self) {} }

impl Trait<i32> for Inner { fn foo(&self) {} }

impl Trait<()> for Inner { fn foo(&self) {} }

impl<T> Trait<T> for Outer {
    fn foo() { Trait::foo(); } // ERROR
}

impl<T> Trait<T> for Outer {
    fn foo(&self) { Trait::<()>::foo(&self.0) } // OK
}
```

The path `Trait::foo` alone is insufficient because multiple implementations may provide the same method. The same principle applies to delegation:

```rust
impl<T> Trait<T> for Outer { reuse Trait::foo { self.0 } } // ERROR
```

Allowing generic arguments in delegation paths makes this possible:

```rust
impl<T> Trait<T> for Outer { reuse Trait::<()>::foo { self.0 } } // OK
```

</details>

Also see [_Future possibilities: Name-based resolution as sugar_](#name-based-resolution-as-sugar)

↩ [_Paths and name resolution_](#paths-and-name-resolution)

#### Why is the delegation resolution the corresponding trait method in trait implementations?

With the _“Refined trait implementations”_ RFC ([rust-lang/rfcs#3245](https://github.com/rust-lang/rfcs/pull/3245)), an implementation signature may be more specific than the one declared in the trait.

If a delegation item is in a trait implementation (e.g., `impl Trait for Type { /*delegate foo*/ }`), we have two options:

1. Inherit information from the function resolved via the callee path.

    This option supports refined implementations.

2. Inherit information from the trait method itself.

    This option supports cases where the callee has a different signature but can still be called because of argument or return-value coercions:

    ```rust
    trait MirPass<'tcx> {
        fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>);
    }

    trait MirLint<'tcx> {
        fn run_lint(&self, tcx: TyCtxt<'tcx>, body: &Body<'tcx>);
    }

    pub(super) struct Lint<T>(pub T);

    impl<'tcx, T> MirPass<'tcx> for Lint<T>
    where
        T: MirLint<'tcx>
    {
        fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
            self.0.run_lint(tcx, body)
        }
    }
    ```
    Here, `run_lint` accepts `&Body<'tcx>`, while the trait method `run_pass` requires `&mut Body<'tcx>`. The delegation can work because `&mut T` can coerce to `&T`. However, if the generated function copied its signature from the callee, `run_pass` would instead take `&Body<'tcx>`, violating the trait definition and resulting in a compilation error.

The `#[refine]` attribute proposed by RFC 3245 could potentially be used to switch between these behaviors. We suggest inheriting signatures from the trait by default.

↩ [_Desugaring of individual delegation_](#desugaring-of-individual-delegation-signature)

#### Why are function qualifiers copied unchanged?

The function header comprises qualifiers such as `const`, `async`, `unsafe`, and `extern "ABI"`. The following alternatives exist:

1. Behave identically to regular functions.

    Delegation items use the same defaults: `const`, `async`, and `unsafe` are omitted, and `extern "Rust"` is used. Specifying a qualifier overrides the corresponding default for the generated function. This works when the callee also uses the default qualifiers. If the callee uses different qualifiers:

    - `const`: If the callee is `const` and the delegation item is not, there is no problem: a `const` function can be called from a non-`const` function. However, the delegation item cannot be called from a const context unless `const` is also specified on the delegation item.
    - `ABI`: A mismatch here does not prevent the call from compiling, but it is difficult to see where that would be useful, and the user usually would have to restate the ABI for the delegation item.
    - `unsafe`: Calling an `unsafe` function from a non-`unsafe` function requires wrapping the call in an `unsafe` block. We do not want this to happen silently, so the delegation item would have to be marked `unsafe`. Otherwise, the compiler would emit an error.
    - `async`: Forwarding to an `async` callee from a non-`async` delegation item isn't possible without changing what gets generated.

2. Inherit qualifiers from the delegation resolution.

The proposal chooses to inherit all function qualifiers from the delegation resolution unchanged. The main problem with the first approach is verbosity. Matching the delegation resolution's qualifiers is essentially the only sensible choice, yet that approach would force users to repeat qualifiers for delegation items.

↩ [_Desugaring of individual delegation_](#desugaring-of-individual-delegation-signature)

#### Why is delegation of variadic functions not supported?

There are pretty fundamental [implementability issues](https://github.com/rust-lang/rust/issues/127443) with passing through C variadic arguments.

↩ [_Desugaring of individual delegation_](#desugaring-of-individual-delegation-signature)

#### Why are inference placeholders allowed in paths?

1. If substitution of own parameters is allowed, inference placeholders can be used to substitute only a subset of the parameters:
   ```rust
   pub fn foo<T, U>(x: T, y: U) { /* impl */ }
   reuse foo::<i32, _> as bar;
   ```
   This could desugar to:
   ```rust
   pub fn bar<U>(x: i32, y: U) {
      foo(x, y)
   }
   ```

2. If a parameter isn't present in the signature or where clauses, there is nothing to substitute and we can omit the full name.

↩ [_Generics remapping_](#generics-remapping)

#### Why are nested inference placeholders not allowed in paths?

> [!WARNING]
>
> The idea below is unconventional, and this RFC does not propose it. It is included for completeness only: we are not currently aware of a use case for it, and treating a nested inference placeholder as an error remains the better default.

Delegation could be made to work with nested inference placeholders (e.g., `Vec<_>`) by generating a new parameter for each placeholder:

<!-- compile-fail: not currently supported -->
```rust
 fn foo<T>(x: T) {}
 reuse foo::<HashMap::<_, _>>;
```

This could desugar into:
```rust
fn bar<A, B>(x: HashMap<A, B>) {
    foo::<HashMap::<_, _>>(x)
}
```

↩ [_Generics remapping_](#generics-remapping)


#### Why might own generic parameters need to be substituted?

Consider the example:

```rust
pub const fn max_leb128_len<T>() -> usize { /* impl */ 42 }

pub const fn largest_max_leb128_len() -> usize {
  max_leb128_len::<u128>()
}
```

With the ability to provide generic arguments for own parameters, the `largest_max_leb128_len` implementation could be replaced with the delegation item `reuse max_leb128_len::<u128>`.

↩ [_Generics remapping_](#generics-remapping)

#### Why is the `Self` type not substituted?

In the name resolution section, we provided an example of how the `Self` type argument can be used to disambiguate a callee without a receiver. However, that argument does not participate in generic substitution; that is, the parent `Self` parameter always refers to the `Self` type of the current context. Consider the example:

```rust
trait Iterator {
    type Item;
    fn any<F>(&mut self, f: F) -> bool
    where
        Self: Sized,
        F: FnMut(Self::Item) -> bool;
}

pub struct UnordItems<T, I: Iterator<Item = T>>(I);

impl<T, I: Iterator<Item = T>> UnordItems<T, I> {
    pub fn any<F: Fn(T) -> bool>(mut self, f: F) -> bool {
        self.0.any(f)
    }
}
```

Suppose we replace the implementation of `UnordItems::any` with the delegation item `reuse Iterator::any { self.0 }`. The generated method looks like this:

```rust
impl<T, I: Iterator<Item = T>> UnordItems<T, I> {
    pub fn any<F: FnMut(<?Self as Iterator>::Item) -> bool>(mut self: &mut ?Self, f: F) -> bool {
        Iterator::any(&mut self.0)
    }
  ...
}
```

Here, `F` can be copied directly because it is one of `Iterator::any`'s own parameters. However, `Self` is defined by the `Iterator` trait itself. To make the example work, we would need to:
1. Substitute `?Self` in `<?Self as Iterator>::Item` with `I` (e.g., with `reuse <I as Iterator>::any { self.0 }`).
2. Substitute `?Self` in `mut self: &mut ?Self` with `UnordItems<T, I>`, i.e., leave the parameter as a regular receiver.

Thus, `?Self` would need to be substituted with different types depending on its position in the signature or where clauses.

↩ [_Generics remapping_](#generics-remapping)

#### Why might parent parameters need to be substituted?

Consider the example:

```rust
impl<K, V, A: AllocatorClone> BTreeMap<K, V, A> {
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    { /* impl */ false }
}

impl<T, A: AllocatorClone> BTreeSet<T, A> {
    reuse BTreeMap::contains_key as contains { self.map } // ERROR
}
```

The generated method looks like this:

```rust
impl<T, A: AllocatorClone> BTreeSet<T, A> {
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        ?K: Borrow<Q> + Ord,
        Q: Ord,
    {
        BTreeMap::contains_key(&self.map, value)
    }
}
```

Here, `Q` can be copied directly because it is one of `BTreeMap::contains_key`'s own parameters. However, `K`, defined in `BTreeMap`, needs to be remapped to `T`, defined in `BTreeSet` (we use `?K` to denote a parameter that has been copied but not yet remapped).

To make the example work, the parameter can be explicitly substituted through the path:

```rust
reuse BTreeMap::<T, (), A>::contains_key as contains { self.map }
```

↩ [_Generics remapping_](#generics-remapping)

#### What happens if unsubstituted parent parameters remain after substitution?

Consider the example:

```rust
impl<K, V, A: AllocatorClone> BTreeMap<K, V, A> {
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    { /* impl */ false }
}

impl<T, A: AllocatorClone> BTreeSet<T, A> {
    reuse BTreeMap::contains_key as contains { self.map } // ERROR
}
```

The `K` parameter defined in `BTreeMap` has not been substituted with the `T` parameter defined in `BTreeSet`, so the generated method looks like this:

```rust
impl<T, A: AllocatorClone> BTreeSet<T, A> {
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        ?K: Borrow<Q> + Ord,
        Q: Ord,
    {
        BTreeMap::contains_key(&self.map, value)
    }
}
```

Here, `?K` denotes a parameter that has been copied but not remapped. There are several options we could consider:

1. Report an error.
2. Infer the parameter from the surrounding context:
   1. From the target block: `typeof(self.map) == BTreeMap::<T, ()>`

      We would need to type-check the function body before generating the full signature, which is not possible with the current compiler architecture. Similar problems exist for delegating to [_type-relative paths_](#paths-and-name-resolution).

   2. The compiler could use a heuristic to substitute parameters defined in the implementation header (e.g., positional 1:1 matching or substituting parameters with the same names). But this approach is fragile and fails whenever generic parameters are reordered, partially instantiated, or renamed.

3. We could generate a new parameter and substitute `?K` with it. This would not pass type checking in the example above, but it might be useful in other cases ([?](#what-happens-if-unsubstituted-parent-parameters-remain-after-substitution-part-2)).

In this proposal, we suggest using the “report an error” option because it is the most conservative approach and requires generic arguments to be specified explicitly. Once the compiler architecture is sufficiently advanced, we can implement more sophisticated inference.

Also see [_Future possibilities: More sophisticated inference of generic parameters_](#more-sophisticated-inference-of-generic-parameters)

↩ [_Generics remapping_](#generics-remapping)

#### Why are effective self types identified beyond the receiver?

TODO: explain the rationale.

↩ [_Effective self type identification_](#effective-self-type-identification)

#### Why are effective self types not detected through type aliases or type equality?

TODO important: explain the rationale. <br>
TODO: future possibilities - allow users to opt in to marking types as effective self types, or use type equality checks and allow users to opt out. <br>
TODO: future possibilities - extend the set of uses of the effective self type to which the conversions apply.

↩ [_Effective self type identification_](#effective-self-type-identification)

#### Why are specific smart pointers used for detecting effective self parameters?

TODO important: explain the rationale. <br>
TODO: future possibilities - extend the set of uses of the effective self type to which the conversions apply. <br>
TODO: think about [field projections](https://github.com/BennoLossin/rfcs/blob/field-projection-v2/text/3735-field-projections.md)

↩ [_Effective self type identification_](#effective-self-type-identification)

#### Why are self return types limited to bare self type?

TODO important: explain the rationale. <br>
TODO: future possibilities - extend the set of uses of the effective self type to which the conversions apply.

↩ [_Effective self type identification_](#effective-self-type-identification)

#### Why is the target block a block expression?

In contrast to [rust-lang/rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393), this RFC uses a block rather than a bare expression (e.g., a hypothetical `reuse prefix::name from expr;`) because a block expression can contain many statements. While having multiple statements during delegation is expected to be a niche use case, anchoring the syntax to the most general form is consistent with our [_guiding principles_](#design-guiding-principles).

↩ [_Target block_](#target-block)

#### Why is the target block unrestricted?

In feedback on [rust-lang/rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406), it was suggested that delegation be limited to fields. This suggestion was adopted in [rust-lang/rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). However, we see no compelling reason for this restriction either from an implementation perspective or from the perspective of the language itself. Also see [_guiding principles_](#design-guiding-principles).

↩ [_Target block_](#target-block)

#### Why can target blocks be allowed without effective self parameters?

TODO important: explain the rationale. <br>
TODO drawback: discuss future compatibility issues with identity in target blocks

↩ [_Target block_](#target-block)

#### Why is `self` the implicit target expression?

TODO: explain the rationale.

↩ [_Target block_](#target-block)

#### Why are statements not passed to the call?

Suppose we have a delegation item:

```rust
reuse path::name { let x = something; self.get(x) }
```

There are two possible ways to generate the call:
- Pass the block expression unchanged:
  ```rust
  path::name(..., { let x = something; self.get(x) }, ...,)
  ```
- Hoist the statements out of the block:
  ```rust
  let x = something;
  path::name(..., self.get(x), ...,)
  ```

Passing the whole block suppresses all the possible adjustments and moves the trailing expression out of the block, which is not what we want.
The more detailed discussion of the choice can be found in [this github comment](https://github.com/rust-lang/rfcs/pull/3530#issuecomment-2197170600).

↩ [_Body desugaring_](#body-desugaring)

#### Why are method-call adjustments applied to effective self parameters?

TODO important: explain the rationale.

↩ [_Body desugaring_](#body-desugaring)

#### Why are the delegation resolution's own generic parameters substituted as arguments to the final segment?

Suppose we have a delegation item:

```rust
fn foo<T>(x: i32) {}
reuse foo as bar;
```

There are two possible ways to generate the call:
- Propagate generic parameters to the call:
  ```rust
  fn bar<T>() { foo::<T>() } // Ok
  ```
- Do not propagate generic parameters to the call:
  ```rust
  fn bar<T>() { foo() } // ERROR: type annotations needed
  ```

The first option should be chosen because otherwise the generated call may fail with a type inference error.

↩ [_Body desugaring_](#body-desugaring)

#### Why is return type wrapping restricted to single-field structs?

TODO important: explain the rationale.

↩ [_Return type wrapping_](#return-type-wrapping)

#### What happens if unsubstituted parent parameters remain after substitution? Part 2.

> [!WARNING]
>
> The idea below is unconventional, and this RFC does not propose it. It is included for completeness only: we are not currently aware of a use case for it, and treating an unsubstituted parent parameter as an error ([_Part 1_](#what-happens-if-unsubstituted-parent-parameters-remain-after-substitution)) remains the better default.

If a parent parameter remains unsubstituted in the signature or where clauses after substitution, one possible alternative is to generate an additional generic parameter. Consider the example:

```rust
trait Ord: Eq + PartialOrd<Self> {
    fn min(self, other: Self) -> Self
    where
        Self: Sized;
}

fn min<T: Ord + Sized>(v1: T, v2: T) -> T {
     Ord::min(v1, v2)
}
```

Traits include an implicit `Self` parameter that can be modeled as a generic parameter `This: Trait`. If we allow parameters to be copied from the delegation resolution’s parent trait, the `min` implementation could be replaced with `reuse Ord::min;`.

In principle, this could extend beyond `Self` to any parent parameter, but doing so raises multiple questions:

- Rust requires lifetime parameters to be declared before type and const parameters, which means copied generics may need to be reordered.
- Default parameters are not permitted in functions. Therefore, either the default type must be used, or a new non-default parameter must be generated.
- Bounds also need to be copied.

This extension is implemented in nightly rustc, and the example below compiles.

<details>

<summary> Example: copying parent parameters.</summary>

```rust
trait Trait1 {}

trait Trait<'a, A: Trait1, C = i32> {
    fn foo<'b, B>(&self, x: A, y: B, c: C);
}

reuse Trait::foo;
```

A conceptual desugaring might look like this:

```rust
fn foo<'a, 'b, This, A: Trait1, B>(this: &This, x: A, y: B, c: i32)
where
    This: Trait<'a, A>,
{
    <This as Trait<'a, A>>::foo::<B>(this, x, y, c)
}
```

</details>

↩ [_Generics remapping_](#generics-remapping)

### Alternatives to this RFC

#### Macros

See [_Prior art: delegate_](#cratesiodelegate) and [_Prior art: ambassador_](#cratesioambassador) for a closer look at the two most widely used delegation crates.

Both show that delegation can already be built as a library, with no change to the language, and both are mature and reasonably ergonomic. However, both are ultimately limited by what a macro can see: macros do not have access to type information such as the callee's resolved signature or the methods of a trait.

Closing this gap fully would require the macro to see type information during expansion, which is the [_reflection_](#reflection) capability discussed as an alternative below.

#### Reflection

An alternative approach to delegation in Rust would be some form of compile-time reflection. Given the ability to inspect type information such as function signatures during macro expansion, delegation could be implemented as a third-party library, removing the need for dedicated language support.

However, reflection is a large and complex feature that may take years to implement and stabilize. Even if it becomes available, it is not clear that it would be a suitable mechanism for delegation.

Work in this direction is already being explored. See the [reflection project goal](https://github.com/rust-lang/rust-project-goals/issues/406).

#### Embedding

Rust could instead adopt some form of type embedding, where an anonymous field's methods are automatically "promoted" onto the outer struct's method set.

Go has a working version of this idea. See [_Prior art: Type embeddings in Go_](#type-embeddings-in-go).

[rust-lang/rfcs#2431](https://github.com/rust-lang/rfcs/issues/2431), opened in 2018, sketches a mechanism for Rust. The issue was posted as a rough idea seeking feedback, but it received little response and remains open with no further activity.

#### Language support for newtypes

An alternative to this RFC would be to add language support specifically for newtypes, allowing requested traits to be derived automatically. This narrower idea has been proposed repeatedly over the years: [rust-lang/rfcs#261](https://github.com/rust-lang/rfcs/issues/261), [rust-lang/rfcs#186](https://github.com/rust-lang/rfcs/pull/186), [rust-lang/rfcs#949](https://github.com/rust-lang/rfcs/pull/949), [rust-lang/rfcs#2242](https://github.com/rust-lang/rfcs/pull/2242), [rust-lang/rfcs#3596](https://github.com/rust-lang/rfcs/issues/3596), [rust-lang/rfcs#3951](https://github.com/rust-lang/rfcs/pull/3951).

The last attempt ([rust-lang/rfcs#3951](https://github.com/rust-lang/rfcs/pull/3951)) was closed by the lang team with a [message](https://github.com/rust-lang/rfcs/pull/3951#issuecomment-4917471822):


> We gave this a brief review in our @rust-lang/lang meeting today.
>
> The meeting consensus was that we don't really see the need to use a tuple struct as a problem to be solved; we agree that it'd be nice to have easier ways to delegate trait impls and so forth (like a delegation RFC), but adding a new concept (newtype) that is still effectively-a-struct-but-different doesn't feel like enough of a win to warrant expanding our language surface in this way.
>
> Thank you for opening the PR! It's always great to see suggestions and thoughts on how to make Rust better.


#### Inheritance

Rust could instead adopt some form of inheritance closer to what object-oriented languages provide. However, inheritance has been discussed extensively in the context of Rust, and it is generally not considered aligned with the language's design philosophy.

Also see the [Rust Book](https://doc.rust-lang.org/book/ch18-01-what-is-oo.html#inheritance-as-a-type-system-and-as-code-sharing).

## Prior art
[prior-art]: #prior-art

### Delegation or similar mechanisms in other languages

#### [Export clauses in Scala](https://docs.scala-lang.org/scala3/reference/other-new-features/export.html)

Scala 3's `export` clause has the form `export path . { sel_1, ..., sel_n }` and defines aliases for selected members of an object.

<details>

<summary> Example: Scala export clause.</summary>

```scala
class Inner:
  def hello(): String = "hello"

class Outer(inner: Inner):
  export inner.hello

@main def run(): Unit =
  println(new Outer(new Inner()).hello())
```

</details>

Its selectors line up closely with this proposal's three function delegation forms: a single selector corresponds to individual delegation, multiple selectors correspond to list delegation, and a wildcard selector (`*`) corresponds to glob delegation.

`x as y` renames a member on export, using the same `as` keyword that this RFC uses for renaming.

#### [Delegation in Kotlin](https://kotlinlang.org/docs/delegation.html)

Kotlin supports interface delegation natively via a `by` clause on the supertype list: `class Derived(b: Base) : Base by b` implements `Base` for `Derived` by forwarding every one of its methods to `b`. It is close in spirit to this proposal's glob delegation.

Kotlin also lets `Derived` override individual delegated members instead of taking all of them from `b`.

Kotlin [extends](https://kotlinlang.org/docs/delegated-properties.html) the same `by` keyword to individual properties, e.g., `val x: Int by lazy { computeX() }`. There, the expression after `by` is a delegate object providing `getValue` and `setValue` operator functions that the compiler invokes whenever `x` is read or written. This is a related but distinct feature with no direct equivalent proposed here.

#### [Type embeddings in Go](https://go.dev/ref/spec#Struct_types)

Go has no inheritance either and addresses the same problem through struct embedding. A struct field declared with only a type, no name, is _embedded_. The embedded value's fields and methods become available directly on the outer struct (`outer.Method()` instead of `outer.inner.Method()`).

<details>

<summary> Example: Go struct embedding.</summary>

```go
package main

import "fmt"

type Inner struct{}

func (Inner) Hello() {
    fmt.Println("hello world")
}

type Outer struct {
    Inner
    Name string
}

func main() {
    o := Outer{Inner: Inner{}, Name: "name"}
    o.Hello() // promoted from Inner; no o.Inner.Hello() needed
}
```

</details>

### Related proposals in Rust

#### [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) (2015, closed)

Delegation was first proposed in [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406). This RFC introduces new syntax within trait `impl` blocks, permitting a type to forward an entire trait implementation (or selected items) to a field or arbitrary expression that already implements that trait. The proposed syntax has the following forms:
- `impl Trait for Type { use expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { use expression for name_1 (, name_i)*; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `typeof(expression)` implements `Trait`.

The main reasons for rejecting the proposal were:

1. _Unclear semantics._ It's not clear what kinds of expressions are allowed in the delegation body. The behavior of `self` is underspecified. The mechanism for desugaring is not defined. Also see this [comment](https://github.com/rust-lang/rfcs/pull/1406#issuecomment-269175112).
2. _Forward compatibility._ The RFC intentionally leaves many features for future work, but there was insufficient evidence that the proposed design could be clearly extended to those features without breaking semantics and requiring a redesign.

#### [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) (2018, closed)

Delegation was proposed again in [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). The design was stricter to address the semantic ambiguities of the earlier proposal. The syntax has the following forms:
- `impl Trait for Type { delegate * to expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { delegate fn name_1 (, fn name_i)* to expression; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `expression` resolves to a field of `self` (e.g., `self.field`) and `typeof(expression)` implements `Trait`.

Delegation is allowed only for methods that take a receiver by value, by shared reference, or by mutable reference. The proposed desugaring scheme translates a delegation item into a [method call](https://doc.rust-lang.org/reference/expressions/method-call-expr.html).

The main reasons for rejecting the proposal were:

1. The second proposal was [postponed](https://github.com/rust-lang/rfcs/pull/2393#issuecomment-816822011) because of the lang team's limited bandwidth.
2. Additionally, forward compatibility concerns were never fully addressed.

#### [rfcs2375](https://github.com/rust-lang/rfcs/pull/2375) (2018, open)

This RFC proposes an `#[inherent]` attribute that allows a trait implementation's methods to be called directly on a type without bringing the trait into scope. For example, given:

<!-- compile-fail: hypothetical feature -->
```rust
#[inherent]
impl Bar for Foo { ... }
```

The methods defined in `Bar` can be called directly on instances of `Foo`, even if `Bar` is not in scope. The RFC defines `#[inherent]` as sugar for a forwarding inherent method:

```rust
impl Foo {
    #[inline]
    pub fn bar(&self) { <Self as Bar>::bar(self); }
}
```


Later in the PR discussion, [nikomatsakis proposed](https://github.com/rust-lang/rfcs/pull/2375#issuecomment-1722647937) replacing `#[inherent]` with `use`, which is almost the same as the delegation item `pub reuse Bar::bar;` under this RFC.

#### [rfcs#3591](https://github.com/rust-lang/rfcs/pull/3591) (2024, merged)

This RFC allows a `use` declaration to bring a trait's associated functions and constants into scope by path, e.g., `use SomeTrait::some_fn;`. This is not delegation: `use Trait::func` creates a local name for an existing associated function and does not define a new item. However, the same use case can be expressed through the delegation feature.

#### [rfcs#3911](https://github.com/rust-lang/rfcs/pull/3911) (2026, open)

This RFC allow deriving an implementation of the `Deref` trait using `#[derive(Deref)]` on structs and enums, which allows emulating delegation with `Deref` coercions more conveniently.

### Crates

#### [crates.io/delegate](https://crates.io/crates/delegate)

`delegate` is the most widely used crate for delegation. It implements the `delegate!` declarative macro, which delegates method calls to selected expressions.

<details>

<summary> Example: delegate macro.</summary>

```rust
struct Inner;
impl Inner {
    pub fn method_res(&self, num: u32) -> Option<u32> { Some(num) }
}

struct Wrapper {
    inner: Inner
}

impl Wrapper {
    delegate! {
        to self.inner {
            // calls method_res, unwraps the result, then calls into
            #[unwrap]
            #[into]
            #[call(method_res)]
            pub fn method_res_into(&self, num: u32) -> u64;
        }
    }
}
```

</details>

_Strengths_:

1. It supports a broad range of transformations through attributes such as `#[into(u64)]`, `#[unwrap]`, `#[await(true/false)]`, and many others, which can modify the signature or body of the generated method. This makes the macro applicable to a wide range of delegation patterns.
2. It is not limited to trait implementations.

_Weaknesses_:

1. Declarative macros have no access to the callee's actual signature. Every delegated method's signature must be restated by hand in the macro definition.


#### [crates.io/ambassador](http://crates.io/crates/ambassador)

`ambassador` is the second most popular crate for delegation. In contrast to [delegate](https://crates.io/crates/delegate), it uses procedural macros rather than declarative macros.

<details>

<summary> Example: ambassador macro.</summary>

```rust
use ambassador::delegatable_trait;

#[delegatable_trait]
pub trait Trait {
    fn method(&self, input: &str) -> String;
}

pub struct Inner;

impl Trait for Inner {
    fn method(&self, input: &str) -> String {
        String::new()
    }
}

#[derive(Delegate)]
#[delegate(Trait)]
pub struct Outer(Inner);
```

</details>

_Strengths_:

1. Unlike `delegate`, the callee's signature does not need to be restated at each delegation site.

_Weaknesses_:

1. It supports a narrower range of delegation patterns than `delegate`: Ambassador can only delegate trait implementations, delegates only to fields, and does not support transformations of the delegated method's signature.
2. The trait being delegated must be annotated with `#[delegatable_trait]`. For foreign traits, it must instead be redeclared locally with `#[delegatable_trait_remote]`.
3. In `#[delegate(..., target = "self")]` or `#[delegate(..., where = "A: Shout")]`, expressions are specified as strings rather than in regular Rust syntax.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

The questions below are not expected to block acceptance of this RFC. They can be settled during implementation or before stabilization.

### Which attributes should be added by default?

Certain attributes may be reasonable to add or copy from the callee by default. The current implementation adds the `#[inline]` attribute and copies the `#[must_use]` attribute.

There should also be a way to opt out of default attributes when they are not desired. For `#[inline]`, this may be done with `#[inline(never)]` on the delegation item, but the appropriate mechanism depends on the attribute, and some attributes may have no corresponding way to opt out.

↩ [_Reference-level explanation_](#reference-level-explanation)

### Should the visibility of the delegation item be restricted?

The ability to specify a reused function's visibility independently of the original's provides welcome flexibility, yet it introduces a potential semver hazard:

```rust
fn foo<T: Copy>(x: T) { /* impl */ }
pub reuse foo as bar;
```

If the signature of `foo` changes, the generated function `bar` changes accordingly. In regular Rust code, such a signature change would cause a type error at every call site, forcing the author to update callers. With `reuse`, however, the change propagates silently. As a result, modifications intended to be internal may accidentally become breaking changes for downstream crates.

Taking this into consideration, several design choices are possible:

1. The visibility of the generated function is taken solely from the callee.
2. The visibility of the generated function cannot exceed the visibility of the reused function. In other words, delegation may only preserve or reduce visibility, never increase it.
3. The user explicitly controls visibility.

We prefer to give the user full control while also adding a lint that warns if a generated function reachable from other crates forwards to a function that is not reachable.

↩ [_Reference-level explanation_](#reference-level-explanation)

## Future possibilities
[future-possibilities]: #future-possibilities

Several extensions could be added on top of the core feature without changing its fundamental semantics. At the same time, the scope for such extensions is relatively limited.

### Name-based resolution as sugar

A shorter syntax that infers the callee from a bare method name could be layered on top of fully qualified paths.

↩ [_Paths and name resolution_](#paths-and-name-resolution)

### More sophisticated inference of generic parameters

We could implement a more advanced mechanism for inferring unsubstituted generic parameters, allowing users to specify fewer generic arguments explicitly.

See ([?](#what-happens-if-unsubstituted-parent-parameters-remain-after-substitution)) and ([?](#what-happens-if-unsubstituted-parent-parameters-remain-after-substitution-part-2)) for more details.

↩ [_Generics remapping_](#generics-remapping)

### Support for delegating types and consts

We could support desugaring for types and consts as follows.
It would allow using glob delegation and `reuse impl` more effectively, without manual overrides for types and constants.

<!-- compile-fail: not yet supported -->
```rust
impl Trait for S {
    reuse Trait::{Item, MAX, func} { self.0 }
}

impl Trait for S {
    type Item = <F as Trait>::Item;
    const MAX = <F as Trait>::MAX;
    fn func(&self) -> u32 {
        <F as Trait>::func(&self.0)
    }
}
```

For associated constants the delegation rules should naturally follow from the function delegation rules, since constants are more or less equivalent to functions with zero parameters and return type matching the constant's type.

However, there are some complications.
For example, types live in the type namespace, while functions and constants live in the value namespace. A single qualified path doesn't say which namespace to pull from, so `Trait::name` is ambiguous whenever `Trait` has both an associated type and an associated function or constant called `name`.
This can be solved by introducing a disambiguator for types. One of the suggestions in [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) is to use `fn`/`type`/`const` keywords.

For these reasons, we would like to postpone delegation of types and constants.

↩ [_Reference-level explanation_](#paths-and-name-resolution)

### Empty list delegation

Supporting empty list or glob delegations, such as `reuse prefix::{};` or `reuse MarkerTrait::*;`, requires keeping a stub for the prefix in the AST and HIR after the list/glob expansion, so the prefix can be resolved and checked for stability.

The implementation therefore has some cost for little benefit. Implementing this makes little sense unless the feature is accepted and stabilized.

Not resolving the prefix and accepting `reuse nonexistent::path::{};` would be surprising.
Resolving the prefix but not checking it for stability would be a compatibility hazard (if an unstable API is removed).

↩ [_List delegation_](#list-delegation)

### Supporting type-relative paths

Several approaches could be considered for supporting type-relative paths:
1. We could generate an incomplete body (e.g., without arguments), then use analysis passes in HIR to infer the missing information and complete body generation during lowering to MIR/THIR.
2. We could lower everything except delegation items, run the analysis passes, and then finish lowering the delegation items.

This would require substantial compiler refactoring, so we do not have a strong opinion on this.

↩ [_Paths and name resolution_](#paths-and-name-resolution)
