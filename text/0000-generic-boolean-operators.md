- Feature Name: generic_boolean_operators
- Start Date: 2015-03-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

We need to consider the forward compatibility path for allowing the boolean binary operators currently defined by `Eq` and `Ord` to be generic in the same way as the operators defined in the traits at `std::ops`. The practical consideration is that `==`, `!=`, `<`, `>`, `<=`, and `>=` may not return `bool`, and we need to consider if and how we may wish to implement this API elegantly, and not back ourselves into a corner without due consideration.

# Motivation

It can be very helpful when considering API design to allow uses of operators to return delayed calculations and other types that have not been anticipated beforehand. This is currently possible for the operators defined in `std::ops`, as implementations of the trait may choose to return _any_ type for any given combination of `LHS` and `RHS` types. However, it is _not_ possible for the boolean operators defined at `std::cmp`.

One example of how this ability is used in another language is with the SQL Alchemy ORM in Python, which is well known for its carefully considered API design and faithfulness to the underlying SQL.


```
session.query(MyModel).filter(
    MyModel.my_field == 'forty-two' &
    MyModel.my_other_field > 57
).first()
```

What's of particular interest in this example is the expression inside the `filter()` method.

* It uses operators to bring the same level of clarity to the filter as you'd get if you were dealing with objects in Python rather than translating and sending this to an SQL backend.
* If the operators were required to return `bool`, then the filter would not be able to be used in this expressive manner.
* The operators are recognized as normal syntax in the language, and give better visual scannability of the meaning of the code.

What follows is one idealized design to implement this feature, which may not be possible in current rust. The purpose is to help us consider forward compatibility for the ideal API, so that we can release Rust 1.0 with an understanding of what we can and cannot do with regard to implementing this. 


# Detailed design

## Add traits for each boolean operator

Following the pattern set up in `std::ops`, we should create traits in `std::ops` for the boolean operators. These should be named in accordance with their associated method names. Specifically, they should be called:

* `Eq::eq` -> `==`
* `Ne::ne` -> `!=`
* `Gt::gt` -> `>`
* `Lt::lt` -> `<`
* `Ge::ge` -> `>=`
* `Le::le` -> `<=`

These new traits should live in `std::ops`, and should match the usage of the current traits. For example, this should be the definition of `Eq`:

```
pub trait Eq<RHS = Self> where <Self as Eq<RHS>>::Output: Sized {
    type Output;
    
    fn eq(self, rhs: RHS) -> <Self as Eq<RHS>>::Output;
}
```

## Rename `Eq` and `PartialEq` to `Equiv` and `PartialEquiv`

Because `std::ops::Eq` will be the trait that defines the `==` operator, and because we may wish to allow that trait in the prelude at some point, it makes the most sense to find a more suitable name for `std::cmp::Eq`. For this I propose `std::cmp::Equiv`. Because we want the partial form to match, that also means renaming `std::cmp::PartialEq` to `std::cmp::PartialEquiv`.

This is a **_breaking change_** that will have widespread effects because of its ubiquitous use across the Rust ecosystem.

## Implement `Equiv` and `Ord` in terms of the operator traits

The semantics of `Equiv` (formerly `Eq`) and `Ord` are both useful and ergonomic, and losing the ability to use those semantics easily would be a step backward for the language. Implementing them in a fully expressive way would require being able to allow the `Equiv` and `Ord` traits to declare that their implementations _will_ implement a suitable form for `Eq`, `Ne`, etc, and will _also_ require that those methods return `bool`.

As far as I know, this is not possible to represent in Rust traits currently. However, if the feature were reasonable and desired, this ability _could_ be added in future versions of Rust without backward compatibility concerns (barring some undecidability issues that I haven't thought of yet). Because of that, we may wish to only rename `Eq` to `Equiv`, and leave the rest for backward-compatible changes after the release of this feature, which itself would likely be after the 1.0 release.

## Note on short-circuit operators

Due to the nature of short-circuit operators, it is assumed that `&&` and `||` will not be able to be overloaded in a similar way. If it were, how would it determine whether or not to do the second operation, since it's not required to be a `bool`?

# Drawbacks

## Backward incompatible change

It's a backward-incompatible change, much too close to 1.0 to be comfortable. That's also a reason to do it, because we won't be willing to make backward incompatible changes after 1.0 is released.

## Different naming than other languages
`Eq` and `Ord` are names that have history in other languages. We don't want to be different without due consideration.

# Alternatives

## Current traits, custom return types

Continue to use the `Eq` and `Ord` traits and their partial forms, but modify them to allow returning types other than `bool`. This is inelegant, and lessens the semantic meaning of implementing `Eq` and `Ord`. It also makes the ergonomics of `Eq` and `Ord` much more unpleasant.

## Allow `Eq` to be implemented in `std::ops`

The naming conflict with `Eq` doesn't necessarily mean that the current `Eq` needs to be renamed to `Equiv`. Since they are in different namespaces, they _could_ co-exist. One of them wouldn't ever be able to be imported in the standard prelude, though.

## Use macros

It's been suggested that using an `ast!` macro (not yet implemented) would be a better approach than allowing the boolean operators to be modified in this way. While macros are a most powerful and awesome tool, I think that they do not eliminate the utility of customizing operators in this way.

They are also not nearly as easily parsable by language tools, especially by text editors, as they will need to be taught about either the whole macro system or each macro in order to properly identify tokens for uses such as syntax highlighting.

## Use methods, not operators

It's also been suggested that these operations should really just be methods, and that instead of wanting `==` to return something other than `bool`, we should just create a new trait, and have the `.eq` method on that trait used, and not bother with the operators at all.

In that scenario, we'd need to do:

```
MyModel.my_field.eq(42).and(MyModel.my_other_field.gt(17))
```

instead of

```
MyModel.my_field == 42 & MyModel.my_other_field > 17
```

It is my opinion that operators are _significantly_ more readable and parsable than methods in this context, and that the return value not being a boolean is apparent in the context. That being said, if allowing the boolean operators to return non-boolean values was not acceptable, then this would probably be the next best option for my use-case.

## Confirm the current state

Resolve that the boolean operators defined by `std::cmp::{Eq,Ord}` should only ever return booleans, and that Rust won't ever wish to implement traits for the boolean operators outside those traits.

# Unresolved questions

* What are the reasons that the boolean operators aren't _already_ defined in a way that matches `std::ops`?
* Is `Equiv` the right name for the rename of `std::cmp::Eq`?
* Is there an alternative grouping that would be preferable over either `Eq` and `Ord` as-is or per-operator traits?
