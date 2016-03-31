- Feature Name: (fill me in with a unique ident, my_awesome_feature)
- Start Date: 2016-03-31
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

One para explanation of the feature.

# Motivation
[motivation]: #motivation

There are not best practices for writing libraries for compression,
encryption and etc.

Compression libraries need some method for "finish" work: write end
stream mark and flush data.

This work MUST explicit call and SHOULD NOT run in drop() method:
on reading incomplete stream much perefered unexpected end of stream
instead of false positive success.

Also "finish" of work is a good place to "unwrap" original writer
or reader.

Currenctly compression libraries have handling thing differently:

 * snappy expects the user to call flush(), and do not allow
   inner writer retrieval;
 * flate2 offers finish(), but manage the inner ownership internally
   with an Option;
 * stdlib BufWriter goes even a bit further by handling a possible
   panic during the finish();
 * lz4-rs expects the user to call finish() and unwrap inner writer.

# Detailed design
[design]: #detailed-design

This is the bulk of the RFC. Explain the design in enough detail for somebody familiar
with the language to understand, and for somebody familiar with the compiler to implement.
This should get into specifics and corner-cases, and include examples of how the feature is used.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Alternatives
[alternatives]: #alternatives

What other designs have been considered? What is the impact of not doing this?

# Unresolved questions
[unresolved]: #unresolved-questions

What parts of the design are still TBD?
