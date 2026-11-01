- Feature Name: `attributes_galore`
- Start Date: 2018-11-25
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

[proptest]: https://github.com/altsysrq/proptest

Permit attributes to be attached to lifetimes, types, bounds, and constraints
as well as associated type equality constraints `Foo<#[attr] Assoc = MyType>`.
For example, in a hypothetical version of [proptest] one may write:

```rust
#[proptest]
// #[quickcheck] could also work similarly.
fn addition_commutes(
    // Run this test for (u8, u8), (u16, u16), and (u32, u32);
    // This is interpreted by `#[proptest]`.
    (a, b): #[types(T, u8, u16, u32)] (T, T)
) -> bool {
    a + b == b + a
}

// Accept x: u8 and vec: Vec<{ y: u8 | y > x }>.
// This is then verified by an external tool.
fn refined(x: u8, vec: Vec<#[require = "> x"] u8>) { ... }
```

# Motivation
[motivation]: #motivation

The motivation in this RFC is quite straightforward and boils down to making
language more extensible for assorted purposes. Among those are:

1. In rust compilers themselves. While most of the built-in compiler provided
   attributes do not make sense when placed on types directly, attributes
   could be provided in the future that do make sense. For example,
   one could imagine a special structural record type using the syntax:

   ```rust
   type Thing = #[repr(C)] { foo: u16, bar: u16 };
   ```

   This type is ordered according to the C ABI,
   unlike the variant lacking the attribute.
   That makes the type usable for FFI purposes in crates such as `winapi`.

   Other uses of placing attributes on types could be for internal purposes
   inside the `rustc` compiler.

[linear types]: https://en.wikipedia.org/wiki/Substructural_type_system#Linear_type_systems

2. For (static analysis) tools. For example, we could imagine an add-on type
   system providing a stronger variant of `#[must_use]` amounting to
   *[linear types]* (*"must be used __exactly once__"*, whereas Rust is seen
   as *affine*):

   ```rust
   fn foo(x: #[linear] ImportantType) {
       // ERROR! We didn't use `x` as promised.
   }
   ```

   Here, the `#[linear]` annotation is seen by Rust compilers as a call to
   a procedural macro. We can provide a simple such macro which does nothing:

   ```rust
   #[proc_macro_attribute]
   fn linear(x: TokenStream) -> TokenStream { x }
   ```

   The logic is instead provided by an external tool which analyses the function
   `foo` as a whole and which provides the actual semantics of `#[linear]`.

   [refinement types]: https://en.wikipedia.org/wiki/Refinement_type
   [LiquidHaskell]: https://ucsd-progsys.github.io/liquidhaskell-blog/

   Another example of attributes on types used by tools is given in the `refined`
   snippet in the [summary]. That example amounts to what is commonly referred
   to as *[refinement types]* such as provided by the [LiquidHaskell] tool.
   Such uses of these attributes could be particularly useful in combination
   with `unsafe`.

3. By procedural and declarative macros. As we saw in the `addition_commutes`
   example, a procedural macro is using `#[types(T, u8, u16, u32)` to multiplex
   a test for several types. In this case, `addition_commutes` could then expand
   to the following:

   ```rust
   #[proptest]
   fn addition_commutes_u8((a, b): (u8, u8)) -> bool { a + b == b + a }

   #[proptest]
   fn addition_commutes_u16((a, b): (u16, u16)) -> bool { a + b == b + a }

   #[proptest]
   fn addition_commutes_u32((a, b): (u16, u16)) -> bool { a + b == b + a }
   ```

   Another example could be to extend `#[derive(Arbitrary)]` in `proptest_derive`
   such that you could write:

   ```rust
   #[derive(Debug, Arbitrary)]
   struct Foo {
       // The macro knows how to make a Strategy for an arbitrary String.
       bar: String,
       baz: (
           // And how to make an arbitrary u8;
           u8,
           // And an usize...
           #[proptest(value = 42)] // But we always want 42 instead here.
           usize
        ),
   }
   ```

   Crates such as `serde` and `diesel` could in theory provide similar
   facilities.

Note that the examples in 2-3 are just that: examples. It is quite impossible
to know at this stage what users will utilize type-attached attributes for
since it may unleash previously held back creativity.

As an additional note, we already permit attributes on type parameters and
thus it is natural to extend attributes to arbitrary type expressions.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Syntax changes

The syntax of Rust is changed such that `#[my_attribute]` can be placed anywhere
a lifetime, type, bound, or constraint is expected. Examples of this include:

```rust
type Alpha = #[banana(split)] u8;

type Beta = #[size = "< 42"] Vec<#[orange_juice] Alpha>;

type Gamma<'a, 'b> = &#[icecream]'a mut #[tomato] &'b #[citrus(fruit)] Beta;

struct Delta
where
    #[epsilon] // This applies to the entire constraint consisting of the line below.
    Vec<#[zeta] Self>: #[eta] Ord + #[theta] Hash // #[theta] Hash is a bound.
{
    iota: #[kappa] for<'a> fn(u8) -> #[mu] (u8, #[nu] u8)
}

impl From<#[xi] u8> for #[omicron] *mut #[lambda] *const #[pi] u8 {
    ...
}

impl Tau
where
    #[cfg(foo)]
    #[rho]
    for<'a> #[sigma] Vec<Tau>: #[upsilon] 'a + #[chi] for<'b> Psi<'a, #[omega] 'b>
{
    ...
}
```

Note that this is not indicative of how actual code will be written since the
use of attributes here is much denser than in real code. The example is meant to
illustrate notable places where attributes are now permitted in a thorough way.

Additionally, attributes may be placed on associated type equality constraints
such as `Foo<#[cucumber] Assoc = Bar>` as well as associated type bounds
of form `Foo<#[avocado] Assoc: Ord>`.

### Anonymous parameters in Rust 2015

For completeness, attributes on method receivers `self`, `&self`, and `&mut self`
are permitted as well, even through neither of these constructs are types.

As Rust 2015 accepts anonymous parameters of form...

```rust
trait Foo {
    fn bar(MyType);
}
```

...we will permit attributes on `MyType` such that you may write:

```rust
trait Foo {
    fn bar(#[spinnage] MyType);
}
```

[RFC 2565]: https://github.com/rust-lang/rfcs/pull/2565

In this case, `#[spinnage] MyType` is interpreted as a parameter per [RFC 2565].

## Built-in attributes

The type-attachable attributes do not have an inherent meaning in the type system.
Instead, the meaning is what your procedural macros, the tools you use,
or what the compiler interprets certain specific attributes as.

As for the built-in attributes and their semantics,
we will, for the time being, only permit:

- Lint check attributes, that is:
  `#[allow(C)]`, `#[warn(C)]`, `#[deny(C)]`, `#[forbid(C)]`,
  and tool lint attributes such as `#[allow(clippy::foobar)]`.

- Conditional compilation attributes:

    - `#[cfg_attr(...)]`

    - `#[cfg(...)]`

      This attribute may only be placed where a list of objects are expected.
      For example:

      - In the comma separated list of constraints in a `where` clause:

        ```rust
        where
            Foo: Bar,
            #[cfg(foo)]
            Baz: Quux, // Will be removed if `foo` is not active.
        ```

      - The list of bounds in a constraint:

        ```rust
        where
            Foo:
                Ord +
                #[cfg(foo)] Hash // Will be removed if `foo` isn't active.
        ```

      - A list of types applied to a type / trait constructor:

        ```rust
        MyTrait<
            MyType<#[cfg(wibble)] u8, u16>,
            #[cfg(foo)] Assoc = Bar,
            #[cfg(baz)] Assoc: Ord + #[cfg(quux)]  Hash
        >
        ```

      However, you may for example not write `&#[cfg(foo)]'a #[cfg(bar)] Baz`
      as this could result in `&` which is semantically nonsense since the type
      is lacking. Such uses of `cfg` will be rejected with a semantic check.

      Additionally, as generic parameters are related to types in general,
      we take the opportunity to allow `#[cfg]` on generic parameter lists
      such that the parameter is removed if the condition of the `cfg`
      does not apply. This also applies to higher ranked types and bounds.

All other built-in attributes will be rejected with a semantic check.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Grammar

Let `OuterAttr` denote the production for an attribute `#[...]`.

### Lifetimes

[lykenware/gll]: https://github.com/lykenware/gll/

We change the grammar (in the [lykenware/gll] notation) of lifetimes:

```rust
Lifetime = LIFETIME; // where LIFETIME lexes a lifetime token.
```

into:

```rust
Lifetime = attrs:OuterAttr* LIFETIME;
```

The macro fragment specifier `lifetime` will permit leading `attr:OuterAttr*`.

### Types

We extend the type expression grammar with:

```rust
Type |= Attributed:{ attr:OuterAttr+ ty:Type };
```

The macro fragment specifier `ty` will permit leading `attr:OuterAttr*`.

### Constraints and bounds

Given roughly the following grammar for `where` clauses:

```rust
WhereClause = "where" constraints:WhereConstraint* % "," ","? ;
WhereConstraint =
  | Lifetime:{ lt:Lifetime ":" bounds:Lifetime* % "+" "+"? }
  | Type:{ binder:ForAllBinder? ty:Type ":" bounds:TypeBoundSet }
  | TypeEq:{ binder:ForAllBinder? left:Type { "=" | "==" } right:Type }
  ;

TypeBoundSet = bounds:TypeBound* % "+" "+"?;

TypeBound =
  | Outlives:LifetimeBound
  | Trait:TypeTraitBound
  | TraitParen:{ "(" bound:TypeTraitBound ")" }
  ;

TypeTraitBound = unbound:"?"? binder:ForAllBinder? path:Path;
```

We extend the constraint grammar with:

```rust
WhereConstraint |= Attributed:{ attr:OuterAttr+ constraint:WhereConstraint };
```

We change the grammar of `TypeTraitBound` to:

```rust
TypeTraitBound = attrs:OuterAttr* unbound:"?"? binder:ForAllBinder? path:Path;
```

Note that when an attribute is attached to a constraint,
e.g. `#[foo] 'a: 'b + 'c`, and `#[bar] Vec<u8>: 'a + Ord`,
then `#[foo]` will apply to `'a: 'b + 'c` as opposed to `'a`
and `#[bar]` will apply to `Vec<u8>: 'a + Ord` as opposed to `Vec<u8>`.

### Type and trait constructors

Given the following grammar for the contents inside the angle brackets of a path,
i.e. `< $contents? >`:

```rust
AngleBracketGenericArgsAndBindings =
  | Args:GenericArg+ % ","
  | Bindings:TypeBinding+ % ","
  | ArgsAndBindings:{ args:GenericArg+ % "," "," bindings:TypeBinding+ % "," }
  ;

GenericArg =
  | Lifetime:Lifetime
  | Type:Type
  ;

TypeBinding =
  | name:IDENT "=" ty:Type
  | name:IDENT ":" bounds:TypeBoundSet // With RFC 2289
  ;
```

we extend `TypeBinding` with:

```rust
TypeBinding |= Attributed:{ attr:OuterAttr+ binding:TypeBinding };
```

### Method receivers and `self`

Attributes are also permitted on `self` and `& $lifetime? mut? self` including
attributes on `$lifetime` as specified above. More formally, the grammar of a
method receiver specified implicitly without the type is:

```rust
ImplicitMethodReceiver =
    {rattrs:OuterAttr* "&" lf:Lifetime? "mut"?}? sattrs:OuterAttr* "self";
```

## Static semantics

Attributes on lifetimes, types, bounds, and constraints (henceforth: *"object"*)
do not have inherent meaning in the type system or elsewhere.
Semantics, if there are any, are given by the attributes themselves
on a case by case basis or by tools external to a Rust compiler.

The built-in attributes that are permitted on the objects are:

1. lint check attributes including tool lint attributes.

2. `cfg_attr(...)` unconditionally.

3. `cfg(...)` is permitted on:

   + `GenericArg`
   + `TypeBinding`
   + `WhereConstraint`
   + Each `+` separated bound in a constraint.

   This is enforced semantically when "cfg-stripping" occurs rather than
   syntactic enforcement in the grammar.

All other built-in attributes are for the time being rejected with a *semantic*
check resulting in a compilation error.

## Dynamic semantics

No changes.

# Drawbacks
[drawbacks]: #drawbacks

This proposal complicates the grammar of Rust but does so in a *predictable* way.
It is unclear whether there are any drawbacks to doing what is proposed other
than that. It may be that tweaks need to be made to certain bits and pieces of
this proposal. However, that does not negate the core idea of the proposal.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## On scope

In this RFC, the approach to scope isn't minimal / conservative in what we allow.
Instead, the goal is to be comprehensive and to make changes that are easier to
reason about than if we had done the minimal change. In particular, one goal in
this proposal is to permit conditional compilation in more places.
Therefore, we have not limited ourselves to just type expressions.
Instead, to enable conditional compilation of constraints and bounds
we permit attributes in these places, i.e:

```rust
where
    FirstTypeParameter: Ord + #[cfg(condition_a)] Debug, // conditional bounds
    #[cfg(condition_b)]
    SecondTypeParameter: Ord + 'a // A conditional constraint.
```

Furthermore, we also want to enable conditional compilation of parameters
applied to type and trait constructors so we permit attributes on lifetimes
and associated type equality constraints and bounds, i.e. `#[foo] Assoc = u8`
and `#[bar] Assoc: Display`. This entails fewer surprises and a smoother
experience but is also simpler in terms of the grammar as compared to
placing various restrictions instead.

## On precedence

There are some noteworthy design choices in this RFC with respect to precedence
and what attributes apply to. In particular, when you write constraints:

```rust
where
    #[foo]
    MyType: 'a + Ord
```

Here, `#[foo]` applies to the whole constraint `MyType: 'a + Ord` as opposed
to just applying to the type `MyType`; The reason for this is threefold:

1. Because otherwise there would be no way to say that we want the attribute
   to apply to the constraint since they cannot be wrapped in parenthesis.
   Meanwhile, we can get the other interpretation by writing:

   ```rust
   where
       (#[foo] MyType): 'a + Ord
   ```

2. As noted before, one of the main reasons for permitting attributes on
   constraints in `where` clauses in the first place is to permit `#[cfg]`
   to work on it; Meanwhile, if `#[cfg(bar)] MyType: Ord` only applied to
   `MyType` we would get `: Ord` left, which is meaningless and ill-formed.

3. It is consistent with the interpretation of:

   ```rust
   where
       for<'a> A: B
   ```

   which associates as `where for<'a> (A: B)` instead of `where (for<'a> A): B`.

# Prior art
[prior-art]: #prior-art

## Java

[annotations]: https://en.wikipedia.org/wiki/Java_annotation

Java's [annotations] are a form of syntactic metadata in the same way as
Rust's `#[attribute]`s are. For example, we may write:

```java
public class Foo extends Bar {
    @Override // An annotation; Enforces that `greet` overrides Bar's `greet`.
    public String greet() { "I am Foo" }
}
```

[java8_annotations]: https://docs.oracle.com/javase/tutorial/java/annotations/basics.html

Since Java 8, it is possible to place annotations where a type is expected.
For example, [we may write][java8_annotations]:

```java
class UnmodifiableList<T> implements
    @Readonly List<@Readonly T> { ... }

void monitorTemperature() throws
    @Critical TemperatureException { ... }
```

The purpose of these annotations is to provide *"pluggable type systems"*;
For example, you may write:

```java
@NonNull String foo = "bar";
```

## Lifetimes, bounds, and constraints

As far as is known to us, there are not many languages with
bounds / constraints (Haskell does), or lifetimes (Cyclone ~does)
which also have annotations or attributes.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

Having introduced attributes attached to lifetimes, types, bounds,
and constraints, there would still exist notable places where attributes
are not yet allowed. Chiefly among these are:

+ Expressions:

  ```rust
  do_stuff(#[foo] 1 + 2);
  ```

+ Patterns:

  ```rust
  A(#[foo] x) | #[bar] B => ...
  ```

These are the two places where effort should be directed towards to come up
with designs that solve any precedence issues that exist.

However, there are also other places where attributes could be allowed;
For example, we could allow attributes on macro arms:

```rust
macro_rules! mac {
    #[foo]
    ($x:item) => { ... };

    #[bar]
    ($x:lifetime) => { ... };
}
```

[note_nemo157]: https://github.com/rust-lang/rfcs/pull/2602#discussion_r236611620
[iliekturtles/uom#62]: https://github.com/iliekturtles/uom/pull/62

This could allow for code generation of macro arms themselves and
to introduce more macro fragment specifiers, through desugaring semantics,
without changing the language itself. As [noted by @Nemo157][note_nemo157], 
there are use cases for `#[cfg(..)]` on macro arms. An example is
[iliekturtles/uom#62] in which depending on the features active,
different types types need to have macro arms added for them.
Due to the current lack of attributes on macro arms,
significant boilerplate is added to the macro instead.

Other, more exotic, places where attributes could be allowed are for example:

+ in the middle of paths, i.e. `::#[foo] std::#[bar] cell::Cell`
+ in UFCS, i.e. `<Type as #[bar] Trait>::Thing`
+ in visibilities, i.e. `pub(#[foo] crate)`
+ on ABI specifications, i.e. `extern #[foo] "C" ...`

However, the utility of these forms are less clear.
