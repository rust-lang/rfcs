- Start Date: 2014-03-16
- RFC PR #:
- Rust Issue #:

# Summary

Introduce the `abstract` keyword. The definitions of normal `struct`s and
`enum`s are public, the built-in traits are inferred for them, and variance is
inferred for their type parameters. The definitions of `abstract struct`s and
`abstract enum`s are private, `impl`s of the built-in traits must be declared or
derived for them explicitly, and their type parameters default to invariant if
not annotated otherwise.

This is an alternative to the proposal by @alexcrichton (#1).

# Motivation

There are ~~three~~ ~~four~~ five broad goals this design aims to achieve:

 * Correctness

 * Clarity

 * Consistency

 * Simplicity

 * Convenience

Correctness means that abstraction boundaries should be given the highest
respect, and nothing about the private implementation of a module should be
exposed to the outside world without the author's intent. Abstraction boundaries
(a.k.a. implementation hiding) are frequently employed to maintain invariants by
only exposing a "safe" interface. Sometimes, the invariant is memory safety.

Clarity: Things that are different should look different. Things that look
similar should be similar.

Consistency: The design should accomodate structural structs, tuple structs, and
enums as well with equal aplomb.

Simplicity: The system should be easy to specify, understand, and implement.

Convenience: The programmer should not be burdened with too many required
annotations.


# Detailed design

### Specification

Remove `pub` and `priv` as modifiers on struct fields and enum variants. Remove
`priv` as a keyword. Introduce the `abstract` keyword as a modifier on `struct`
and `enum` declarations.

Normal `struct`s and `enum`s without the `abstract` modifier may be constructed
(using literals), pattern matched (have their constructors used in patterns),
and have their fields accessed in any module. As currently, `impl`s of the
built-in traits (`Pod`/`Copy`, `Send`, `Freeze`, `Share`, etc. - whichever ones
we end up having) and variances for their type and lifetime parameters are
inferred automatically based on their contents.

Abstract `struct` and `enum` types, declared with the `abstract` modifier, may
be constructed (using literals), pattern matched (have their constructors used
in patterns), and have their fields accessed only inside the module which owns
them. `impl`s of the built-in traits must be declared or derived for these types
explicitly, and the legality of these declarations is checked by the compiler.
Variances for their type and lifetime parameters default to invariant unless the
author annotates a more permissive variance (covariance, contravariance, or
bivariance), which is likewise checked by the compiler. (In other words, the
interface exposed with respect to the built-in traits and variances may be more
restrictive than what the compiler would infer based on the contents of the
type, but not more permissive.)

### Elaboration

Here I'll just go through the goals mentioned in the Motivation section and
explain how each of them are satisfied by this design.

##### Correctness

In current Rust, the types of private fields are taken into consideration when
inferring the built-in traits and variances, and the results of this are
reflected in the public interface. This subverts abstraction boundaries, and
causes types like `Cell` to be "unsafe by default". Here we straightforwardly
avoid this problem by not doing that. No assumptions are made about the
properties of abstract datatypes, and the programmer must explicitly declare
what she will commit to supporting in the public interface of her module.

For types with public definitions, by contrast, inferring their properties based
on their contents is completely fine. This is because making the definition of
the type part of the public interface is a stronger contract than declaring any
particular built-in trait or variance: information about the latter is logically
subsumed by the former. It also wouldn't make sense to, e.g., make a public
`struct` which you neglect to make `Copy`-able, because the restriction can be
worked around with just a bit of extra fingerwork:

    // (non-Copy)
    pub struct Foo { a: int, b: int }

    let foo = Foo { a: -1, b: 1 };

    let also_foo = foo; // illegal

    let foo_as_well = Foo { a: foo.a, b: foo.b }; // legal!


##### Clarity

In most languages, abstract datatypes are expressed by making fields private
(for example: current Rust), or neglecting to export constructors (Haskell).
Here we are more direct about it: abstract datatypes are declared with the
`abstract` keyword. Abstract and non-abstract types behave differently under
this proposal beyond the fact of their fields being public or private, and it
might be surprising if this different behavior were a function of the mere
presence or absence of a private field. Therefore they have different names:
`struct` and `abstract struct`, and `enum` and `abstract enum` respectively.

##### Consistency

The `abstract` keyword has the same effect regardless of whether the type is a
`struct` or an `enum`, or how it is defined. (Observe that we did not "branch"
on the definition of the type anywhere in the Specification section above.) A
type is either fully public or fully abstract, and this applies equally well to
structural structs, tuple structs, unit structs(!), and enums with any kind of
variants. (Contrast to the current language, where privacy defaults and
modifiers are defined separately, on a seemingly ad hoc basis, for each of
these.)

##### Simplicity

An attractive feature of this design is that visibility modifiers only exist at
the item level. Current Rust as well as Haskell provide more power with respect
to visibility control than is actually useful. There's no use case whatsoever
that I'm aware of for an `enum` with a mixture of public and private variants (I
asked around in #haskell, and neither do they). There's also no reason that I
know of to allow or require re-exporting the variants or fields of a type
separately from the type itself. Furthermore, the vast majority of types, as can
be seen in the statistics collected by @alexcrichton (thanks!), have either
all-public or all-private fields. Here those are exactly the options available.

##### Convenience

A type which is "just a bunch of data" should ideally not be burdened with
additional baggage in the form of annotations to declare its properties and/or
the visibility of its fields. The classic example is:

    struct Point { x: int, y: int }

Under this proposal, you can write exactly that, and it will behave as you might
expect. The same applies for many other common types, for example `Option`.

Instead of having to individually declare each field or variant private (if the
default is public) to declare an abstract type, or each of them public (if the
default is private) to declare a non-abstract one, here you control the
visibility for the defition of the whole type at once with the `abstract`
keyword.

Where in current Rust you would write:

    pub struct Handle {
        priv desc: &'static Desc,
        priv data: ~Data,
        priv name: ~str
    }

Here you would write:

    pub abstract struct Handle {
        desc: &'static Desc,
        data: ~Data,
        name: ~str
    }

Where under competing proposals, you might write:

    #[deriving(Data)]
    pub struct Pair<A, B>(pub A, pub B);
    // (variance annotations omitted!)

Here you may simply write:

    pub struct Pair<A, B>(A, B);

Another interesting case is unit structs. In current Rust, these by necessity
have their definitions public, and if you want to declare an empty type which
may not be constructed outside your module, you need to declare a dummy private
field:

    pub struct Token { priv dummy: () }

Here you can simply write:

    pub abstract struct Token;

for the same effect.

Under some designs, only `struct`s, but not `enum`s may have private
definitions. Therefore if you want an abstract type whose implementation is an
`enum`, you need to wrap it in a `struct`. Here there is no such need.


# Alternatives

### Competing designs

The two main alternatives are the system we currently have, and the proposal by
@alexcrichton. I'm assuming that the latter would incorporate @nikomatsakis's
proposal for explicitly declaring the built-in traits. I'm also assuming that no
one needs to be persuaded about the problems with the system we currently have.
While @alexcrichton's proposal would already be a significant improvement over
the status quo (especially, when combined with @nikomatsakis's, with respect to
correctness), I believe this proposal would represent a further improvement with
respect to consistency (enums and unit structs are also brought into the fold),
simplicity (I do not believe finer-grained interior visibility control carries
its weight), and convenience (not needing to write `pub` N times for each field
+ not having to declare built-in traits and variances for types with public
definitions).

### Variations on this design

##### Bikeshedding the `abstract` keyword

The word `abstract` has a different meaning in some other languages, where it's
closer to our `trait`, but it's also commonly used conversationally in the same
sense as in this proposal. I haven't, even with the assistance of thesaurus.com,
been able to find a better word.

One alternative that comes up is`class`, but that carries  *more* unwanted
baggage from OOP and OOP-like languages than does `abstract`, and it also
doesn't gel with `enum`s.

##### Flip the defaults

Rather than have an `abstract` keyword, make it the default, and find another
keyword in its stead to mark types whose definitions should be public. I
personally like being able to write `struct Point { x: int, y: int}` and prefer
it to the alternative, but I would also be open to flipping it around (on the
presumable grounds that this is more consistent with everything else being
private-by-default). The biggest obstacle here is that I'm even less able to
think of any suitable word.

##### Add back `pub` fields in `abstract struct`s

If having individual `pub` fields in otherwise `abstract struct`s is deemed
valuable, it's fairly simple to add them back. Just allow putting `pub` on its
fields, and then you may access those fields with dot syntax, and potentially
also in pattern matches and functional-style struct updates. An important point
here is that even if *all* of the fields of an `abstract struct` are marked
`pub`, the rest of the program should be able to see the public fields, but
*not* whether or not there are any others, i.e. should still not be able to
construct instances of the type or pattern match on it without using a `..`
wildcard.

# Unresolved questions

These questions are tangential to the proposal under discussion, but still
important:

 * *What should the syntax for declaring variances be?*

   What I know: these should be modifiers on type and lifetime parameters, and
there should be two of them: one for "may be used covariantly", another for "may
be used contravariantly", with bivariance being expressible as their
combination.

   What I don't know: what they should be called.

 * *Should enum variants be brought into scope together with the enum itself, or
should they have to be imported individually?*

   The latter has the advantage that it's more obvious which variant is coming
from where.

   The former has the advantage that it's more convenient and makes more sense:
an enum and its variants are logically a unit, and there's no reason you should
want to separate them (any more than a `struct` from its fields).

   It should also not be possible to re-export enums and variants separately
with `pub use`, which also supports the first option.

Apart from these, and things already mentioned under "Variations", there's
nothing else that I'm aware of.
