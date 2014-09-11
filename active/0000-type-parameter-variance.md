- Start Date: 2014-09-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

- Use inference to determine the *variance* of type parameters.
- Add a lint (defaulting to `deny`) for unconstrained type/lifetime
  parameters.
- Continue to use markers to allow explicit indication of variance.

# Motivation

## Why variance is good

Today, all type parameters are invariant. This can be problematic
around lifetimes. A particular common example of where problems
arise is in the use of `Option`. Here is a simple example. Consider
this program, which has a struct containing two references:

```
struct List<'l> {
    field1: &'l int,
    field2: &'l int,
}

fn foo(field1: &int, field2: &int) {
    let list = List { field1: field1, field2: field2 };
    ...
}

fn main() { }
```

Here the function `foo` takes two references with distinct lifetimes.
The variable `list` winds up being instantiated with a lifetime that
is the intersection of the two (presumably, the body of `foo`).  This
is good.

If we modify this program so that one of those references is optional,
however, we will find that it gets a compilation error:

```
struct List<'l> {
    field1: &'l int,
    field2: Option<&'l int>,
}

fn foo(field1: &int, field2: Option<&int>) {
    let list = List { field1: field1, field2: field2 };
        // ERROR: Cannot infer an appropriate lifetime
    ...
}

fn main() { }
```

The reason for this is that because `Option` is *invariant* with
respect to its argument type, it means that the lifetimes of `field1`
and `field2` must match *exactly*. It is not good enough for them to
have a common subset. This is not good.

## Why variance should be inferred

Actually, lifetime parameters already have a notion of variance, and
this varinace is fully inferred. In fact, the proper variance for type
parameters is *also* being inferred, we're just largely ignoring
it. (It's not completely ignored; it informs the variance of
lifetimes.)

The main reason we chose inference over declarations is that variance
is kind of tricky business and it's annoying to have to think about
it, since it's a purely mechanical thing. The main reason that it pops
up from time to time in Rust today (specifically, in examples like the
one above) is because we *ignore* the results of inference.

## Why variance could be dangerous if we don't add a lint

The current variance inference assumes that the type definitions it
sees are accurate. Sometimes this is not true. In particular, many
types have "phantom" type or lifetime parameters that are not used
in the body of the type. This generally occurs with unsafe code:

    struct Items<'vec, T> { // unused lifetime parameter 'vec
        x: *mut T
    }
    
    struct AtomicPtr<T> { // unused type parameter T
        data: AtomicUint  // represents an atomically mutable *mut T, really
    }
    
Since these parameters are unused, the inference currently obtains
a result of "bivariant". This basically means "completely ignore this
parameter for the purpose of subtyping". If we were to use such a result,
it would mean that, e.g., `AtomicPtr<int>` and `AtomicPtr<uint>` would
be interchangable. Not good. (In fact, this is already the case for
lifetimes.)

To avoid this hazard, the RFC proposes a lint, initially set to deny,
for type or lifetime parameters that are inferred to be bivariant (in
other words, which are (transitively) unused). Almost always, the
correct thing to do in such a case is to either remove the parameter
in question or insert one of the existing *marker types*. The marker
types basically inform the inference engine to pretend as if the type
parameter were used in particular ways. (Almost always, the right
thing is to use a `CovariantType` marker for type parameters; the only
exception would be a case like `AtomicPtr`, where the type parameter
is conceptually used inside of a `Cell` or other interior mutable
location.)

I chose to set the lint to deny rather than warn because there is
practically no known use case for bivariance. Hence this is almost
always an error, and one with potentially serious consequences, since
it almost always occurs in the context of unsafe code.

# Detailed design

In bullet points:

- Use variance results to inform subtyping of nominal types
  (structs, enums).
- Use variance for the output type parameters on traits.
- Input type parameters of traits are considered invariant.
- Variance has no effect on the type parameters on an impl or fn;
  rather those are freshly instantiated at each use.
- Add a lint that is triggered whenever a bivariant inference result
  occurs. Set the lint to deny.

