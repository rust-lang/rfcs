- Feature Name: `param_attrs`
- Start Date: 2018-10-14
- RFC PR: [rust-lang/rfcs#2565](https://github.com/rust-lang/rfcs/pull/2565)
- Rust Issue: [rust-lang/rust#60406](https://github.com/rust-lang/rust/issues/60406)

## Summary
[summary]: #summary

Allow attributes in formal function parameter position.
For example, consider a Jax-Rs-style HTTP API:

```rust
#[resource(path = "/foo/bar")]
impl MyResource {
    #[get("/person/:name")]
    fn get_person(
        &self,
        #[path_param = "name"] name: String, // <-- formal function parameter.
        #[query_param = "limit"] limit: Option<u32>, // <-- here too.
    ) {
        ...
    }
}
```

## Motivation
[motivation]: #motivation

Allowing attributes on formal function parameters enables external tools and
compiler internals to take advantage of the additional information that the
attributes provide.

Conditional compilation with `#[cfg(..)]` is also
facilitated by allowing more ergonomic addition and removal of parameters.

Moreover, procedural macro authors can use annotations on
these parameters and thereby richer DSLs may be encoded by users.
We already saw an example of such a DSL in the [summary].
To further illustrate potential usages, let's go through a few examples.

### Compiler internals: Improving `#[rustc_args_required_const]`

[memory_grow]: https://doc.rust-lang.org/nightly/core/arch/wasm32/fn.memory_grow.html

A number of platform intrinsics are currently provided by rust compilers.
For example, we have [`core::arch::wasm32::memory_grow`][memory_grow] which,
for soundness reasons, requires that when `memory_grow` is applied,
`mem` must provided a `const` expression:

```rust
#[rustc_args_required_const(0)]
pub fn memory_grow(mem: u32, delta: usize) -> usize { .. }
```

This is specified in a positional manner, referring to `mem` by `0`.
While this is serviceable, this RFC enables us encode the invariant more directly:

```rust
pub fn memory_grow(
    #[rustc_args_required_const] mem: u32,
    delta: usize
) -> usize {
    ..
}
```

### Property based testing of polymorphic functions

[QuickCheck]: https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf
[proptest]: https://github.com/altsysrq/proptest
[quickcheck]: https://github.com/BurntSushi/quickcheck

Property based testing a la [QuickCheck] allows users to state properties they
expect their programs to adhere to. These properties are then tested by
randomly generating input data and running the properties with those.
The properties are can then be falsified by finding counter-examples.
If no such example are found, the test passes and the property is "verified".
In the Rust ecosystem, property based testing is primarily provided by the
[proptest] and [quickcheck] crates where the former uses integrated shrinking
whereas the latter uses type based shrinking.

Consider a case where we want to test a "polymorphic" function on a number
of concrete types.

```rust
#[proptest] // N.B. Using proptest doesn't look like this today.
fn prop_my_property(#[types(T = u8, u16, u32)] elem: Vec<T>, ..) { .. }
```

Here, we've overloaded the test for the types `u8`, `u16`, and `u32`.
The test will then act as if you had written:

```rust
#[proptest]
fn prop_my_property_u8(elem: Vec<u8>, ..) { .. }

#[proptest]
fn prop_my_property_u16(elem: Vec<u16>, ..) { .. }

#[proptest]
fn prop_my_property_u32(elem: Vec<u32>, ..) { .. }
```

By allowing attributes on function parameters, the test can be specified
more succinctly and without repetition as done in the first example.

### FFI and interoperation with other languages

[wasm_bindgen]: https://github.com/rustwasm/wasm-bindgen

There's interest in using attributes on function parameters for
[`#[wasm_bindgen]`][wasm_bindgen]. For example, to interoperate well
with TypeScript's type system, you could write:

```rust
#[wasm_bindgen]
impl RustLayoutEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Default::default() }

    #[wasm_bindgen(typescript(return_type = "MapNode[]"))]
    pub fn layout(
        &self, 
        #[wasm_bindgen(typescript(type = "MapNode[]"))]
        nodes: Vec<JsValue>, 
        #[wasm_bindgen(typescript(type = "MapEdge[]"))]
        edges: Vec<JsValue>
    ) -> Vec<JsValue> {
        ..
    }
}
```

Currently, in `#[wasm_bindgen]`, the arguments and return type of `layout`
are all `any[]`. By using allowing the annotations above, tighter types can
be used which can help in catching problems at compile time rather than
having UI bugs later.

### Greater control over optimizations in low-level code

For raw pointers that are oftentimes used when operating with C code,
additional information could be given to the compiler about the set of parameters.
You could for example mirror C's restrict keyword or even be more explicit by
stating which pointer arguments may overlap:

```rust
fn foo(
 #[overlaps_with(in_b)] in_a: *const u8,
 #[overlaps_with(in_a)] in_b: *const u8,
 #[restrict] out: *mut u8
);
```

This would tell the compiler or some static analysis tool that the pointers
`in_a` and `in_b` might overlap but `out` is non overlapping. Note that neither 
`overlaps_with` and `restrict` are part of this proposal; rather, they are
examples of what this RFC facilities.

### Handling of unused parameter

In today's Rust it is possible to prefix the name of an identifier to silence
the compiler about it being unused. With attributes on formal parameters,
we could hypothetically have an attribute like `#[unused]` that explicitly
states this for a given parameter. Note that `#[unused]` is not part of this
proposal but merely a simple use-case. In other words, we could write (1):

```rust
fn foo(#[unused] bar: u32) -> bool { .. }
```

instead of (2):

```rust
fn foo(_bar: u32) -> bool { .. }
```

Especially Rust beginners might find the meaning of (1) to be clearer than (2).

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Formal parameters of `fn` definitions as well closures parameters may have
attributes attached to them. Thereby, additional information may be provided.

For the purposes of illustration, let's assume we have the attributes
`#[orange]` and `#[lemon]` available to us.

### Basic examples

The syntax for attaching attributes to parameters is shown in the snippet below:

```rust
// Free functions:
fn foo(#[orange] bar: u32) { .. }

impl Alpha { // In inherent implementations.
    // - `self` can also be attributed:
    fn bar(#[lemon] self, #[orange] x: u8) { .. }
    fn baz(#[lemon] &self, #[orange] x: u8) { .. }
    fn quux(#[lemon] &mut self, #[orange] x: u8) { .. }

    ..
}

impl Beta for Alpha { // Also works in trait implementations.
    fn bar(#[lemon] self, #[orange] x: u8) { .. }
    fn baz(#[lemon] &self, #[orange] x: u8) { .. }
    fn quux(#[lemon] &mut self, #[orange] x: u8) { .. }

    ..
}

fn foo() {
    // Closures:
    let bar = |#[orange] x| { .. };
    let baz = |#[lemon] x: u8, #[orange] y| { .. };
}
```

### Trait definitions

An `fn` definition doesn't need to have a body to permit parameter attributes.
Thus, in `trait` definitions, we may write:

```rust
trait Beta {
    fn bar(#[lemon] self, #[orange] x: u8);
    fn baz(#[lemon] &self, #[orange] x: u8);
    fn quux(#[lemon] &mut self, #[orange] x: u8);
}
```

In Rust 2015, since anonymous parameters are allowed, you may also write:

```rust
trait Beta {
    fn bar(#[lemon] self, #[orange] u8); // <-- Note the absence of `x`!
}
```

### `fn` types

You can also use attributes in function pointer types.
For example, you may write:

```rust
type Foo = fn(#[orange] x: u8);
type Bar = fn(#[orange] String, #[lemon] y: String);
```

### Built-in attributes

Attributes attached to formal parameters do not have an inherent meaning in
the type system or in the language. Instead, the meaning is what your
procedural macros, the tools you use, or what the compiler interprets certain
specific attributes as.

As for the built-in attributes and their semantics, we will, for the time being,
only permit the following attributes on parameters:

- Lint check attributes, that is:
  `#[allow(C)]`, `#[warn(C)]`, `#[deny(C)]`, `#[forbid(C)]`,
  and tool lint attributes such as `#[allow(clippy::foobar)]`.

- Conditional compilation attributes:

    - `#[cfg_attr(...)]`, e.g.

      ```rust
      fn foo(#[cfg_attr(bar, orange)] x: u8) { .. }
      ```

      If `bar` is active, this is equivalent to:

      ```rust
      fn foo(#[orange] x: u8) { .. }
      ```

      And otherwise equivalent to:

      ```rust
      fn foo(x: u8) { .. }
      ```

    - `#[cfg(...)]`, e.g.

      ```rust
      fn foo(#[cfg(bar)] x: u8, y: u16) { .. }
      ```

      If `bar` is active, this is equivalent to:

      ```rust
      fn foo(x: u8, y: u16) { .. }
      ```

      And otherwise equivalent to:

      ```rust
      fn foo(y: u16) { .. }
      ```

All other built-in attributes will be rejected with a semantic check.
For example, you may not write:

```rust
fn foo(#[inline] bar: u32) { .. }
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Grammar

Let `OuterAttr` denote the production for an attribute `#[...]`.

On the formal parameters of an `fn` item, including on method receivers,
and irrespective of whether the `fn` has a body or not, `OuterAttr+` is allowed
but not required. For example, all the following are valid:

```rust
fn g1(#[attr1] #[attr2] pat: Type) { .. }

fn g2(#[attr1] x: u8) { .. }

fn g3(#[attr] self) { .. }

fn g4(#[attr] &self) { .. }

fn g5<'a>(#[attr] &mut self) { .. }

fn g6<'a>(#[attr] &'a self) { .. }

fn g7<'a>(#[attr] &'a mut self) { .. }

fn g8(#[attr] self: Self) { .. }

fn g9(#[attr] self: Rc<Self>) { .. }
```

The attributes here apply to the parameter *as a whole*,
e.g. in `g2`, `#[attr]` applies to `pat: Type` as opposed to `pat`.

More generally, an `fn` item contains a list of formal parameters separated or
terminated by `,` and delimited by `(` and `)`. Each parameter in that list may
optionally be prefixed by `OuterAttr+`.

#### Variadics

Attributes may also be attached to `...` on variadic functions, e.g.

```rust
extern "C" {
    fn foo(x: u8, #[attr] ...);
}
```

That is, for the purposes of this RFC, `...` is considered as a parameter.

#### Anonymous parameters in Rust 2015

In Rust 2015 edition, as `fn`s may have anonymous parameters, e.g.

```rust
trait Foo { fn bar(u8); }
```

attributes are allowed on those, e.g.

```rust
trait Foo { fn bar(#[attr] u8); }
```

#### `fn` pointers

[lykenware/gll]: https://github.com/lykenware/gll/

Assuming roughly the following type grammar for function pointers
(in the [lykenware/gll] notation):

```rust
Type =
  | ..
  | FnPtr:{
      binder:ForAllBinder? unsafety:"unsafe"? { "extern" abi:Abi }?
      "fn" "(" inputs:FnSigInputs? ","? ")" { "->" ret_ty:Type }?
    }
  ;

FnSigInputs =
  | Regular:FnSigInput+ % ","
  | Variadic:VaradicTail
  | RegularAndVariadic:{ inputs:FnSigInput+ % "," "," "..." }
  ;

VaradicTail = "...";
FnSigInput = { pat:Pat ":" }? ty:Type;
```

we change `VaradicTail` to:

```rust
VaradicTail = OuterAttr* "...";
```

and change `FnSigInput` to:

```rust
FnSigInput = OuterAttr* { pat:Pat ":" }? ty:Type;
```

Similar to parameters in `fn` items, the attributes here also apply to the
pattern and the type if both are present, i.e. `pat: ty` as opposed to `pat`.

#### Closures

Given roughly the following expression grammar for closures:

```rust
Expr = attrs:OuterAttr* kind:ExprKind;
ExprKind =
  | ..
  | Closure:{
      by_val:"move"?
      "|" args:ClosureArg* % "," ","? "|" { "->" ret_ty:Type }? body:Expr
    }
  ;

ClosureArg = pat:Pat { ":" ty:Type }?;
```

we change `ClosureArg` into:

```rust
ClosureArg = OuterAttr* pat:Pat { ":" ty:Type }?;
```

As before, when the type is specified, `OuterAttr*` applies to `pat: Type`
as opposed to just `pat`.

### Static semantics

Attributes on formal parameters of functions, closures and function pointers
have no inherent meaning in the type system or elsewhere. Semantics, if there
are any, are given by the attributes themselves on a case by case basis or by
tools external to a Rust compiler.

#### Built-in attributes

The built-in attributes that are permitted on the parameters are:

1. lint check attributes including tool lint attributes.

2. `cfg_attr(..)` unconditionally.

3. `cfg(..)` unconditionally.

   When a `cfg(..)` is active, the formal parameter will be included
   whereas if it is inactive, the formal parameter will be excluded.

All other built-in attributes are for the time being rejected with a *semantic*
check resulting in a compilation error.

#### Macro attributes

Finally, a registered `#[proc_macro_attribute]` may not be attached directly
to a formal parameter. For example, if given:

```rust
#[proc_macro_attribute]
pub fn attr(args: TokenStream, input: TokenStream) -> TokenStream { .. }
```

then it is not legal to write:

```rust
fn foo(#[attr] x: u8) { .. }
```

### Dynamic semantics

No changes.

## Drawbacks
[drawbacks]: #drawbacks

All drawbacks for attributes in any location also count for this proposal.

Having attributes in many different places of the language complicates its grammar.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why is this proposal considered the best in the space of available ideas?

This proposal goes the path of having attributes in more places of the language.
It nicely plays together with the advance of procedural macros and macros 2.0
where users can define their own attributes for their special purposes.

### Alternatives

An alternative to having attributes for formal parameters might be to just use
the current set of available attributable items to store meta information about
formal parameters like in the following example:

```rust
#[ignore(param = bar)]
fn foo(bar: bool);
```

An example of this is `#[rustc_args_required_const]` as discussed
in the [motivation].

Note that this does not work in all situations (for example closures) and might
involve even more complexity in user's code than simply allowing attributes on
formal function parameters.

### Impact

The impact will be that users might create custom attributes when designing
procedural macros involving formal function parameters.

There should be no breakage of existing code.

### Variadics and `fn` pointers

In this proposal it is legal to write `#[attr] ...` as well as `fn(#[attr] u8)`.
The primary justification for doing so is that conditional compilation with
`#[cfg(..)]` is facilitated. Moreover, since the `fn` type grammar and
that of `fn` items is somewhat shared, and since `...` is the tail of a
list, allowing attributes there makes for a simpler grammar.

## Prior art
[prior-art]: #prior-art

Some example languages that allow for attributes on formal function parameter
positions are Java, C#, and C++.

Also note that attributes in other parts of the Rust language could be
considered prior art to this proposal.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

None as of yet.

## Future possibilities

### Attributes in more places

[RFC 2602]: https://github.com/rust-lang/rfcs/pull/2602

In the pursuit of allowing more flexible DSLs and more ergonomic conditional
compilation, [RFC 2602] builds upon this RFC.

### Documentation comments

In this RFC, we have not allowed documentation comments on parameters.
For example, you may not write:

```rust
fn foo(
    /// Some description about `bar`.
    bar: u32
) {
    ..
}
```

Neither may you write the desugared form:

```rust
fn foo(
    #[doc = "Some description about `bar`."]
    bar: u32
) {
    ..
}
```

In the future, we may want to consider supporting this form of documentation.
This will require support in `rustdoc` to actually display the information.

### `#[proc_macro_attribute]`

In this RFC we stated that `fn foo(#[attr] x: u8) { .. }`,
where `#[attr]` is a `#[proc_macro_attribute]` is not allowed.
In the future, if use cases arise to justify a change,  we could lift this
restriction such that transformations can be done directly on `x: u8`.
