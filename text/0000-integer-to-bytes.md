- Start Date: 2014-01-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

The Standard Library currently does not provide a way to cast an integer into a byte array. This RFC would add such methods. 

# Motivation

I have been looking around in the rust source code, and I figured there is quite some places that replicate this behaviour. Also a lot of users seem to be asking around how to do this in rust. It is probably a good decision to put this in the standard library. 

# Detailed design

An implementation of the RFC can already be found [here](https://github.com/Binero/rust/blob/master/src/libcore/num/mod.rs#L373), [here](https://github.com/Binero/rust/blob/master/src/libcore/num/mod.rs#L464) and [here](https://github.com/Binero/rust/blob/master/src/libcore/num/mod.rs#L594). 
It was implemented in libcore. Tests (or examples as you please) can be found [here](https://github.com/Binero/rust/blob/master/src/libcoretest/num/mod.rs#L122).

# Drawbacks

In the future, this might has to be rewritten to not return a slice, but to return an fixed-size array. This is not possible at the moment in rust yet though. 

# Alternatives

The method could be moved outside of the Int trait, to allow it to be of a fixed size that way. This is less convenient however, and a future change to allow it to return a fixed-size array should be backward-compatible. 

# Unresolved questions

There are some places where rust replicates the behaviour of these functions, and they should probably be replaced by using this. [[1]](https://github.com/rust-lang/rust/blob/master/src/libstd/io/extensions.rs#L80)[[2]](https://github.com/rust-lang/rust/blob/master/src/libstd/io/extensions.rs#L121)[[3]](https://github.com/rust-lang/rust/blob/master/src/librustc_back/sha2.rs#L24)
