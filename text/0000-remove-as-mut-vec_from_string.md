- Feature Name: remove_as_mut_vec_from_string
- Start Date: 2015-02-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Remove the method `std::string::String::as_mut_vec` to allow `String` 
implementations not using `Vec` as underlying buffer.

# Motivation

`String` is currently merely a `Vec`-wrapper that ensures that the buffer
containts valid UTF8. There is nothing particular bad with using `Vec` as the
underlying buffer, but the current interface of `String` (specifically: 
`as_mut_vec`) makes it almost impossible to implement it in any other way.

Restricting the default string type in it's implementation is by no means a 
good idea. `Vec` and `String` are pretty much the same on a technical level, but
are very different in their use. There are some techniques to utilize the
special usage pattern of string types, that would be impossible to use with 
`as_mut_vec` in `String`'s interface. 

One of those techniques is SSO (small string optimization) which is widely used
in the C++ world (clang's standard library's `std::string` uses SSO, gcc would
use it too but is restricted by their standard library ABI). There is still a
need for good benchmarks comparing SSO and the default string implementation, 
but this RFC is not just about SSO. There are more promising implementations of
strings and there may be even more in the future. It's difficult or rather 
impossible to say what is the fastest implementation now or in 3 years. So it's
good to be able to change the implementation later on.

# Detailed design

The method `as_mut_vec` returns a mutable reference to a `Vec` that owns the
same internal buffer as the string making it possible to change the buffer 
without UTF8 checks. This of course implies that a `Vec` (`ptr`, `len` and 
`cap` -> 24 bytes) is stored somewhere. There are a number of ways how to 
implement this method without using `Vec` as an internal buffer, but they all 
add overhead to every other method: Either they increase the string type's size
or they add branches to every other method. Furthermore `as_mut_vec` will very 
likely be expensive. To summarize: It is somehow possible implement 
`as_mut_vec`, but not in a fast/good way.

The solution is to just remove `as_mut_vec` from `String`. 

# Drawbacks

Why should we *not* do this?

# Alternatives

What other designs have been considered? What is the impact of not doing this?

# Unresolved questions

What parts of the design are still TBD?
