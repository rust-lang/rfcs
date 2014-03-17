- Start Date: (fill me in with today's date, YYY-MM-DD)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Change Rust's variance inference rules to make them less error-prone.

# Motivation

Rust currently employs automatic variance inference to decide whether
a type parameter is covariant, contravariant, or invariant. The
current inference scheme is as general as possible but sometimes leads
to surprising behavior, particularly around type or lifetime
parameters that are not used.

For example, consider the following struct definition:

    struct Wrap<T>;
    
Here, the type parameter `T` is not used at all. The inference
algorithm will infer that this is *bivariant*, meaning that no matter
what value we supply for `T`, accesses to the fields of `Wrap` will
continue to be safe. In other words, if I have `&Wrap<int>`, that is
interchangable with `&Wrap<uint>`. The algorithm is not wrong. Since
`Wrap` has no fields, there is nothing you can do with this `T` that
is incorrect per se. But the result is certainly surprising.

The current algorithm is particularly broken when people employ unsafe
code that may encode uses for types that are not immediately evident.
For example, one idiom is to attach a lifetime to a struct that
contains unsafe pointers, to prevent that struct from being used
outside of a certain scope. For example, at some point the iterator
for a vector was defined as follows:

    struct Elements<'a, T> {
        start: *T, end: *T
    }
    
Here of course, the lifetime `'a` is not used, and hence the inference
algorithm again infers bivariance. This is certainly not what was
intended.

The current way to control the inference algorithm is to employ marker
types like `CovariantType` or `ContravariantLifetime`. These are
maximally flexible but also dangerous as defaults -- if you don't know
that you need to use them, you will get very lax typing, much laxer
than you might expect.

# Detailed design

I'd like to change the default behavior of the inference algorithm as
follows: **If a type or lifetime parameter is not used within the body
of a type, we default to covariance for types and contravariance for
lifetimes.**

This change chooses as defaults what I believe most people would expect.
More specifically, if you write a struct with an unused type parameter `T`:

    struct Foo<T> { } // Note: T is not used
    
That is equivalent (from the variance algorithm's point of view) to
a struct that contains an instance of `T`:

    struct Foo<T> { field: T } // Foo<T> above equivalent to this
    
Similarly, if you write a struct with an unused lifetime parameter `'a`:

    struct Bar<'a> { } // Note: 'a is not used
    
That is equivalent to a struct containing a reference of lifetime `'a`:

    struct Bar<'a> { f: &'a () } // Note: 'a is not used
    
This is what I intuitively expect when I see `Foo<T>` or `Bar<'a>`.

# Alternatives

There are many possible alternatives:

**Keep the current system, with the attendant risks.** I think we'll
see more and more bugs this way.

**Default to invariance for unused type parameters, not covariance.**
This would mean that, e.g., `Foo<&'static int>` is not a subtype of
`Foo<&'a int>`. Probably not much difference in practice, but I think
it's slightly more intuitive to follow the rule that unused type
parameters act *as if* they appeared the struct had a member of that
type.

# Unresolved questions

None.
