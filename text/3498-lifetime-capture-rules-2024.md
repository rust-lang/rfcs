- Feature Name: `lifetime_capture_rules_2024`
- Start Date: 2023-07-26
- RFC PR: [rust-lang/rfcs#3498](https://github.com/rust-lang/rfcs/pull/3498)
- Tracking Issue: [rust-lang/rust#117587](https://github.com/rust-lang/rust/issues/117587)
- Initiative: [`impl Trait` Initiative](https://github.com/rust-lang/impl-trait-initiative)

# Summary

In Rust 2024 and later editions, return position `impl Trait` (RPIT) opaque types will automatically capture all in-scope type *and* lifetime parameters.  In preparation for this, new RPIT-like `impl Trait` features introduced into earlier editions will also automatically capture all in-scope type and lifetime parameters.

# Background

Rust's rules in the 2021 and earlier editions around capturing lifetimes in return position `impl Trait` (RPIT) opaque types are inconsistent, unergonomic, and not helpful to users.  In common scenarios, doing the correct thing requires a *trick* that is not well known and whose purpose is commonly not well understood.

As we look forward to the 2024 edition and move toward stabilizing features such as type alias `impl Trait` (TAIT), associated type position `impl Trait` (ATPIT), return position `impl Trait` in trait (RPITIT), and `async fn` in trait (AFIT), we must decide on a clear vision of how lifetimes should be captured in Rust.

We want the upcoming features in the stabilization pipeline to capture lifetimes in a way that's consistent with each other and with the way we want Rust to work and develop going forward.

This RFC specifies a [solution] that achieves this.  But first, we'll describe the problem in further detail.  The descriptions and examples in this section use the semantics of Rust 2021.

## Capturing lifetimes

In return position `impl Trait` (RPIT) and `async fn`, an **opaque type** is a type that can only be used for its specified trait bounds (and for the "leaked" auto trait bounds of its hidden type).  A **hidden type** is the actual concrete type of the values hidden behind the opaque type.

A hidden type is only allowed to name lifetime parameters when those lifetime parameters have been *"captured"* by the corresponding opaque type. For example:[^ref-captures-trait-ltps]

```rust
// Returns: `impl Future<Output = ()> + Captures<&'a ()>`
async fn foo<'a>(x: &'a ()) { _ = (x,); }
```

In the above, we would say that the lifetime parameter `'a` has been captured in the returned opaque type.

For an opaque type that *does not* specify an outlives bound (e.g. `+ 'other`), when a caller receives a value of that opaque type and wants to prove that it outlives some lifetime, the caller must prove that all of the captured lifetime components of the opaque type outlive that lifetime.  The captured lifetime components are the set of lifetimes contained within captured type parameters and the lifetimes represented by captured lifetime parameters.

For an opaque type that *does* specify an outlives bound (e.g. `+ 'other`), when a caller receives a value of that opaque type and wants to prove that it outlives some lifetime, it's enough to prove that the lifetime substituted for the specified lifetime parameter in the bounds of the opaque outlives that other lifetime after transitively taking into account all known lifetime bounds.  For such an opaque type, the *callee* must prove that all lifetime and type parameters that are used in the hidden type outlive the specified bound.

See [Appendix H] for examples and further exposition of these rules.

[^ref-captures-trait-ltps]: See ["The `Captures` trick"](#the-captures-trick) for the definition of `Captures`.

## Capturing lifetimes in type parameters

In return position `impl Trait` (RPIT) and `async fn`, lifetimes contained within all in-scope type parameters are captured in the opaque type.  For example:[^ref-captures-trait-tps]

```rust
// Returns: `impl Future<Output = ()> + Captures<T>`
async fn foo<T>(x: T) { _ = (x,); }

fn bar<'a>(x: &'a ()) {
    let y = foo(x);
    //  ^^^^^^^^^^^
    //  ^ Captures 'a.
}
```

In the above, we would say that `foo` captures the type parameter `T` or that it "captures all lifetime components contained in the type parameter `T`".  Consequently, the call to `foo` captures the lifetime `'a` in its returned opaque type.

[^ref-captures-trait-tps]: See ["The `Captures` trick"](#the-captures-trick) for the definition of `Captures`.  Note that in this example, the `Captures` trick would not be needed, but it is notated explicitly for exposition.

### Behavior of `async fn`

As we saw in the examples above, `async` functions automatically capture in their returned opaque types all type and lifetime parameters in scope.

This is different than the rule for return position `impl Trait` (RPIT) in Rust 2021 and earlier editions which requires that lifetime parameters (but not type parameters) be captured by writing them in the bound.  As we'll see below, RPIT requires users to use the `Captures` trick to get the correct behavior.

The inconsistency is visible to users when desugaring from `async fn` to RPIT.  As that's something users commonly do, users have to be aware of this complexity in Rust 2021.

For example, given this `async fn`:

```rust
async fn foo<'a, T>(x: &'a (), y: T) {
    _ = (x, y);
}
```

To correctly desugar this to RPIT, we must write:

```rust
use core::future::Future;

trait Captures<U> {}
impl<T: ?Sized, U> Captures<U> for T {}

fn foo<'a, T>(x: &'a (), y: T)
-> impl Future<Output = ()> + Captures<&'a ()> {
//                            ^^^^^^^^^^^^^^^^
//                            ^ Capture of lifetime.
    async move { _ = (x, y); }
}
```

(As we'll discuss below, other seemingly simpler desugarings are incorrect.)

Given how `async fn` captures all type and lifetime parameters in scope in its returned opaque type, we could imagine that if it had happened first, the original lifetime capture rules for RPIT might have done that as well.

### Behavior of `async fn` with lifetimes in outer impl

Lifetimes in scope from an outer impl are also captured automatically by an `async fn`.  For example:

```rust
struct Foo<'a>(&'a ());
impl<'a> Foo<'a> {
    async fn foo(x: &'a ()) { _ = (x,); }
    //       ^^^^^^^^^^^^^^
    //       ^ The lifetime `'a` is automatically
    //       captured in the opaque return type.
}
```

Note that the lifetime is captured in the returned opaque type whether or not the lifetime appears in the `async fn` return type and whether or not the lifetime is actually used in the hidden type at all.

## Working with the lifetime capture rules in RPIT

For the borrow checker to function with an opaque type it must know what lifetimes it captures (and consequently what lifetimes may be used by the hidden type), so it's important that this information can be deduced from the signature, either by writing it out or by an automatic rule.

As we saw in the previous examples, for RPIT (but not for `async fn`), the rule in Rust 2021 is that opaque types automatically capture lifetimes within the type parameters but only capture lifetime parameters when those lifetime parameters are mentioned in their bounds.

When someone wants to capture a lifetime parameter not already in the bounds, that person must use one of the tricks we'll describe next.

### The outlives trick

Consider this example:

```rust
// error[E0700]: hidden type captures lifetime
//               that does not appear in bounds
fn foo<'a>(x: &'a ()) -> impl Sized { x }
```

This does not compile in Rust 2021 because the `'a` lifetime is not mentioned in the bounds of the opaque type.  We can make this work by writing:

```rust
fn foo<'a>(x: &'a ()) -> impl Sized + 'a { x }
```

This is called the "outlives trick".  But this is actually a non-solution in the general case.  Consider what `impl Sized + 'a` means.  We're returning an opaque type and promising that it outlives any lifetime `'a`.

This isn't actually what we want to promise.  We want to promise that the opaque type *captures* some lifetime `'a`, and consequently, that for the opaque type to outlive some other lifetime, `'a` must outlive that other lifetime.  If we could say in Rust that a lifetime must outlive a type, we would say that the `'a` lifetime must outlive the returned opaque type.

That is, the promise we're making is the wrong way around.

It works anyway in this specific case only because the lifetime of the returned opaque type is *exactly* equal to the lifetime `'a`.  Because equality is symmetric, the fact that our promise is the wrong way around doesn't matter.

This trick fails when there are multiple independent lifetimes that are captured, including lifetimes contained within type parameters (see [Appendix D] for an example of this).  Further, it confuses users and makes it more difficult for those users to build a consistent mental model of Rust lifetime bounds.

### The `Captures` trick

The correct way to express the capture of lifetime parameters in Rust 2021 is with the `Captures` trick.  It's the only option when multiple independent lifetimes must be captured (including lifetimes from captured type parameters).  Consider again our example:

```rust
// error[E0700]: hidden type captures lifetime
//               that does not appear in bounds
fn foo<'a>(x: &'a ()) -> impl Sized { x }
```

We could solve the problem in this way using the `Captures` trick:[^captures-trait]

```rust
trait Captures<U> {}
impl<T: ?Sized, U> Captures<U> for T {}

fn foo<'a>(x: &'a ()) -> impl Sized + Captures<&'a ()> { x }
```

Because the `'a` lifetime parameter appears in the bounds of the opaque type, Rust 2021 captures that lifetime parameter in the opaque type and accepts this code.

We can extend this trick to multiple lifetimes.  For example:

```rust
fn foo<'a, 'b>(x: &'a (), y: &'b ()) -> impl Sized + Captures<(&'a (), &'b ())> {
    (x, y)
}
```

While this does work, the `Captures` trick is ungainly, it's not widely known, and its purpose is not commonly well understood.

[^captures-trait]: Note that there are various ways to define the `Captures` trait.  In most discussions about this trick, it has been defined as above.  However, internally in the Rust compiler it is currently defined instead as `trait Captures<'a> {}`.  These notational differences do not affect the semantics described in this RFC.  Note, however, that `Captures<'a> + Captures<'b>` is not equivalent to `Captures<(&'a (), &'b ())>` because lifetimes do not participate in trait selection in Rust.  To get equivalent semantics, one would have to define `trait Captures2<'a, 'b> {}`, `trait Captures3<'a, 'b, 'c> {}`, etc.

## Behavior of RPIT in Rust 2021 with type parameters

The Rust 2021 rules for capturing lifetime parameters in opaque types are also inconsistent with the rules for capturing lifetime components within type parameters.  Consider:

```rust
fn foo<T>(x: T) -> impl Sized { x }
//                 ^^^^^^^^^^
//                 ^ Captures any lifetimes within `T`.

fn bar<'a>(x: &'a ()) -> impl Sized + 'a {
    foo(x) // Captures `'a`.
}
```

Rust captures all type parameters automatically in the opaque type.  This results in all lifetime components within those type parameters being captured automatically.  It can be surprising for lifetime parameters to not work in the same way as lifetime components contained within captured type parameters.

## Overcapturing

The rules that cause us to capture all generic parameters in the opaque type might cause us to capture too much.  This is already a problem in Rust 2021.  E.g.:

```rust
fn foo<T>(_: T) -> impl Sized {}
//                 ^^^^^^^^^^
//                 The returned opaque type captures `T`
//                 but the hidden type does not.

// error[E0515]: cannot return value referencing function parameter `x`.
fn bar(x: ()) -> impl Sized + 'static {
    foo(&x) // Captures local lifetime.
}
```

In Rust 2021, lifetimes within type parameters are automatically captured in RPIT opaque types, and both lifetime parameters and lifetimes within type parameters are automatically captured in `async fn` opaque types.  Additionally capturing lifetime parameters in RPIT opaque types may make this problem somewhat worse.

There are a number of possible solutions to this problem.  One appealing partial solution is to more fully implement the rules specified in [RFC 1214][].  This would allow type and lifetime parameters that do not outlive a specified bound to mostly act as if they were not captured.  See [Appendix G][] for a full discussion of this.

Another solution would be to add syntax for precisely specifying which type and lifetime parameters to capture.  One proposal for this syntax is described in [Appendix F][].

Type alias `impl Trait` (TAIT) is another solution.  It has accepted RFCs (see [RFC 2515][], [RFC 2071][]), it's implemented and actively maintained in nightly Rust, and there is a consensus to stabilize it in some form.  The stabilization of TAIT would allow all currently accepted code to continue to be expressed with precisely the same semantics.  See [Appendix I][] for further details on how TAIT can be used to precisely control the capturing of type and lifetime parameters.

The stabilization of the 2024 lifetime capture rules in this RFC is contingent on the stabilization of some solution for precise capturing that will allow all code that is allowed under Rust 2021 to be expressed, in some cases with syntactic changes, in Rust 2024.

[RFC 1214]: https://github.com/rust-lang/rfcs/blob/master/text/1214-projections-lifetimes-and-wf.md

## Summary of problems

In summary, in Rust 2021, the lifetime capture rules for RPIT opaque types are unergonomic and require unobvious tricks.  The rules for capturing lifetime parameters are inconsistent with the rules for capturing lifetimes within type parameters.  The rules for RPIT are inconsistent with the rules for `async fn`, and this is exposed to users because of the common need to switch between these two equivalent forms.

# Solution

[solution]: #solution

This section is normative.

In Rust 2024 and later editions, return position `impl Trait` (RPIT) opaque types will automatically capture all in-scope type *and* lifetime parameters.  In preparation for this, new RPIT-like `impl Trait` features introduced into earlier editions will also automatically capture all in-scope type and lifetime parameters.

## Apply `async fn` rule to RPIT in 2024 edition

Under this RFC, in the Rust 2024 edition, RPIT opaque types will automatically capture all lifetime parameters in scope, just as `async fn` does in Rust 2021, and just as RPIT does in Rust 2021 when capturing type parameters.

This updates and supersedes the behavior specified in [RFC 1522] and [RFC 1951].

[RFC 1522]: https://github.com/rust-lang/rfcs/blob/master/text/1522-conservative-impl-trait.md

[RFC 1951]: https://github.com/rust-lang/rfcs/blob/master/text/1951-expand-impl-trait.md

Consequently, the following examples will become legal in Rust 2024:

### Capturing lifetimes from a free function signature

```rust
fn foo<'a, T>(x: &'a T) -> impl Sized { x }
//                         ^^^^^^^^^^
//                         ^ Captures `'a` and `T`.
```

### Capturing lifetimes from outer inherent impl

```rust
struct Foo<'a, T>(&'a T);
impl<'a, T> Foo<'a, T> {
    fn foo(self) -> impl Sized { self }
    //              ^^^^^^^^^^
    //              ^ Captures `'a` and `T`.
}
```

### Capturing lifetimes from an inherent associated function signature

```rust
struct Foo<T>(T);
impl<T> Foo<T> {
    fn foo<'a>(x: &'a T) -> impl Sized { x }
    //                      ^^^^^^^^^^
    //                      ^ Captures `'a` and `T`.
}
```

### Capturing lifetimes from an inherent method signature

```rust
struct Foo<T>(T);
impl<T> Foo<T> {
    fn foo<'a>(&self, x: &'a ()) -> impl Sized { (self, x) }
    //                              ^^^^^^^^^^
    // Captures `'_`, `'a`, and `T`.^
}
```

### Capturing lifetimes from `for<..>` binders

Once higher kinded lifetime bounds on nested opaque types are supported in Rust (see [#104288][]), the following code will become legal:

```rust
trait Trait<'a> {
    type Assoc;
}

