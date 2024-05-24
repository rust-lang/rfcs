- Feature Name: collection_contains_trait
- Start Date: 2024-05-24
- RFC PR: [rust-lang/rfcs#3647](https://github.com/rust-lang/rfcs/pull/3647)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Implement a new `Contains` trait for all collections, slices, and maybe more.

# Motivation
[motivation]: #motivation

Currently giving a whitelist, a blacklist, or anything that can be described as one(such as a list of enabled/available features) to a function is only possible using `Iterator`s or by requiring a specific type of collection.

The disadvantage of using `Iterator`(or `IntoIterator`) is that you need to iterator over it to check whether an item is included.  
If it is not included you need to iterate over the entire collection.

The disadvantage of using a specific type of collection is that it doesn't allow the caller to decide which type of collection to choose for the code.  
This may be relevant in other places then just this one function though.  
Nor does it allow using special-purpose collections, unless the function requires them, in cases where they are preferable.

Of course using a specific type of collection allows the caller to use another type of collection, and then copy its contents to the required type of collection.  
This too, however, causes a significant needless performance penalty.

Having a `Contains` trait would allow both to make the intention behind this parameter more clear, as well as allowing better performance with `HashSet`s and `BTreeSet`s.

In other languages I would usually use something like `Set<T>`(or even `Collection<T>`) as the parameter type in this case.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `Contains<Foo>` trait declares that an object(usually a collection) supports checking whether it contains an object of type `Foo`.

## Example 1: Enabled Features

Using `Contains` for enabled features:

```rust
enum Features {
    Sort,
    Min,
    Max
}

pub fn fn_with_optional_features<F: Contains<Features>>(feat: &F) -> Vec<u16> {
    let mut vals = vec![0, 7, 1, 91, 135, 321, 23, 5];
    if feat.contains(&Features::Sort) {
        vals.sort();
    }
    if feat.contains(&Features::Min) {
        println!("The max value is {}.", vals.iter().min().unwrap());
    }
    if feat.contains(&Features::Max) {
        println!("The max value is {}.", vals.iter().max().unwrap());
    }
    vals
}
```

Of course this basic example can easily be done in any number of ways without `Contains`.

One example like this, which is too large to implement here, but where not having this can be a pain, are optional parser features.


## Example 2: Specifying all basic collection operations

This kind of generic block could be used to specify a parameter that needs to support all basic set operations(except remove, for which there is no trait).

```rust
pub fn needs_set<Set, Item>(set: Set)
where
    Set: Contains<Item> + IntoIterator<Item = Item> + Extend<Item>,
    Set::IntoIter: ExactSizeIterator,
    for<'a> &'a Set: IntoIterator<Item = &'a Item>,
    for<'a> <&'a Set as IntoIterator>::IntoIter: ExactSizeIterator
{
    // Use set for something
}
```

While a generic list could be specified like this:

```rust
pub fn needs_list<List, Item>(list: List)
where
    List: Contains<Item> + IntoIterator<Item = Item> + Extend<Item> + Index<usize, Output = Item> + IndexMut<usize, Output = Item>,
    List::IntoIter: ExactSizeIterator + DoubleEndedIterator,
    for<'a> &'a List: IntoIterator<Item = &'a Item>,
    for<'a> <&'a List as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator
{
    // Use list for something
}
```

Of course situations where a big generics block like the ones above are actually needed are pretty rare.


## Example 3: Blacklisted values

Another possibility would be to have a blacklist of values a function is not permitted to return.

```rust
pub fn algorithm_with_blacklist<BL: Contains<u64>>(blacklist: &BL) -> Option<u64> {
    let mut value = None;
    while !value.is_some_and(|val| !blacklist.contains(&val)) {
        if value.is_none() {
            value = Some(1);
        }
        // Some kind of complex algorithm
        value.replace(value.map(|val| (val << 5) ^ 0b01011001).unwrap());
    }
    value
}
```

This may make code slightly harder to read, by encouraging developers to create even more complex generic bounds,  
but it might make code slightly easier to maintain since it would make it easier to change the collection type used by a project, or a part of one.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The goal is to move the already existing `contains` methods from collection types to a trait.

For example 1 the improvement is relatively small, as the features could easily be either  
 a) be copied from an `Iterator` into a set, or  
 b) if they are as simple as those in the example they could easily be implemented as callbacks and directly called from an `Iterator`.


For example 2: from the basic set operations(add, remove, contains, iterate) contains and remove are currently not usable using a trait.  
For remove i'm uncertain about the best way to handle it, and also believe if you need to remove elements it might be more reasonable to either  
 a) move the elements to a collection with a precisely known type, or  
 b) require a specific type of collection as a parameter.  
For contains however i believe it would be nice to be able to do that with an unknown type using a trait.


For example 3 the `Contains` trait allows using collections of an arbitrary type for the blacklist.  
Also if `Contains` ends up being implemented for the range types, this would also make it possible to allow restricting possible values using either ranges or explicit lists in the same function.  
A custom `Contains` impl would then even allow things like for example forbidding all even values.

Of course implementations of this trait could be done in such a way that they keep the `Borrow` signature of the current set contains methods.


# Drawbacks
[drawbacks]: #drawbacks

 - I'm not entirely certain of how much of a breaking change it would be to move existing methods to a new trait, but that **might** cause issues.
 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

One alternative i considered was to call the new trait method `has` instead of `contains`, to avoid having to remove the native method.  
I believe that this is only worth it, though, if moving the `contains` method would be too much of a breaking change to have any chance of getting accepted.

Could this be done in a library?  
Kind of. It could be implemented, but there would be limitations.  
It could either be implemented with a method called `contains`, but this would mean that this function cannot just call the native `contains` method leading to code duplication.  
Alternatively it could be done if the trait method is called something else, for example `has`, but this would make it less intuitive to use.

I do not believe that this change will have any significant impact on the readability of code, but it might make it slightly easier to maintain.  
This is because it would make it easier to switch parts of a project to a different type of collection in the future.


# Prior art
[prior-art]: #prior-art

There isn't really anything.

I haven't used a language with a trait system before Rust, so i wouldn't know whether this is common.

The only related thing i could think of is [this one reddit post about collection traits](https://www.reddit.com/r/rust/comments/83q25s/why_no_traits_for_collections_in_rust/).  
But even that is only loosely related to this, though it did give me some motivation to actually write this RFC.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

 - Should non-collection types with a contains function(str, range) implement `Contains<T>`?
 - Should `Vec<T>` implement `Contains<T>` directly, or only using `Deref<Target = [T]>`?

Should the contains method of those types  
 a) be moved to the trait and instantly stabilized, or  
 b) be kept in place until this is stabilized, requiring users to use its fully qualified name, or  
 c) should the trait method be declared unstable and the the native methods be disabled using inverted feature flags, if inverted feature flags even exist?

# Future possibilities
[future-possibilities]: #future-possibilities

 - It would make sense to implement either `Contains<K>` or `Contains<(K, V)>`(or possibly both) for map types.