These changes have been implemented. You can view the results, and the
impact on the standard library, in
[this branch on nikomatsakis's repository][b].

## Input type parameters for traits

One slightly non-obvious thing is why I chose to make input type
parameters for traits always invariant. The reason was because
it frequently happens that input type parameters do not appear
in the trait inferface at all, most obviously for marker traits:

    trait Send { } // clearly, Self is not constrained here!

Also, even when the `Self` type parameter (or other inputs) do appear,
they are frequently in both `&self` and `&mut self` contexts, in which
case the parameter will always be invariant anyway.

Finally, making input type parameters invariant does not actually
prevent many "covariant"-ish use cases from working, due to the
details of how method resolution works. In particular, we currently
take the "self" value (which might, e.g., have type `&T`), and then
try to "assign" it to the declared self (which might, e.g., have type
`&U`).  This assignment will succeed if `&T <: &U` and hence if `T <:
U`.

## Error message details

In the case of an unused type (resp. lifetime) parameter, the error
message explicitly suggests the use of a `CovariantType`
(resp. `ContravariantLifetime`) marker:

    type parameter `T` is never used; either remove it, or use a
    marker such as `std::kinds::marker::InvariantType`"
    
The goal is to help users as concretely as possible. The documentation
on the `InvariantType` marker type should also be helpful in guiding
users to make the right choice (the ability to easily attach
documentation to the marker type was in fact the major factor that led
us to adopt marker types in the first place). I chose to suggest
invariance because it is the safest choice.

One problem with the current implementation is that, because
bivariance warnings are issued as lints, bivariant type parameters
often manifest as "unconstrainted type variable" errors during
inference (because indeed the value of the type parameter is not
relevant). This is because lint warnings are only issued after type
checking, even though in this case the lint has been added to the lint
list before type checking begins. This is annoying and comes up from
time to time. It may be worth reporting the results of some lints
"early", before type-checking runs.

# Alternatives

**Default to a particular variance when a type or lifetime parameter
is unused.** A prior RFC advocated for this approach, mostly because
markers were seen as annoying to use, and because Rust so frequently
has a single right choice (`CovariantType`,
`ContravariantLifetime`). However, after some discussion, it seems
that it is more prudent to make a smaller change and retain explicit
declarations. We can always modify this behavior in the future in a
backwards compatible way.

Some factors that influenced this decision:

- Many unused lifetime parameters (and some unused type parameters) are in
  fact completely unnecessary. Defaulting to a particular variance would
  not help in identifying these cases (though a better dead code lint might).
- There are cases where phantom type parameters ought to be
  *invariant* and not *covariant* (e.g., `AtomicPtr`). Admittedly, these
  are few and far between. But defaulting to covariant would make these
  types silently wrong.
- Phantom type parameters occur rarely so it is not particularly painful
  to use explicit notation.
  
**Use a hard error rather than a lint.** I chose to use a lint because
if and when we encounter a rare case where bivariance is appropriate,
one can always turn off the lint.

**Remove variance inference and use fully explicit declarations.**
Variance inference is a rare case where we do non-local inference
across type declarations. It might seem more consistent to use
explicit declarations. However, variance declarations are notoriously
hard for people to understand. We were unable to come up with a
suitable set of keywords or other system that felt sufficiently
lightweight. Nonetheless this might be a good idea for a future RFC if
someone feels clever. I did gather some statistics at one point that
seem relevant:

- in standard library:
  - 85% of region parameters are contravariant
  - 15% are invariant
  - 0% are covariant
- for types, harder to analyze quickly, but:
  - ~50% invariant
  - ~25% covariant
  - ~25% contravariant

I found these results somewhat surprising, particularly the fact that
50% of types wound up as invariant. These results are somewhat
suspicious since in the process of implementing this RFC I did fix
various bugs in the inference algorithm. Perhaps I will regather them
at some point.

**Rename the marker types.** The current marker types have
particularly uninspiring names. In particular, it might be advisable
to remove everything but `CovariantType`, since the other markers can
all be modeled using variations on `CovariantType`. Renaming or
restructing the markers, however, seems out of scope for this RFC.

# Unresolved questions

None.

[b]: https://github.com/nikomatsakis/rust/tree/variance-defaults
