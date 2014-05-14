- Start Date: 2014-05-09
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Provide syntax to allow multiple traits to be implemented in a single `impl` block.

# Motivation

The current syntax for implementing multiple Traits is quite verbose, and this simple syntactic change will make code more understandable than mountains of boilerplate

# Drawbacks

This change might make discerning which function belongs to trait more difficult when visually parsing source code. However, it seems as though trait functions should be easily identifiable with the traits they belong to. Also, proper tooling support should help with this.

# Detailed design

If this RFC is implemented, the following syntax will be allowed:

```rust

impl MyFirstTrait + MySecondTrait for MyStruct { //the RFC will allow this 
//... method impls here
}
```

The plus sign `+` is used as it holds consistent to the current syntax for multiple Traits used in bounds.

# Alternatives

* Keeping the current syntax is not too bad, but this syntax would be quite helpful, and possibly make it more readable and understandable.

* Instead of a plus sign `+` a comma `,` could be used, for consistency with other languages. However keeping consistency with Rust is more beneficial than with other languages.

# Unresolved questions

* Unknown.


# See Also

* Java and C# style interface implementation syntax.
* [Example of use](https://gist.github.com/sinistersnare/b656e39480372606edb5)