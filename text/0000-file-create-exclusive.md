- Feature Name: file_create_exclusive
- Start Date: 2015-10-30
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Atomically create a file is it does not exist, or fail.

# Motivation
[motivation]: #motivation

This exposes an OS operation available on at least Unix and Windows. It can be
emulated as a non-atomic operation.

My use case is to sequentially create files like `snapshot-1`, `snapshot-2`,
etc. without overwriting an existing file. I believe a more common usage for
this feature is to open a temporary file ensuring this does not conflict with
another file.

# Detailed design
[design]: #detailed-design

Add another function to `std::fs::OpenOptions`:
    
    /// Sets the option for creating a file exclusively.
    /// 
    /// If this is set and "create" is set, the `open()` operation shall fail
    /// if the file already exists.
    /// 
    /// The check for the existence of the file and the creation of the file
    /// if it does not exist shall be atomic with respect to other threads
    /// executing open() naming the same filename in the same directory with
    /// "exclusive" and "create" options set.
    fn exclusive(&mut self, excl: bool) -> &mut OpenOptions

On Unix this shall set the [`O_EXCL` flag](http://linux.die.net/man/3/open).
On Windows this shall use the [`CREATE_NEW` parameter](https://msdn.microsoft.com/en-us/library/windows/desktop/aa363858%28v=vs.85%29.aspx).

# Drawbacks
[drawbacks]: #drawbacks

I don't see any. This functionality should eventually be exposed.

# Alternatives
[alternatives]: #alternatives

I have no idea whether more extensive modifications to the file-open
functionality are under way.

# Unresolved questions
[unresolved]: #unresolved-questions

I have looked over the code, and implementation looks straightforward, so I do
not anticipate any issues. There may however be other opinions over how this
functionality should be exposed.
