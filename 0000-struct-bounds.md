- Start Date: 2014-03-26
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Allow bounds on formal type variable in structs and enums. Check these bounds
are satisfied wherever the struct or enum is used with actual type parameters.
Ensure all types are checked for well-formedness with respect to type variable
bounds.

# Motivation

Currently formal type variables in traits and functions may have bounds and
these bounds are checked whenever the item is used against the actual type
variables. Where these type variables are used in types, these types could
(should?) be checked for well-formedness with respect to the type definitions.
E.g.,

```
trait U {}
trait T<X: U> {}
trait S<Y> {
    fn m(x: ~T<Y>) {}  // Could be flagged as an error
}
```

Formal type variables in structs and enums may not have bounds. It is possible
to use these type variables in the types of fields, and these types cannot be
checked for well-formedness until the struct is instantiated, where each field
must be checked.

```
struct St<X> {
    f: ~T<X>, // Cannot be checked
}
```

Likewise, impls of structs are not checked. E.g.,

```
impl<X> St<X> {  // Cannot be checked
    ...
}
```

Here, no struct can exist where `X` is replaced by something implementing `U`,
so in the impl, `X` can be assumed to have the bound `U`. But the impl does not
indicate this. Note, this is sound, but does not indicate programmer intent very
well.

Essentially we have two kinds of type variables - those on traits and functions
which are checked up front (which I believe fits with our policy of type
checking generic code before type substitution). And those on structs and enums
which are checked 'on use' (which feels a bit more like C++ style checking of
generics after type substitution).

I would like all type variables to be treated consistently. This will make the
language simpler (semantically, although at the expense of some extra
verbosity), and type checking more consistent. Errors should be caught earlier
in the development process.

Furthermore, we are currently adding the `unsized` keyword (the actual keyword
might be `type`, but that is irrelevant here). `unsized` may be placed on any
formal type variable (including on structs), and is necessary to know about up
front, since it affects the in-memory layout of fields in structs. With the
proposed change, unsized is checked in the same way as other bounds (well, kind
of, it actually indicates the absence of an implicit bound, but that is a
detail). Without this change, unsized checking has to be a special case.

# Detailed design

* Allow bounds on type variable in structs and enums (this is the case
syntactically, but we have a check in collect.rs which forbids it).
* Wherever a concrete struct or enum type appears, check the actual type
variables against the bounds on the formals (the type well-formedness check).
* Ensure we do the type well-formedness check for trait and function types too.

From the above examples:

'''
trait U {}
trait T<X: U> {}
trait S1<Y> {
    fn m(x: ~T<Y>) {}  //~ ERROR
}
trait S2<Y: U> {
    fn m(x: ~T<Y>) {}
}

struct St<X: U> {
    f: ~T<X>,
}

impl<X: U> St<X> {
    ...
}
'''

# Alternatives

Not do this and leave things as they are. Check the `unsized` 'bound' as a
special case.

We could add bounds on structs, etc. But not check them in impls. This is safe
since the implementation is more general than the struct. It would mean we allow
impls to be un-necessarily general.

# Unresolved questions

Do we allow and check bounds in type aliases? We currently do not. We should
probably continue not to since these type variables (and indeed the type
aliases) are substituted away early in the type checking process. So if we think
of type aliases as almost macro-like, then not checking makes sense. OTOH, it is
still a little bit inconsistent.
