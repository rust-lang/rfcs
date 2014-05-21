- Start Date: 2014-05-21
- RFC PR #:
- Rust Issue #:

# Summary

Space optimization for variables of type ```Option<E>``` when ```E``` is an integral enum type.

# Motivation

There's no need to waste memory for storing a separate tag in variables of type ```Option<E>``` if ```E``` is an integral enum type and the set of valid values of ```E``` does not cover all possible bit patterns. Any ```E``` size bit pattern that doesn't represent a valid value of type ```E``` could be used by the compiler to represent a ```None``` value of type ```Option<E>```.

# Detailed design

Given an integral enum type ```E```, the compiler should check if some bit pattern exists which does not represent a valid value of ```E``` as determined by the enumerators of type ```E```. If such a bit pattern is found, the compiler should use it to represent a ```None``` value of type ```Option<E>``` and omit storing the tag in variables of type ```Option<E>```. If more than one such "invalid" bit pattern exists, the compiler should be free to choose any one of those.

# Drawbacks

Any serialization format that wants to stay backward compatible should take into consideration that if an enumerator is added after some ```Option<E>``` values have been serialized to disk, the new enumerator might have the same bit pattern as what was earlier used for a ```None``` value.

# Alternatives

-

# Unresolved questions

There are a lot of types ```T``` for which ```Option<T>``` could be space-optimized. But this proposal does not attempt to cover any of those other cases.