impl<'a, F: Fn(&'a ()) -> &'a ()> Trait<'a> for F {
    type Assoc = &'a ();
}

fn foo() -> impl for<'a> Trait<'a, Assoc = impl Sized> {
    //                                     ^^^^^^^^^^
    //                      Captures `'a`. ^
    fn f(x: &()) -> &() { x }
    f
}
```

That is, the `'a` lifetime parameter from the higher ranked trait bounds (HRTBs) `for<..>` binder is in scope for the `impl Sized` opaque type, so it is captured under the rules of this RFC.

Note that support for higher kinded lifetime bounds is not required by this RFC and is not a blocker to stabilizing the rules specified in this RFC.

[#104288]: https://github.com/rust-lang/rust/issues/104288

## Overcapturing

Sometimes the capture rules result in unwanted type and lifetime parameters being captured.  This happens in Rust 2021 due to the RPIT rules for capturing lifetimes from all in-scope type parameters and the `async fn` rules for capturing all in-scope type and lifetime parameters.  Under this RFC, in Rust 2024, lifetime parameters could also be overcaptured by RPIT.

The stabilization of the 2024 lifetime capture rules in this RFC is contingent on the stabilization of some solution for precise capturing that will allow all code that is allowed under Rust 2021 to be expressed, in some cases with syntactic changes, in Rust 2024.

## Type alias `impl Trait` (TAIT)

Under this RFC, the opaque type in type alias `impl Trait` (TAIT) in all editions will automatically capture all type and lifetime parameters present in the type alias.  For example:

```rust
#![feature(type_alias_impl_trait)]

type Foo<'a, T> = impl Sized;
//                ^^^^^^^^^^
//                ^ Captures `'a` and `T`.

fn foo<'a, T>() -> Foo<'a, T> {}
```

This updates and supersedes the behavior specified in [RFC 2071] and [RFC 2515].

[RFC 2071]: https://github.com/rust-lang/rfcs/blob/master/text/2071-impl-trait-existential-types.md

[RFC 2515]: https://github.com/rust-lang/rfcs/blob/master/text/2515-type_alias_impl_trait.md

## Associated type position `impl Trait` (ATPIT)

Under this RFC, the opaque type in associated type position `impl Trait` (ATPIT) in all editions will automatically capture all type and lifetime parameters present in the GAT and in the outer impl.  For example:

```rust
#![feature(impl_trait_in_assoc_type)]

trait Trait<'t> {
    type Gat<'g> where 'g: 't; // Bound required by existing GAT rules.
    fn foo<'f>(self, x: &'t (), y: &'f ()) -> Self::Gat<'f>;
}

