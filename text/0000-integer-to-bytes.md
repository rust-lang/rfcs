- Start Date: 2014-01-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

The Standard Library currently does not provide a way to cast an integer into a byte array. This RFC would add such methods. 

# Motivation


There are some places where rust replicates the behaviour of these functions, and they should probably be replaced by using this. [[1]](https://github.com/rust-lang/rust/blob/8efd9901b628d687d11a4d0ccc153553b38ada49/src/libstd/io/extensions.rs#L82)[[2]](https://github.com/rust-lang/rust/blob/8efd9901b628d687d11a4d0ccc153553b38ada49/src/libstd/io/extensions.rs#L123)[[3]](https://github.com/rust-lang/rust/blob/8efd9901b628d687d11a4d0ccc153553b38ada49/src/librustc_back/sha2.rs#L24) Also a lot of users seem to be asking around how to do this in rust. It is probably a good decision to put this in the standard library. 

# Detailed design

An implementation of the RFC can already be found [here](https://github.com/Binero/rust/blob/8a39638f02ea5600c0292d82c647ef77d14d525a/src/libcore/num/mod.rs#L382), [here](https://github.com/Binero/rust/blob/8a39638f02ea5600c0292d82c647ef77d14d525a/src/libcore/num/mod.rs#L465) and [here](https://github.com/Binero/rust/blob/8a39638f02ea5600c0292d82c647ef77d14d525a/src/libcore/num/mod.rs#L595). 
It was implemented in libcore. Tests (or examples as you please) can be found [here](https://github.com/Binero/rust/blob/8a39638f02ea5600c0292d82c647ef77d14d525a/src/libcoretest/num/mod.rs#L123).

# Drawbacks

In the future, this might has to be rewritten to not return a slice, but to return an fixed-size array. This is not possible at the moment in rust yet though. This change however should be backwards compatible. 

# Alternatives

The method could be moved outside of the Int trait, to allow returning a fixed-sized array. In the future though, this will be possible in traits too, and as mentioned in 'Drawbacks' a future change to make it return a fixed sized array should be backwars compatible. 

# Unresolved questions

~~There should probably be a similar function, that does the same thing, but in reverse.~~ 
[Already exists here.](https://github.com/rust-lang/rust/blob/8efd9901b628d687d11a4d0ccc153553b38ada49/src/libstd/io/mod.rs#L734)
