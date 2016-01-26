- Feature Name: thread_affinity
- Start Date: 2016-01-26
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Thread affinity provides functionality to lock/unlock a specific thread to a specified CPU core.
The locked threads are then not scheduled so they do not ever move to other cores by the OS scheduler. I believe it useful for a systems programmer to have the ability to set/unset set thread affinity for Rust threads.

# Motivation
[motivation]: #motivation

My motivation derives from a personal use-case of creating a fiber-based server/client application framework for Rust in my spare time. I need the ability to set thread affinity of Rust threads in an OS-indepedent manager to CPU cores to minimize cache misses and thread context switching.

The proposed feature provides a performance optimization for servers. It would also be useful for MIO and similar frameworks where the main event-loop is locked to one core and passes connections to workers that are also locked to the remaining cores so it takes advantage of cache-locality.

# Detailed design
[design]: #detailed-design

I strongly encourage others to improve my RFC since I am not a Rust expert and only serves to start a dialog on a proposed API.

I split it into two parts: *User API* and *Implementation*.

### Locking to an arbitrary free CPU core
```
let t = thread::spawn(move || {
    let cpu_lock = CpuLock::lock().unwrap(); // the current thread is now locked to an arbitrary free CPU core
    let cpu_id = cpu_lock.cpu_id();
    // thread logic goes here
    // the cpu_lock will unlock from the core once out of scope
});
```

Here's the equivalent but with some syntatical sugar to hide the lock if the user doesn't care to access it:

```
let t = thread::spawn(move || {
    // thread logic goes here
    // the cpu_lock will unlock from the CPU core once out of scope
});
```

The user may want more fine grained control of which core it should be locked to.

### Locking to a specific CPU core

```
// locks thread to cpu core 2
let t = thread::spawn(move || {
    let cpu_no = 2;
    let cpu_lock = CpuLock::lock_on(cpu_no).unwrap(); // the current thread is now locked to an arbitrary free CPU core
    // thread logic goes here
    // the cpu_lock will unlock from the core once out of scope
});
```

```
// locks thread to cpu core 2
let t = thread::spawn(move || {
    let cpu_no = 2;
    let cpu_lock = CpuLock::lock_on(cpu_no).unwrap(); // the current thread is now locked to an arbitrary free CPU core
    let cpu_id = cpu_lock.cpu_id();
    // thread logic goes here
    // the cpu_lock will unlock from the core once out of scope
});
```

### Unlocking prematurely

```
// locks thread to cpu core 2
let t = thread::spawn(move || {
    let cpu_lock = CpuLock::lock().unwrap(); // the current thread is now locked to an arbitrary free CPU core    
    // thread logic goes here
    cpu_lock.unlock();

});
```

## Implementation

The implementation is quite simple as it just needs to store the underlying thread's id and invoke system calls to lock/unlock.

*  Windows -> *SetThreadAffinityMask*

*  Nix: -> *PTHREAD_SETAFFINITY_NP(3)*

The implementation needs to know the number of available CPUS and manage state to know which CPUs are available for arbitrary locking.

To get the affinity mask from the cpu no is trivial:

```
let cpu_no = 2;
let affinity_mask = 1 << cpu_no;
```

### Corner-cases

1. Passing an cpu number out of bounds.

2. Moving the lock out of the thread.

3. Moving the locked from one thread to another thread.

# Drawbacks
[drawbacks]: #drawbacks

I believe the drawbacks of the current RFC are:

1. This RFC does not provide any empirical evidence of the computational benefits. I can spend time to provide benchmarks from a C/C++ benchmark.
2. Thread affinity can have the adverse effect - degrading performance if it interferes with the OS's scheduler if used in the wrong way.
3. My lack of experience of Rust could be prohibiting an optimal design.
4. Could be regarded as adding complexity to the standard library.

# Alternatives
[alternatives]: #alternatives

Calling unsafe OS-specific system calls to create threads and changing the affinity mask because `std::thread::Thread` do not expose the underling OS-specific thread id (AFIAW).

# Unresolved questions
[unresolved]: #unresolved-questions

No research has been conducted into the following hypotheses:

  *  Is it possible for consistent & symmetric semantics across all supported operating systems?

  * Is it better to be an external library rather than in std?

  * What empirical measurements are there to justify the computational benefits of thread affinity?

  * Is there a better API design?