struct Foo<'s>(&'s ());
impl<'t, 's> Trait<'t> for Foo<'s> {
    type Gat<'g> = impl Sized where 'g: 't;
    //             ^^^^^^^^^^
    //             ^ Captures:
    //
    //                 - `'g` from the GAT.
    //                 - `'f` from the method signature (via the GAT).
    //                 - `'t` from the outer impl and a trait input.
    //                 - `'s` from the outer impl and Self type.
    fn foo<'f>(self, x: &'t (), y: &'f ()) -> Self::Gat<'f> {
        (self, x, y)
    }
}
```

This updates and supersedes the behavior specified in [RFC 2071] and [RFC 2515].

## Return position `impl Trait` in Trait (RPITIT)

Under this RFC, when an associated function or method in a trait definition contains in its return type a return position `impl Trait` in trait (RPITIT), the impl of that item may capture in the returned opaque type, in all editions, all trait input type and lifetime parameters, all type and lifetime parameters present in the `Self` type, and all type and lifetime parameters in the associated function or method signature.

When such an associated function or method in a trait definition provides a default implementation, the opaque return type will automatically capture all trait input type and lifetime parameters, all type and lifetime parameters present in the `Self` type, and all type and lifetime parameters in the associated function or method signature.

In trait impls, return position `impl Trait` (RPIT), in all editions, will automatically capture all type and lifetime parameters from the outer impl and from the associated function or method signature.  This ensures that signatures are copyable from trait definitions to impls.

For example:

```rust
#![feature(return_position_impl_trait_in_trait)]

