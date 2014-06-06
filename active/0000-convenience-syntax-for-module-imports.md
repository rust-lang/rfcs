- Start Date: 2014-06-06
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add syntax sugar for importing the module itself along with some of its items.

# Motivation

The new syntax would make import clauses more concise.

# Detailed design

Instead of having to write:
```
use module::Type;
use module;
```
...the programmer could write:
```
use module::{self, Type};
```
The ```self``` keyword must be listed first in the list of items being imported and it refers to the module directly on its left. A syntax such as:
```
use module::self;
```
...should cause a compile-time error saying "Invalid syntax `use module::self;`. Use the syntax `use module;` instead."

# Drawbacks

This does complicate the language a miniscule amount.

# Alternatives

# Unresolved questions
