- Start Date: 2015-01-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Decide the error handling policy for IO objects (Writer, BufferedWriter, etc.),
especially for "late" methods like flush and close.   Not checking the return
code of close is a common but severe error.


# Motivation

We want code to be correct and not let "errors pass silently".  But we also
don't want to worry about panics and double panics.  Find a pragmatic solution
to make the most code correct and obvious, and not have hidden surprises.  

# Detailed design

There should be a `.close(self)` method of Writer that returns A `Result<(),
IoError>`.  This also implicitly calls `.flush()`.  If flush failed, the error
will be what was returned from flush, but resources are still released.

IO objects that are being dropped should not call `.flush()` or `.close()` on
themselves or any sub objects; only the minimal cleanup (freeing memory,
releasing file descriptors) should be performed.  *drop means drop it on the
floor*

Developers should be advised to call `.close()` of any IO objects and check the
result.   If they don't, the remaining data won't get flushed and that should
get caught at dev time (a very good thing)

Developers should be advised that panic / unwinding will not perform flush or
close, and that's probably a good thing (explicit is better than implicit).
The rationale here is that a panic caused by array out of bounds may indicate
some severe error, and data should not be flushed (similar to the PoisonedMutex
philosophy).

# Drawbacks

Existing code breakage

It's not the "simple python way", e.g. developers have to "type another line".

# Alternatives

Java like suppressed exceptions, RAII, double panics, ...

I believe you either go all in and have destructors that can throw at any time
(including nested), or do not throw at all.   Rust's architecture fits the
latter.

# Unresolved questions

Is consuming .close(self) technically and practically possible.  You would lose the
.path() attribute if .close() returned an error (you could .clone() it before
hand though)

Does this work on what's returned from stdout() as well? I think so, its just a
BufferedWriter.  However, .close() on that won't actually close the fd.
