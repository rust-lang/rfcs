- Start Date: 2014-06-18
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Rename _mod.rs_ files to _self.rs_.

# Motivation

The name _self.rs_ feels more logical for this purpose given how `self` in module-scope refers to the enclosing module. Also, `Self` is used to refer to the enclosing trait, so there's a strong precedent of using "self" to refer to the enclosing "thing". For a _self.rs_ file, the enclosing "thing" is the folder it lies in, and that folder's name is the name of the module for which the _self.rs_ file is the implementation.

# Detailed design

Simply depracate _mod.rs_ files and require them to be renamed to _self.rs_

# Drawbacks

# Alternatives

# Unresolved questions