trait Trait<'t> {
    fn foo<'f>(self, x: &'t (), y: &'f ()) -> impl Sized;
    //                                        ^^^^^^^^^^
    // Method signature lifetimes, trait input lifetimes, and
    // lifetimes in the Self type may all be captured in this opaque
    // type in the impl.
}

struct Foo<'s>(&'s ());
impl<'t, 's> Trait<'t> for Foo<'s> {
    fn foo<'f>(self, x: &'t (), y: &'f ()) -> impl Sized {
        //                                    ^^^^^^^^^^
        // The opaque type captures:
        //
        //   - `'f` from the method signature.
        //   - `'t` from the outer impl and a trait input lifetime.
        //   - `'s` from the outer impl and the Self type.
        (self, x, y)
    }
}
```

This updates and supersedes the behavior specified in [RFC 3425].

[RFC 3425]: https://github.com/rust-lang/rfcs/blob/master/text/3425-return-position-impl-trait-in-traits.md

## `async fn` in trait (AFIT)

Under this RFC, when an associated function or method in a trait definition is an `async fn` in trait (AFIT), the impl of that item may capture in the returned opaque type, in all editions, all trait input type and lifetime parameters, all type and lifetime parameters present in the `Self` type, and all type and lifetime parameters in the associated function or method signature.

When such an associated function or method in a trait definition provides a default implementation, the opaque return type will automatically capture all trait input type and lifetime parameters, all type and lifetime parameters present in the `Self` type, and all type and lifetime parameters in the associated function or method signature.

In the trait impls, AFIT will automatically capture all type and lifetime parameters from the outer impl and from the associated function or method signature.  This ensures that signatures are copyable from trait definitions to impls.

This behavior of AFIT will be parsimonious with the current stable capture behavior of `async fn` in inherent impls.

For example:

```rust
#![feature(async_fn_in_trait)]

