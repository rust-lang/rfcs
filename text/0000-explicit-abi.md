- Start Date: 2015-01-20
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Disallow omitting the ABI in `extern` declarations.

# Motivation

Some advantages of making the ABI explicit are:
 - it would ensure a more uniform coding style (even in the main rust repo you can find both extern fn and extern "C" fn)
 - it would make it easier to search for what ABIs are used in a codebase
 - it would be more familiar to C++ programmers

# Detailed design

Removing the default is simple, just a matter of making the token non-optional in the parser.

Updating exisiting code to take the change into account can be automated, as it is a straightforward search-and-replace.

# Drawbacks

It should be trivial to add the "C" ABI (the current default) wherever it is missing, but the change would still break existing code.

# Alternatives

Keeping the current default will not break code; it might lead to some inconsistent usage of the explicit "C" ABI, but this could be solved on a per-project basis using coding guidelines.

# Unresolved questions

Should no-ABI empty `extern` blocks (occasionally used to link a library without naming its functions) be allowed?
