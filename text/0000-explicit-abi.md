- Start Date: 2015-01-20
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Disallow omitting the ABI in `extern` declarations, both for extern functions and for external blocks.

# Motivation

Some advantages of making the ABI explicit are:
 - it would ensure a more uniform coding style (even in the main rust repo you can find both extern fn and extern "C" fn)
 - it would make it easier to search for what ABIs are used in a codebase
 - it would be more familiar to C++ programmers

Removing the default for both functions and blocks helps with ensuring consistency.

# Detailed design

Removing the default is simple, just a matter of making the token non-optional in the parser.

Updating exisiting code to take the change into account can be automated, as it is a straightforward search-and-replace.

# Drawbacks

It should be trivial to add the "C" ABI (the current default) wherever it is missing, but the change would still break existing code.

# Alternatives

Keeping the current default will not break code; it might lead to some inconsistent usage of the explicit "C" ABI, but this could be solved on a per-project basis using coding guidelines.

It would be possible to perform the change ony on extern functions or on external blocks, but this would make the grammar rules less consistent and more surprising for users.

# Unresolved questions

Should no-ABI empty `extern` blocks (occasionally used to link a library without explicitly accessing its functions) be allowed?