trait Trait<'t>: Sized {
    async fn foo<'f>(self, x: &'t (), y: &'f ()) -> (Self, &'t (), &'f ());
    //                                              ^^^^^^^^^^^^^^^^^^^^^^
    // Method signature lifetimes, trait input lifetimes, and
    // lifetimes in the Self type may all be captured in this opaque
    // type in the impl.
}

struct Foo<'s>(&'s ());
impl<'t, 's> Trait<'t> for Foo<'s> {
    async fn foo<'f>(self, x: &'t (), y: &'f ()) -> (Foo<'s>, &'t (), &'f ()) {
        //                                          ^^^^^^^^^^^^^^^^^^^^^^^^^
        // The opaque type captures:
        //
        //   - `'f` from the method signature.
        //   - `'t` from the outer impl and a trait input lifetime.
        //   - `'s` from the outer impl and the Self type.
        (self, x, y)
    }
}
```

This updates and supersedes the behavior specified in [RFC 3425].

# Acknowledgments

Thanks to Tyler Mandry (@tmandry) for his collaboration on the earlier design document for the 2024 lifetime capture rules, and thanks to Michael Goulet (@compiler-errors) for helpful discussions and insights on this topic.

All errors and omissions remain those of the author alone.

# Appendix A: Other resources

Other resources:

- [Lifetime capture rules 2024 T-lang design meeting](https://hackmd.io/sFaSIMJOQcuwCdnUvCxtuQ)
- [Capturing lifetimes in RPITIT](https://hackmd.io/zgairrYRSACgTeZHP1x0Zg)

# Appendix B: Matrix of capturing effects

| | 2021: *Outer LP* | 2021: *Item LP* | 2024: *Outer LP* | 2024: *Item LP* |
|-|-|-|-|-|
| RPIT          | N   | N | Y   | Y |
| `async fn`    | Y   | Y | Y   | Y |
| GATs          | Y   | Y | Y   | Y |
| TAIT          | N/A | Y | N/A | Y |
| ATPIT         | Y   | Y | Y   | Y |
| RPITIT: trait | Y   | Y | Y   | Y |
| RPITIT: impl  | Y   | Y | Y   | Y |

In the table above, "LP" refers to "lifetime parameters".

The 2024 behavior described for all items is the behavior under this RFC.

The 2021 behavior described for RPIT and `async fn` is the stable behavior in Rust 2021.  The other 2021 behaviors described are the behaviors that will be implemented for the features ahead of stabilization.

*All* of the features above automatically capture all lifetimes from all type parameters in scope in both the 2021 and the 2024 editions.

# Appendix C: The 2021 edition rules fail for RPITIT

Under the 2021 edition RPIT semantics, RPITs on inherent associated functions and methods do not capture any lifetime parameters automatically.  E.g.:

```rust
struct Foo<'a>(&'a ());
impl<'a> Foo<'a> {
    fn into_sized(self) -> impl Sized { self.0 }
    //^ Error: hidden type captures lifetime
    //         that does not appear in bounds.
}
```

If we were to apply this rule directly to RPITIT, we'd have an unworkable situation.  E.g.:

```rust
#![feature(return_position_impl_trait_in_trait)]

trait IntoSized {
    fn into_sized(self) -> impl Sized;
}

