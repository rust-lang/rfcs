- Start Date: 2015-01-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Decide the error handling policy for IO objects that write to files (Writer,
BufferedWriter, etc.), especially for "late" methods like flush and close.
Not checking the return code of close is a common but severe error, especially
with async network filesystems.


# Motivation

Writing to files safely presents special challenges.  Unlike sockets, where
there is usually bidirectional communication, explicit ok responses, and
expectation of packet loss, writing to a file successfully is (unfortunately)
signaled by close().   Doing this in a destructor which can't throw (or
certainly not double throw), has been tried in C++ and is not safe.

We want code to be correct and not let "errors pass silently".  But we also
don't want to worry about panics and double panics.  Find a pragmatic solution
to make the most code correct, fast, obvious, and predictable.

Support use cases like compressed writers, encrypted writers, etc., which also
need end-to-end .close() checking and finalization.

Please also note that a "truncation attack" can be a severe security
vulnerability; having your /etc/passwd file only partially written is a DOS, as
is shortening a password.

Rust "somewhat" works today because Rust's stdout is line buffered all the
time.  When that changes to be fully buffered, except to tty, that means even
`println!()` panic semantics are totally useless, because nothing will be
written until flush/close/drop.


# Detailed design

There should be a `.close(self)` method of Writer that returns A `Result<(),
IoError>`.  This also implicitly calls `.flush()`.  If flush failed, the error
will be what was returned from flush, but resources (in memory buffers, file
descriptors) are still released.

IO objects that are being dropped must not implicitly call `.flush()` or
`.close()` on themselves or any sub objects; only the minimal cleanup (freeing
memory, releasing file descriptors) should be performed.  *drop means drop it
on the floor*.  

Developers should be advised to call `.close()` of any IO objects and check the
result.   If they don't, the remaining data won't get flushed and that should
get caught at dev time (a very good thing).

The .close() method should set a flag indicating the file was properly
closed, so drop() doesn't try it again.  

# Rationale

The reason `.close` consumes self is for two reasons.  First, it doesn't hurt
the runtime speed of Writers, as the 'was closed' flag is only checked by
drop().  Second, close is not a retryable operation on posix, so this reflects
that.

One reason drop should not flush (due to unwinding or early return) is that
explicit is better than implicit.  In some case, an early return may well mean
the programmers intent it *not* to flush other objects.  In panic cases, due to
array out of bounds or out of memory, a programmer error may have happened or
trying to flush a giant buffer during unwinding would be counter productive.

The other reason drop does not flush is it encourages correct error handling.
Developers know that HTTP requests and database transactions have to be checked
for success.  Nobody in their right mind would make a destructor commit a
database transaction with no way to report failure. So it's mystifying why so
many think filesystems are somehow "more magical" and can't fail, and
destructors should try to "commit" filesystem actions by default.

RAII works great for releasing existing resources that can't fail.  Attempting
to flush buffers and metadata out to disk is not "an existing resource".

Note that writer having write, flush, and close methods mirrors what Java does,
except close consumes the object.


# Drawbacks

Existing code breakage (although said code is likely buggy)

It's not the "simple python way", e.g. developers have to "type another line".

People that don't like it have to write a wrapper `UnsafeCloser<T>` guard to
return to the current implicit, but not error checked, semantics.  (Note that the
reverse is not possible today; I can't write a wrapper to undo an implicit close)


# Alternatives

Add the `.close()` method but still fall back to flushing/closing with no error
checks in drop(), regardless of if a panic is happening.  I think this tries to
predict intent and is not safe; there could be many bad reasons for the initial
panic and 'fail stop' could be more appropriate than flushing more buffers.  It
also encourages not checking errors.  Note: this approach is the current status quo.

Add an UnsafeCloser guard in std:: to make the above more explicit opt-in.  (Of
course that's probably as much work as just writing the .close() call)

Add the `.close()` method but still fall back to flushing/closing with no error
checks in drop(), only if a panic was not happening.  This will not cover the
case where a file is created and dropped entirely in a destructor during a
panic, and is hard to reason about - it increases state space.  This also tries
to predict programmer intent; if `try!(myfile1.close())` failed, perhaps myfile2
should not be automatically flushed either.

Add the `.close()` method but still fall back to flushing/closing with panic
semantics in drop(), regardless of if a panic is happening.  This seems
guaranteed to give double panics or needless panics, so is a non starter.  (See
the bottom example of closing two files)

Add the `.close()` method but still fall back to flushing/closing with panic
semantics in drop(), only if if a panic was not happening.   Same problems as
previous (try! closing two files needlessly panicing).

golang examples typically show `defer dst.Close()` (no error checking, not
useful)

Suppressed exceptions in Java are useful for logging somewhere but it's hard to
see how actionable they are otherwise, and are not appropriate for a systems
language as it is basically an unbounded linked list.


# Unresolved questions

Is consuming .close(self) technically and practically possible.  You would lose the
.path() attribute if .close() returned an error (you could .clone() it before
hand though)

Does this work on what's returned from stdout() as well? I think so, its just a
BufferedWriter.  However, .close() on that won't actually close the fd.


# Tried but Rejected

I thought that files could warn or panic if they went out of scope and
`.close()` wasn't called.  However, that could have many false positives.  You
could start writing a file out and then discover your inputs were faulty, so
you'd just `return` early from a scope and the output file would be implicitly
dropped, and that should not indicate programmer error.

A Closeable trait - where things are auto closed with panic semantics on
"normal" exit from scope is also quite difficult.  Just consider:

    try!(my_file1.close());
    try!(my_file2.close());

If `my_file1.close()` errors, my_file2 will and should go out of scope implicitly
and not cause a panic.  The programmer clearly intends that this code should not
panic.
