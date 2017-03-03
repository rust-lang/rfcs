- Start Date: 2014-06-14
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add support for not consuming an entire Iterator\<char\> to serialize::json::Parser and Builder; in other words, allow incrementally reading one JSON object at a time.

# Motivation

In networked applications, a socket may be used to transfer multiple JSON objects by streaming them, concatenated, across the network. As serialize::json::Parser currently stands, it is incompatible with this usecase (modulo hacks that e.g. separate JSON objects with terminator characters and pass each object as a string to a Builder). This problem is solvable with a very simple modification to Parser.

# Detailed design

A private boolean "incremental" would be added to Parser and Builder. A new argument "incremental" would also be added to their constructors, setting the value of this boolean. This flag would change the behavior of Parser and Builder as follows:

## Parser

In Parser.next, when it would normally yield a TrailingCharacters error, reset the Parser's state to the initial state instead.

## Builder
When constructing a Builder with "incremental", also construct the Parser with "incremental" set. No other changes should need to be made- calling Builder.build() multiple times should return multiple JSON objects, until the end of the Iterator\<char\>. Iterator\<Json\> could also be implemented for Builder.

# Drawbacks

Complexity is added to Parser and Builder, and their size is trivially increased.

Code using Builder::new or Parser::new directly must be rewritten; however, most code in the wild uses the wrapper functions serialize::json::from\_str or serialize::json::from\_reader . Changing these wrapper functions should obviate any need for changes in the majority of programs.

# Alternatives

Leaving this unchanged causes enormous headaches when attempting to send several JSON objects through the same socket. Although they could be passed as a JSON list, Builder will consume the entire list before returning it; thus lists cannot be used to communicate with JSON.

# Unresolved questions

"incremental" could be bikeshedded a bit.

Should Builder implement Iterator\<Json\>  ?