struct Foo<'a>(&'a ());
impl<'a> IntoSized for Foo<'a> {
    fn into_sized(self) -> impl Sized { self.0 }
    //^ Error: hidden type captures lifetime
    //         that does not appear in bounds.
}
```

There's nowhere that we could put `+ 'a` (or `+ Captures<&'a ()>`) in the above code to make it compile.  The trait has no way of naming `'a` at all.  It's part of the `Self` type.  The trait itself knows nothing about that.

Under the 2021 edition capture rules, our options would be to:

- Allow implicit captures of outer lifetime parameters for all RPITITs.  That would create an inconsistency between RPITIT and Rust 2021 RPIT for inherent associated functions and methods.

- Require that only the impl list the outer lifetime parameters it captures.  This would create an inconsistency between signatures in the trait definition and in the trait impl.  Even more strangely, copying the signature from a trait definition to a trait impl would result in *refinement* of the signature because the impl would be saying it does not capture the outer lifetime parameters.

- Don't allow useful impls of RPITITs on types with lifetime parameters.  This would limit the expressiveness of the language.

For RPITIT, the Rust 2021 lifetime capture rules would necessarily lead to some kind of inconsistency or loss of expressiveness.  Conversely, the rules in this RFC obviate the problem and allow RPIT to be fully consistent, whether it is used in an inherent impl, in a trait impl, or in a trait definition.

# Appendix D: The outlives trick fails with only one lifetime parameter

[Appendix D]: #appendix-d-the-outlives-trick-fails-with-only-one-lifetime-parameter

In the past, people often thought that the outlives trick was OK as long as there was only one lifetime parameter.  This is not in fact true.  Consider:

```rust
// This is a demonstration of why the Captures trick is needed even
// when there is only one lifetime parameter.

// ERROR: the parameter type `T` may not live long enough.
fn foo<'x, T>(t: T, x: &'x ()) -> impl Sized + 'x {
    //                                         ^^
    // We don't need for `T` to outlived `'x`, |
    // and we don't want to require that, so   |
    // the Captures trick must be used here. --+
    (t, x)
}

fn test<'t, 'x>(t: &'t (), x: &'x ()) {
    foo(t, x);
}
```

# Appendix E: Adding a `'static` bound

Adding a `+ 'static` bound will work in Rust 2024 in exactly the same way that it works in Rust 2021.  E.g.:

```rust
trait Captures<U> {}
impl<T: ?Sized, U> Captures<U> for T {}

fn foo<'x, T>(t: T, x: &'x ())
-> impl Sized + Captures<&'x ()> + 'static {
// ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    // In Rust 2021, this opaque type automatically captures the type
    // `T`.  Additionally, we have captured the lifetime `'x` using
    // the `Captures` trick.
    //
    // Since there is no `T: 'static` bound and no `'x: 'static`
    // bound, this opaque type would not be `'static` without the
    // specified bound on the opaque type above.  *With* that
    // specified bound, the opaque type is `'static`, and this code
    // compiles in Rust 2021.
    //
    // In Rust 2024, this opaque type will automatically capture the
    // lifetime parameter in addition to the type parameter.  The
    // `Captures` trick will not be needed in the signature.  However,
    // specifically bounding the opaque type by `'static` will still
    // work, exactly as it does in Rust 2021.
    ()
}

fn is_static<T: 'static>(_t: T) {}
fn test<'t, 'x>(t: &'t (), x: &'x ()) {
    is_static(foo(t, x));
}
```

# Appendix F: Future possibility: Precise capturing syntax

[Appendix F]: #appendix-f-future-possibility-precise-capturing-syntax

If other solutions for precise capturing of type and lifetime parameters turn out to be unergonomic or needed too often, we may want to consider adding new syntax to `impl Trait` to allow for precise capturing.  One proposal for that would look like this:

```rust
fn foo<'x, 'y, T, U>() -> impl<'x, T> Sized { todo!() }
//                        ^^^^^^^^^^^
//                        ^ Captures `'x` and `T` in the opaque type
//                        but not `'y` or `U`.
```

# Appendix G: Future possibility: Inferred precise capturing

[Appendix G]: #appendix-g-future-possibility-inferred-precise-capturing

When an outlives bound is stated for the opaque type, we can use that bound to allow code to compile that does not currently.  Consider:

```rust
fn capture<'o, T>(_: T) -> impl Send + 'o {}

// error[E0597]: `x` does not live long enough.
fn test_return<'o>(x: ()) -> impl Send + 'o {
    capture(&x)
}

// error[E0503]: cannot use `x` because it was mutably borrowed.
fn test_drop_captured(mut x: ()) {
    let _c = capture(&mut x);
    drop(x);
}

// OK.
fn test_outlives<'o>(x: ()) {
    fn outlives<'o, T: 'o>(_: T) {}
    outlives::<'o>(capture(&x));
}
```

In these examples, we're capturing a lifetime that's local to the function.  Even though Rust recognizes that the returned opaque type from `capture` outlives any other lifetime (due to the `+ 'o` bound on the opaque), the fact that the opaque type *captures* the lifetime components within `T` results in the compilation errors above.

