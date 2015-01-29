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
to make the most code correct, fast, obvious, and predictable.

Support use cases like compressed writers, encrypted writers, etc., which also
need end-to-end .close() checking and finalization.

Please also note that a "truncation attack" can be a severe security
vulnerability; having your /etc/passwd file only partially written is a DOS, as
is shortening a password.

# Detailed design

There should be a `.close(self)` method of Writer that returns A `Result<(),
IoError>`.  This also implicitly calls `.flush()`.  If flush failed, the error
will be what was returned from flush, but resources (in memory buffers, file
descriptors) are still released.

IO objects that are being dropped must not impicitly call `.flush()` or
`.close()` on themselves or any sub objects; only the minimal cleanup (freeing
memory, releasing file descriptors) should be performed.  *drop means drop it
on the floor*.  

Developers should be advised to call `.close()` of any IO objects and check the
result.   If they don't, the remaining data won't get flushed and that should
get caught at dev time (a very good thing)

The .close() method should probably set a flag indicating the file was properly
closed, so drop() doesn't try it again.  In addition, some IO objects with
special constraints - i.e. designed for network file systems - might find it
useful to print a warning or even panic if they are being dropped and a panic
is not currently active but .close() was not called; that means the developer
failed to attempt .close() on them.

Note that this RFC should not hurt the runtime speed of BufferedWriter, as
the 'was gracefully closed' flag is only checked by drop(), and .close(self)
consumes the object.

Developers should be advised that panic / unwinding will not perform flush or
close, and that's probably a good thing (explicit is better than implicit).
The rationale here is that a panic caused by array out of bounds may indicate
some severe internal error, and additional data should not be flushed (similar
to the PoisonedMutex philosophy).

# Drawbacks

Existing code breakage (although said code is likely buggy)

It's not the "simple python way", e.g. developers have to "type another line".

# Alternatives

Java like suppressed exceptions, RAII, double panics, do nothing...

RAII works great for releasing existing resources that can't fail.
Flushing data out to disk is not "an existing resource".

Suppressed exceptions in Java are useful for logging somewhere but it's hard to
see how actionable they are otherwise, and are not appropriate for a systems
language as it is basically an unbounded linked list.

I believe you either go all in and have destructors that can throw at any time
(including nested), or do not throw at all.   Rust's architecture fits the
latter.

# Unresolved questions

Is consuming .close(self) technically and practically possible.  You would lose the
.path() attribute if .close() returned an error (you could .clone() it before
hand though)

Does this work on what's returned from stdout() as well? I think so, its just a
BufferedWriter.  However, .close() on that won't actually close the fd.
