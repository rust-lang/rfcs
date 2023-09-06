- Feature Name: `cfg_attribute_in_where`
- Start Date: 2023-03-11
- RFC PR: [rust-lang/rfcs#3399](https://github.com/rust-lang/rfcs/pull/3399)
- Rust Issue: [rust-lang/rust#115590](https://github.com/rust-lang/rust/issues/115590)

# Summary
[summary]: #summary

Let's make it more elegant to conditionally compile trait bounds by allowing cfg-attributes directly in where clauses.

# Motivation
[motivation]: #motivation

Currently, there is limited support for conditionally compiling trait bounds. Rust already supports using cfg-attributes in 
angle-bracketed bounds, so the following implementation is possible but unwieldy, and grows combinatorically with multiple 
independent compilation condition/bound pairs:

```rust
impl<
    #[cfg(something)] T: SomeRequirement, 
    #[cfg(not(something))] T
> SomeTrait<T> for Thing {}
```

This also can't be used for bounds on associated types or other more complicated left-hand items that can only occur in full where bounds.

Another somewhat-common approach is to create a dummy trait that conditionally branches and implement that, like so:

```rust
#[cfg(something)]
trait Dummy: SomeRequirement {}
#[cfg(something)]
impl<T: SomeRequirement> Dummy for T {}
#[cfg(not(something))]
trait Dummy {}
#[cfg(not(something))]
impl<T> Dummy for T {}

impl<T: Dummy> SomeTrait<T> for Thing {}
```

However, this boilerplate does not grow well for multiple conditionally-compiled requirements, becoming rather soupy even at N = 2:

```rust
#[cfg(something_a)]
trait DummyA: SomeRequirementA {}
#[cfg(something_a)]
impl<T: SomeRequirementA> DummyA for T {}
#[cfg(not(something_a))]
trait DummyA {}
#[cfg(not(something_a))]
impl<T> DummyA for T {}

#[cfg(something_b)]
trait DummyB: SomeRequirementB {}
#[cfg(something_b)]
impl<T: SomeRequirementB> DummyB for T {}
#[cfg(not(something_b))]
trait DummyB {}
#[cfg(not(something_b))]
impl<T> DummyB for T {}

impl<T: DummyA + DummyB> SomeTrait<T> for Thing {}
```

Other alternative ways of achieving this also exist, but are typically macro heavy and difficult to implement or check. Importantly, this 
functionality already exists in the language, but quickly grows out of reasonable scope to ergonomically implement.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`where` clauses can use cfg-attributes on individual trait bounds, like so:

```rust
impl<T> SomeTrait<T> for Thing
where
    #[cfg(something_a)] T: SomeRequirementA,
    #[cfg(something_b)] T: SomeRequirementB,
{}
```
or on functions, including multiple cfg-attributes on a single bound:
```rust
fn some_function<T>(val: &T)
where
    #[cfg(something_a)] 
    T: SomeRequirementA,
    #[cfg(something_b)] 
    #[cfg(not(something_a))] 
    #[cfg(target_os(some_os))] 
    T: SomeRequirementB,
{}
```
and in other situations where `where` clauses apply.

During compilation, all cfg-attributes on a where bound are evaluated. If the evaluation result is false, then the bound in question is not
compiled and the bound does not apply to the given type. This may cause errors if code that relies on those bounds is not itself also 
conditionally compiled. For anyone familiar with cfg-attributes already, this should behave similarly to how they are used in, say, struct 
fields or on function signatures.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In positions that accept where clauses, such as trait implementations and function signatures, individual clauses can now be decorated with 
cfg-attributes. The cfg-attribute must be on the left hand of the colon (e.g. `#[cfg(...)] T: Foo` rather than `T: #[cfg(...)] Foo`) and 
applies to that one bound, up to the comma or end of the where section. Each bound collection will be conditionally compiled depending on the 
conditions specified in the cfg arguments. Note that this may cause a where clause to conditionally compile as having no bound entries 
(i.e. an empty where clause), but this has been allowed in Rust since 1.16 and already occurs from time to time when using macros.

# Drawbacks
[drawbacks]: #drawbacks

As with any feature, this adds complication to the language and grammar. In general, conditionally compiled trait bounds can create 
unintended interactions or constraints on code based on compilation targets or combinations of features. The drawbacks to this proposed 
code path already apply to the existing workarounds used to achieve the same functionality.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This functionality can already be achieved in Rust, but not elegantly, and without a clear relationship between the written code and its
intent. The two main alternatives are dummy traits and cfg-attributes in angle-bracketed bounds. Compared to using dummy traits, adding a 
cfg-attribute in a where clause makes the intent immediately local and more directly associates it with the piece of code it's intended to 
control. Compared to using cfg-attributes in angle-bracketed bounds, adding a cfg-attribute in a where clause means each bound can be 
individually toggled without the need for combinatoric combinations of conditions, and allows conditional compilation on bounds with 
nontrivial item paths.

The need for conditionally compiling trait bounds can arise in applications with different deployment targets or that want to release 
builds with different sets of functionality (e.g. client, server, editor, demo, etc.). It would be useful to support cfg-attributes 
directly here without requiring workarounds to achieve this functionality. Macros, proc macros, and so on are also ways to conditionally 
compile where clauses, but these also introduce at least one level of obfuscation from the core goal. Finally, traits can be wholly 
duplicated under different cfg-attributes, but this scales poorly with both the size and intricacy of the trait and the number of 
interacting attributes (which may grow combinatorically), and can introduce a maintenance burden from repeated code.

# Prior art
[prior-art]: #prior-art

I'm not aware of any prior work in adding this to the language. Languages with preprocessors could support this with something like:

```rust
impl<T> SomeTrait<T> for Thing
where
#ifdef SOMETHING_A
    T: SomeRequirementA
#endif
{}
```
but that's not the way I would expect Rust to provide this kind of functionality.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* In theory, I don't see any harm in cfg-attributes decorating individual bounds on the right hand side of the colon. Is it worth adding that
potential feature as well? Personally, I don't see it as being worth the added complexity given that you can have multiple individual bound
declarations for the same item. Doing so would also create an inconsistency, given that this isn't currently allowed in angle-bracketed 
bounds either.

* rustfmt is supposed to be able to format the where clause somehow, do we expect it to (try to) put the attribute on the same line, or would it always prefer the attribute on separate lines?

# Future possibilities
[future-possibilities]: #future-possibilities

Conditional bounds on where clauses could also be used for [trivial bounds](https://github.com/rust-lang/rust/issues/48214). I don't believe 
any extra support would be needed here since the conditional compilation would occur at the grammar level rather than the type level.