Notably, this behavior is not specific to RPIT-like opaque types.  It can also be demonstrated using GATs:

```rust
trait PhantomCapture {
    type FakeOpaque<'o, T>: Send + 'o;
    fn capture<'o, T>(_: T) -> Self::FakeOpaque<'o, T>;
}

// error[E0597]: `x` does not live long enough
fn test_return<'o, T: PhantomCapture + 'o>(x: T) -> impl Send + 'o {
    <T as PhantomCapture>::capture(&x)
}

// error[E0505]: cannot move out of `x` because it is borrowed
fn test_drop_captured<T: PhantomCapture>(mut x: T) {
    let _c = <T as PhantomCapture>::capture(&mut x);
    drop(x);
}

// OK.
fn test_outlives<'o, T: PhantomCapture>(x: T) {
    fn outlives<'o, T: 'o>(_t: T) {}
    outlives::<'o>(<T as PhantomCapture>::capture(&x))
}
```

Future work may relax this current limitation of the compiler by more fully implementing the rules of [RFC 1214][] (see, e.g., [#116733][]).  Fixing this completely is believed to require support in the compiler for existential lifetimes (see [#60670][]).

The end result of these improvements would be that, when an outlives bound is specified for the opaque type, any type or lifetime parameters that the compiler could prove to not outlive that bound would mostly act as if it were not captured by the opaque type.

This would not be quite the same as those type and lifetime parameters not actually being captured.  By checking type equality between opaque types where different captured type or lifetime parameters have been substituted, one could tell the difference.

Still, this improvement would allow for solving many cases of overcapturing elegantly.  Consider this transformation:

```rust
fn callee<P1, .., Pn>(..) -> impl Trait { .. }
//-------------------------------------------------------------
fn callee<'o, P1: 'o, .., Pn: 'o>(..) -> impl Trait + 'o { .. }
```

Using this transformation (which is described more fully in [Appendix H][]), we can add a specified outlives bound to an RPIT opaque type without changing the effective proof requirements on either the caller or the callee.  We can then drop the `Pi: 'o` outlives bound from any type or lifetime parameter that we would like to act as if it were not captured.

This comes at the cost of adding an extra early-bound lifetime parameter in the general case.  Adding that lifetime parameter may require changing the externally visible API of the function.  However, for the common case of adding a `+ 'static` bound, or for any other case where an existing lifetime parameter suffices to specify the needed bounds, this is not a problem.

[#60670]: https://github.com/rust-lang/rust/issues/60670
[#116733]: https://github.com/rust-lang/rust/pull/116733

# Appendix H: Examples of outlives rules on opaque types

[Appendix H]: #appendix-h-examples-of-outlives-rules-on-opaque-types

There is some subtlety in understanding the rules for outlives relationships on RPIT-like `impl Trait` opaque types as [described above](#capturing-lifetimes).  In this appendix, we provide annotated examples to make these rules more clear.

### Caller proof for opaque without a specified bound

Consider:

```rust
// For an opaque type that *does not* specify an outlives bound...
fn callee<T, U>(_: T, _: U) -> impl Send {}

fn caller<'short, T: 'short, U: 'short>(x: T, y: U) {
    fn outlives<'o, T: 'o>(_: T) {}
    // ...when a caller receives a value of that opaque type...
    let z = callee(x, y);
    // ...and wants to prove that it outlives some lifetime
    // (`'short`), the caller must prove that all of the captured
    // lifetime components of the opaque type (the lifetimes within
    // `T` and `U`) outlive that lifetime (`'short`).
    //
    // The caller proves this because `T: 'short, U: 'short`.
    outlives::<'short>(z);
}
```

In this example, the caller wants to prove that the returned opaque type outlives the lifetime `'short`.  To prove this, since there is no specified outlives bound on the opaque type, it must prove that all lifetimes captured by the opaque type outlive `'short`.  To do that, it must prove that `T` and `U` outlive `'short`, since those type parameters are captured by the opaque type and may contain lifetimes.  The caller is able to prove this since `T: 'short, U: 'short`.

### Caller proof for opaque with a specified bound

Consider:

```rust
// For an opaque type that *does* specify an outlives bound...
fn callee<'o, T, U>(_: T, _: U) -> impl Send + 'o {}

fn caller<'short, 'long: 'short, T, U>(x: T, y: U) {
    fn outlives<'o, T: 'o>(_: T) {}
    // ...when a caller receives a value of that opaque type...
    let z = callee::<'long, _, _>(x, y);
    // ...and wants to prove that it outlives some lifetime
    // (`'short`), it's enough to prove that the lifetime substituted
    // (`'long`) for the specified lifetime parameter (`'o` in
    // `callee`) in the bounds of the opaque type outlives that other
    // lifetime (`'short`).
    //
    // The caller proves this because `'long: 'short`.
    outlives::<'short>(z);
}
```

In this example, the caller wants to prove that the returned opaque type outlives the lifetime `'short`.  To prove this, since there is a specified outlives bound on the opaque type (`+ 'o` in `callee`), it must prove only that the lifetime substituted for that lifetime parameter outlives `'short`.  Since `'long` is substituted for `'o`, and since `'long: 'short`, the caller is able to prove this.  Note that the caller does *not* need to prove that `T: 'short` or that `U: 'short`.

### Callee proof for opaque with a specified bound

Consider:

```rust
// For an opaque type that *does* specify an outlives bound, the
// callee must prove that all lifetime and type parameters that are
// used in the hidden type (`T` in this example) outlive the specified
// bound (`'o`).
fn callee<'o, T: 'o, U>(x: T, _: U) -> impl Sized + 'o { x }
```

In this example, the callee has specified an outlives bound on the opaque type (`+ 'o`).  For this code to be valid, the callee must prove that all lifetime and type parameters used in the returned *hidden* type (`T` in this example) outlive `'o`.  Since `T: 'o`, the callee is able to prove this.  Note that even though `U` is also captured in the opaque type, the callee does *not* need to prove `U: 'o` since it is not used in the hidden type.

### Rough equivalence between opaques with and without a specified bound

Consider these two roughly equivalent examples.

Example H.1:

```rust
fn callee<T, U>(x: T, y: U) -> impl Sized { (x, y) }
fn caller<'short, T: 'short, U: 'short>(x: T, y: U) {
    fn outlives<'o, T: 'o>(_: T) {}
    outlives::<'short>(callee(x, y));
}
```

Example H.2:

```rust
fn callee<'o, T: 'o, U: 'o>(x: T, y: U) -> impl Sized + 'o { (x, y) }
fn caller<'short, 'long: 'short, T: 'long, U: 'long>(x: T, y: U) {
    fn outlives<'o, T: 'o>(_: T) {}
    outlives::<'short>(callee::<'long, _, _>(x, y));
}
```

In the first example, to prove that the opaque type outlives `'short`, the *caller* has to prove that each of the captured lifetime components outlives `'short`.  In the second example, to prove that same thing, it only needs to prove that `'long: 'short`.

(Obviously, the caller then still needs to prove the outlives relationships necessary to satisfy the other specified bounds in the signature of `callee`.)

That is, at the cost of an extra early-bound lifetime parameter in the signature of the callee, we can always express an RPIT without a specified outlives bound as an RPIT with a specified outlives bound in a way that does not change the requirements on the caller or the callee.  We do this by applying the following transformation:

```rust
fn callee<P1, .., Pn>(..) -> impl Trait { .. }
//-------------------------------------------------------------
fn callee<'o, P1: 'o, .., Pn: 'o>(..) -> impl Trait + 'o { .. }
```

One application of this transformation to solve problems created by overcapturing is described in [Appendix G][].

# Appendix I: Precise capturing with TAIT

[Appendix I]: #appendix-i-precise-capturing-with-tait

Sometimes the capture rules result in unwanted type and lifetime parameters being captured.  This happens in Rust 2021 due to the RPIT rules for capturing lifetimes from all in-scope type parameters and the `async fn` rules for capturing all in-scope type and lifetime parameters.  Under this RFC, in Rust 2024, lifetime parameters could also be overcaptured by RPIT.

Type alias `impl Trait` (TAIT) provides a precise solution.  It works as follows.  Consider this overcaptures scenario in Rust 2024:

```rust
fn foo<'a, T>(_: &'a (), _: T) -> impl Sized { () }
//                                ^^^^^^^^^^
// The returned opaque type captures `'a` and `T`
// but the hidden type does not use either.

