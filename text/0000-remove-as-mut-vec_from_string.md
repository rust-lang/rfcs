- Feature Name: remove_as_mut_vec_from_string
- Start Date: 2015-02-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Remove the method `std::string::String::as_mut_vec` to allow `String` 
implementations not using `Vec` as underlying buffer.

# Motivation

`String` is currently merely a `Vec`-wrapper which ensures that the buffer
contains valid UTF8. There is nothing particularly bad about using `Vec` as the
underlying buffer, but the current interface of `String` (specifically: 
`as_mut_vec`) makes it almost impossible to implement it in any other way.

Restricting the default string type in its implementation is by no means a 
good idea. `Vec` and `String` are pretty much the same on a technical level, but
are very different in their uses. There are some techniques to utilize the
special usage pattern of string types, that would be impossible to use with 
`as_mut_vec` in `String`'s interface. 

One of those techniques is SSO (small string optimization) which is widely used
in the C++ world (clang's standard library's `std::string` uses SSO, gcc would
use it too but is restricted by their standard library ABI). There still is a
need for good benchmarks comparing SSO and the default string implementation, 
but this RFC is not just about SSO. There are more promising implementations of
strings and there might be even more in the future. It's difficult if not 
impossible to say what is the fastest implementation now or what it will be in 
3 years. So it would be good to be able to change the implementation later on.

# Detailed design

The method `as_mut_vec` returns a mutable reference to a `Vec` that owns the
same internal buffer as the string allowing to change the buffer 
without UTF8 checks. This of course implies that a `Vec` (`ptr`, `len` and 
`cap` -> 24 bytes) is stored somewhere. There are a number of ways how to 
implement this method without using `Vec` as an internal buffer, but they all 
add overhead: Either they increase the string type's size or they add branches 
to every other method. Furthermore, `as_mut_vec` will very likely be expensive. 
To summarize: It is possible to implement `as_mut_vec`, but not in a fast/good 
way.

The solution is to just remove `as_mut_vec` from `String`. The same 
functionality can be achieved with other methods anyway: Instead of using 
`as_mut_vec` to obtain a reference into the string one can use 
`into_bytes(self) -> Vec<u8>` and `from_utf8_unchecked(Vec<u8>) -> String` to 
"convert" the `String` into a `Vec` and back. Those "conversions" are very cheap
since they just copy 3 pointer/usize on the stack (and maybe the optimizer even
eliminates those copies). 

# Drawbacks

When there are frequent changes to the buffer in an alternatingly safe (UTF8 
checked) and unsafe way, `as_mut_vec` would be more efficient than converting
back and forth between `String` and `Vec` (assuming the optimizer doesn't
eliminate those). But this is a fairly rare use case: In most cases there are 
just 0 or 1 unsafe changes to a string buffer. 

# Alternatives

It would also be possible to redesign the whole `std::string` module to get 
even more flexibilty out of it. But such a big change is not possible before 
releasing 1.0. Not removing `as_mut_vec` would limit the ability to change the
implementation of `String` in the future.

# Unresolved questions

None so far.
