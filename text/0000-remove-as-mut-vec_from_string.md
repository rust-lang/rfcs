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
three years. So it would be good to be able to change the implementation later on.

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
since they just copy three pointer/usize on the stack (and maybe the optimizer even
eliminates those copies). 

# Drawbacks

When frequent changes to the buffer are made in an alternatingly safe (UTF8 
checked) and unsafe way, `as_mut_vec` would be more efficient than converting
back and forth between `String` and `Vec` (assuming the optimizer doesn't
eliminate those). But this is a fairly rare use case: Usually there is 
at most one unsafe change to a string buffer.

# Alternatives

An option would be to redesign the whole `std::string` module to achieve even higher flexibility. One possible design could be to just define a generic UTF8 wrapper. The module would mainly provide three things:

* A trait `StringBuf`. Types that implement that trait are able to be used as an underlying buffer of a string.
* `Utf8Wrapper<T: StringBuf>` type that provide UTF8-safe methods around the raw string buffer. Furthermore they can also provide a method `as_mut_buffer(&mut self) -> &mut T` that works like `as_mut_vec`. That wouldn't be a problem in this case because the type `T` is not fixed.
* A variety of string buffers (which implement `StringBuf`) and can be used as underlying buffer for `Utf8Wrapper`. For example: A fixed size buffer, a hybrid SSO buffer and an implementation for `Vec`.

The module would probably also have some type alias like `type VecString = Utf8Wrapper<Vec<u8>>` and `type SmallString = Utf8Wrapper<SSOBuffer>;`. The current `std::string` is just about UTF8-safety. To be able to use every buffer in an UTF8-safe way would be a huge benefit.

However, such a big change is impossible before releasing 1.0. Keeping 
`as_mut_vec` would limit the ability to change the implementation of `String` in the future, therefore removing it is a step in the right direction.

# Unresolved questions

None so far.