fn bar<'a, 'b>(x: &'a (), y: &'b ()) {
    fn is_static<T: 'static>(_: T) {}
    is_static(foo(x, y));
    //        ^^^^^^^^^
    // Error: `foo` captures `'a` and `'b`.
}
```

In the above code, we want to rely on the fact that `foo` does not actually use any lifetimes in the returned hidden type.  We can't do that using RPIT because there's no way to prevent the opaque type from capturing too much.  However, we can use TAIT to solve this problem elegantly as follows:

```rust
#![feature(type_alias_impl_trait)]

type FooRet = impl Sized;
fn foo<'a, T>(_: &'a (), _: T) -> FooRet { () }
//                                ^^^^^^
// The returned opaque type does NOT capture `'a` or `T`.

fn bar<'a, 'b>(x: &'a (), y: &'b ()) {
    fn is_static<T: 'static>(_: T) {}
    is_static(foo(x, y)); // OK.
}
```

The type alias `FooRet` has no generic parameters, so none are captured in the opaque type.  It's always possible to desugar an RPIT opaque type into a TAIT opaque type that expresses precisely which generic parameters to capture.

The stabilization of the 2024 lifetime capture rules in this RFC is contingent on the stabilization of some solution for precise capturing that will allow all code that is allowed under Rust 2021 to be expressed, in some cases with syntactic changes, in Rust 2024.
