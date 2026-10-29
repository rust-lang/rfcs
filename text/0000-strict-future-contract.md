- Feature Name: `strict_future_contract`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#4019](https://github.com/rust-lang/rfcs/pull/4019)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Clarify and document the contract requirements of [`Future::poll`], in
particular that a future should be polled or dropped promptly when it requests
a wakeup. In other words, we can cancel a future at any time, but we should
never "pause" or ["snooze"][snooze] a future.

[`Future::poll`]: https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll

There are widely used functions and patterns that violate this "strict"
`Future` contract, including [`select!`]-by-reference and [`StreamExt::next`].
This RFC [identifies several][broken_patterns], but it avoids endorsing any
specific changes beyond the `Future` docs. The goal is to agree that existing
violations are bugs, and that we can and should fix them, but deciding how
exactly to fix or replace each problematic case is left to follow-up RFCs and
the crates ecosystem.

## Motivation
[motivation]: #motivation

Cancellation and pausing are distinct features of async Rust. Regular,
synchronous Rust doesn't let us kill or suspend threads, because doing either
of those things tends to cause non-local deadlocks.[^deprecated] Async
cancellation solves the deadlock problem(!) by dropping cancelled futures,
which automatically releases any locks they might be holding. But async pausing
does not solve the deadlock problem, and that makes it more of a bug than a
feature. For a case study in how difficult and non-local these bugs can be, see
["Futurelock"] (Oxide, October 2025).

[^deprecated]: Lots of languages have old APIs for killing or suspending
    threads that are now deprecated ([Java][java], [C#][c_sharp],
    [Python][python]). The fundamental problem is that killing threads doesn't
    respect language-level cleanup constructs like `try`/`finally` or
    destructors. If we kill a thread in Rust, we leak everything that thread
    owned, including lock guards. Pausing a thread doesn't leak anything per
    se, but if the paused thread is holding a lock, and the thread that's
    supposed to unpause it touches the same lock in the meantime, we get
    similar deadlocks.

[java]: https://docs.oracle.com/javase/8/docs/technotes/guides/concurrency/threadPrimitiveDeprecation.html
[c_sharp]: https://learn.microsoft.com/en-us/dotnet/core/compatibility/core-libraries/5.0/thread-abort-obsolete
[python]: https://docs.python.org/3/c-api/threads.html#c.PyThread_exit_thread

Another problem with async pausing is that, although we almost never do it
explicitly,[^dioxus] we often do it implicitly, and it's surprisingly easy to
do it accidentally. Futurelock was caused by a snoozing mistake [in a `select!`
loop][futurelock_diff], and async streams have been [battling hangs and
deadlocks][barbara] for years. These bugs are invisible unless you know exactly
what you're looking for.

[^dioxus]: The only widely-used counterexample might be the Dioxus framework,
    which [provides a `pause` method][dioxus_docs] and sometimes [calls it
    automatically][dioxus_src].

[dioxus_docs]: https://docs.rs/dioxus/0.7.10/dioxus/prelude/struct.UseFuture.html#method.pause
[dioxus_src]: https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/hooks/src/use_future.rs#L63-L72

Here's a minimized example similar to Futurelock. `main` is trying to drive
`bar` to completion, and while `bar` is running it also wants to call `baz`
every so often. But `bar` and `baz` both call `foo` internally, which takes a
lock. Imagine that `foo`, `bar`, `baz`, and `main` are all defined in different
crates, and that the author of `main` has never heard of `foo` ([playground
link][select_deadlock]):

[select_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Aselect%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++%2F%2F+While+%60bar%60+is+running%2C+call+%60baz%60+every+5+ms.%0A++++let+mut+bar_future+%3D+pin%21%28bar%28%29%29%3B%0A++++loop+%7B%0A++++++++select%21+%7B%0A++++++++++++_+%3D+%26mut+bar_future+%3D%3E+break%2C%0A++++++++++++_+%3D+sleep%28Duration%3A%3Afrom_millis%285%29%29+%3D%3E+%7B%0A++++++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++++++baz%28%29.await%3B%0A++++++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++++++%7D%0A++++++++%7D%0A++++%7D%0A%7D>

```rust
async fn foo() {
    // Acquire a global lock, sleep briefly, and release it.
    static LOCK: Mutex<()> = Mutex::const_new(());
    let _guard = LOCK.lock().await;
    sleep(Duration::from_millis(10)).await;
}

// A couple trivial wrapper functions, to make the deadlock below less "obvious".
async fn bar() {
    foo().await;
}

async fn baz() {
    foo().await;
}

#[tokio::main]
async fn main() {
    // While `bar` is running, call `baz` every 5 ms.
    let mut bar_future = pin!(bar());
    loop {
        select! {
            _ = &mut bar_future => break,
            _ = sleep(Duration::from_millis(5)) => baz().await, // Deadlock!
        }
    }
}
```

Unfortunately, `bar_future` is holding the lock that `baz` wants to acquire.
Either polling it or dropping it would release the lock, but `main` doesn't do
either of those things while it waits on `baz`, so instead it deadlocks.

Let's look closely at who gets polled when. If we [add some prints][squawk], we
can see that `main` gets polled three times:

[squawk]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Aselect%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+Instant%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+main_inner%28%29+%7B%0A++++%2F%2F+While+%60bar%60+is+running%2C+call+%60baz%60+every+5+ms.%0A++++let+mut+bar_future+%3D+pin%21%28bar%28%29%29%3B%0A++++loop+%7B%0A++++++++select%21+%7B%0A++++++++++++_+%3D+%26mut+bar_future+%3D%3E+break%2C%0A++++++++++++_+%3D+sleep%28Duration%3A%3Afrom_millis%285%29%29+%3D%3E+%7B%0A++++++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++++++baz%28%29.await%3B%0A++++++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++++++%7D%0A++++++++%7D%0A++++%7D%0A%7D%0A%0A%2F%2F+Squawk+a+timestamp+every+time+%60future%60+gets+polled.%0Afn+squawk%3CFut%3A+Future%3E%28future%3A+Fut%29+-%3E+impl+Future%3COutput+%3D+Fut%3A%3AOutput%3E+%7B%0A++++let+start+%3D+Instant%3A%3Anow%28%29%3B%0A++++let+mut+future+%3D+Box%3A%3Apin%28future%29%3B%0A++++std%3A%3Afuture%3A%3Apoll_fn%28move+%7Ccx%7C+%7B%0A++++++++let+elapsed+%3D+Instant%3A%3Aelapsed%28%26start%29.as_secs_f32%28%29+*+1000.0%3B%0A++++++++println%21%28%22%5B%7Belapsed%3A.3%7D+ms%5D+POLLED%21%22%29%3B%0A++++++++future.as_mut%28%29.poll%28cx%29%0A++++%7D%29%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++squawk%28main_inner%28%29%29.await%3B%0A%7D>

- 0 ms: Control first enters `main`, `bar`, and the concurrent 5 ms `sleep`.
- 5-6 ms:[^slack] The `sleep` in `main` finishes and invokes its `Waker`,
  `main` gets polled again, and control enters `baz`.
- 10-11 ms: The `sleep` in `bar` finishes and invokes its `Waker`,[^sleep]
  `main` gets polled a third time, and it polls `baz` again.

[^slack]: Tokio's timer implementation adds ~1 ms of slack to our 5 ms timeout
    and our 10 ms sleep.

[^sleep]: This can be confusing: How does the `sleep` in `bar` invoke anything
    if `bar_future` isn't getting polled? It's true that control never reaches
    `bar` again after the second poll at 5 ms, but low-level IO is driven by
    external events, and we arrange for those events to invoke wakers. For
    sleeps and other timers, Tokio runs a ["hashed timer wheel"][tokio_timer]
    in the background, and `sleep` registers its `Waker` there.

[tokio_timer]: https://tokio.rs/blog/2018-03-timers

`main` re-polls `baz` at 10 ms, even though `baz` didn't request a
wakeup.[^also_bar] That's not in and of itself a problem; pending futures are
expected to tolerate extra polling. But `bar` did request a wakeup, and `main`
doesn't poll it. That's a problem. This RFC focuses on deadlocks for clarity,
but missed wakeups can also cause performance issues, timeout errors, and UI
stuttering.

[^also_bar]: There's a 50% chance that `select!` also re-polls `bar_future` at
    5 ms. But the sleep in `bar` hasn't finished at that point, so it returns
    `Pending` again without making any progress. Either way, `select!` drops
    both its arguments after the second arm "wins", so it does not and cannot
    re-poll `bar_future` at 10 ms.

Consider the deadlock problem from the perspective of a programmer writing
`foo`. You're acquiring `LOCK`, and it's your responsibility not to hold it for
too long. If you wait on some IO while you hold it, you're trusting the runtime
to trigger your wakeup correctly. That's fine; you naturally rely on the
runtime for correctness, just like you rely on the standard library and the
compiler. But then, you're also trusting your callers to deliver your wakeup.
Is that fine? What if you're writing library code, and you don't know anything
about your callers?[^spawn_task]

[^spawn_task]: Futures spawned as tasks get their wakeups directly from the
    runtime, so spawning is one way to guarantee our wakeups will arrive
    without trusting unknown callers. But spawning isn't always an option,
    because it isn't supported in all environments, and it's not compatible
    with local borrowing. The [`moro`] crate solves the local borrowing
    problem(!), but its wakeups come through the caller and not directly from
    the runtime, so it doesn't help here.

[`moro`]: https://github.com/nikomatsakis/moro

For async locks to be usable -- or any type that contains one, like a
[`OnceCell`] or a [bounded `mpsc` channel][mpsc] -- you need a guarantee that
callers will either deliver your wakeups or drop you promptly. If that doesn't
happen, it has to be the caller's fault for breaking the rules and not your
fault for trusting them. We need to agree that `main`, not `foo`, is at fault
for the deadlock above, and the `Future` trait needs to document a "strict"
contract to make that clear.

[`OnceCell`]: https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html
[mpsc]: https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html

At the same time, if `main` is broken, we'd strongly prefer to have a warning
or error telling us that. (There had better be a way to fix it too. See [the
rationale section][fixing_main].) It's tempting to blame `select!`, since it's
often found at the scene of these crimes. Should we get our warning in `main`
by deprecating `select!`? In fact that's both too broad and also insufficient.
We can produce similar deadlocks with any form of cancellation. Here's [a
version using `timeout`][timeout_deadlock]. Here's [another version using
`try_join`][try_join_deadlock].

[timeout_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%2C+timeout%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+bar_future+%3D+pin%21%28bar%28%29%29%3B%0A++++_+%3D+timeout%28Duration%3A%3Afrom_millis%285%29%2C+bar_future%29.await%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++baz%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

[try_join_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Afuture%3A%3Atry_join%3B%0Ause+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+-%3E+Result%3C%28%29%2C+%28%29%3E+%7B%0A++++foo%28%29.await%3B%0A++++Ok%28%28%29%29%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+error_in_5ms%28%29+-%3E+Result%3C%28%29%2C+%28%29%3E+%7B%0A++++sleep%28Duration%3A%3Afrom_millis%285%29%29.await%3B%0A++++Err%28%28%29%29%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+bar_future+%3D+pin%21%28bar%28%29%29%3B%0A++++_+%3D+try_join%28bar_future%2C+error_in_5ms%28%29%29.await%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++baz%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

What all these deadlocks have in common is that they depend on [the blanket
`Future` impl for `Pin<&mut _>` references][blanket].[^pin] That impl means a
`Pin<&mut _>` reference to a `Future` is itself a `Future`, except that
dropping it has no effect. If the strict `Future` contract requires us to drop
cancelled futures promptly, then that impl is broken, and we should deprecate
it. There's our warning in `main`.

[^pin]: `pin!` is arguably also a red flag, but pinning per se doesn't have
    anything to do with control flow or wakeups. We often manage heterogeneous
    futures with `Pin<Box<dyn Future>>`, and we can [abuse one of those][boxed]
    to replace `pin!` in this example.

[boxed]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3AFutureExt%3B%0Ause+tokio%3A%3Aselect%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++%2F%2F+While+%60bar%60+is+running%2C+call+%60baz%60+every+5+ms.%0A++++let+mut+bar_future+%3D+bar%28%29.boxed%28%29%3B%0A++++loop+%7B%0A++++++++select%21+%7B%0A++++++++++++_+%3D+%26mut+bar_future+%3D%3E+break%2C%0A++++++++++++_+%3D+sleep%28Duration%3A%3Afrom_millis%285%29%29+%3D%3E+%7B%0A++++++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++++++baz%28%29.await%3B%0A++++++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++++++%7D%0A++++++++%7D%0A++++%7D%0A%7D>

However, this RFC doesn't propose deprecating that blanket impl today, for
several reasons:

- Rust doesn't currently support deprecating trait impls.
- It's a broad impl that also covers e.g. `Pin<Box<dyn Future>>`, which we
  certainly don't want to deprecate.
- When we rework existing code to avoid `Pin<&mut _>`, we often need [helper
  functions that don't currently exist][fixing_main].

All of those problems are solvable, but designing and rolling out new helper
functions will take months or years, and it needs to happen in the crates
ecosystem rather than in the standard library. Also, this ["cancellation by
reference"][cancellation_by_reference] pattern isn't the only way to violate
the strict `Future` contract today. [`AsyncIterator`] and [`Stream`] have their
own deadlock bugs, and we'll need [at least one follow-up
RFC][poll_progress_rfc] to address those. See below for [a list of broken
patterns][broken_patterns].

Instead of trying to fix everything everywhere all at once, this RFC proposes
the smallest possible change: Document the strict `Future` contract. Once we
agree on where these bugs are coming from, we can start the long and gradual
process of fixing them.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

> This RFC proposes adding the following text (or similar) to the [`Future`]
> docs.

**When a pending `Future` requests a wakeup, its owner must either `poll` it
promptly or drop it.**

Unlike threads, which have a life of their own after they start running, a
future only makes progress when it's polled. If you stop polling a future for a
while, you can effectively pause its execution. However, the "poll promptly"
rule forbids this. Once you've polled a future and given it a `Waker`, you're
required to poll it again when it invokes that `Waker`.[^unknown] This
requirement repeats until `poll` returns `Ready` or you drop the future.
Dropping a pending future because you don't want to poll it anymore is called
"cancellation".

[^unknown]: Often we don't know which of our children triggered a given wakeup,
    so we need to poll all of them every time we're polled. Alternatively, it's
    possible to know which of our children triggered a wakeup if we give each
    of them a unique `Waker`. [`FuturesUnordered`] does this, as do most
    runtimes at the task level.

Here's an example of an incorrect future adapter that breaks this rule. Every
time `CoinFlip` is polled, it flips a coin to decide whether to poll its child:

```rust
struct CoinFlip<Fut> {
    inner: Pin<Box<Fut>>,
}

impl<Fut: Future> Future for CoinFlip<Fut> {
    type Output = Fut::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Fut::Output> {
        if rand::random() {
            self.inner.as_mut().poll(cx)
        } else {
            // XXX: `inner` might have requested a wakeup. Returning without polling it here
            // violates the `Future` contract.
            Poll::Pending
        }
    }
}
```

The problem is that `random()` might be true the first time, polling `inner`
and letting it register wakeups, but then it might be false the second time
when one of those wakeups triggers, failing to poll `inner`
promptly.[^first_time] This tends to cause hangs and deadlocks, not only in the
future that didn't get polled (which we might've intended), but also in distant
and unrelated futures that happen to use the same shared resources ([playground
link][coin_flip]). `CoinFlip` would be at fault for those bugs.

[^first_time]: On the other hand, if `random` is false the first time, we might
    never poll `Fut`. Whether that's acceptable according to the `Future`
    contract is an open question. See [the unresolved questions
    section](#should-we-allow-an-indefinite-delay-between-creation-and-polling).

[coin_flip]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+std%3A%3Apin%3A%3APin%3B%0Ause+std%3A%3Atask%3A%3A%7BContext%2C+Poll%7D%3B%0Ause+tokio%3A%3Aselect%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Astruct+CoinFlip%3CFut%3E+%7B%0A++++inner%3A+Pin%3CBox%3CFut%3E%3E%2C%0A%7D%0A%0Aimpl%3CFut%3A+Future%3E+CoinFlip%3CFut%3E+%7B%0A++++fn+new%28inner%3A+Fut%29+-%3E+Self+%7B%0A++++++++Self+%7B%0A++++++++++++inner%3A+Box%3A%3Apin%28inner%29%2C%0A++++++++%7D%0A++++%7D%0A%7D%0A%0Aimpl%3CFut%3A+Future%3E+Future+for+CoinFlip%3CFut%3E+%7B%0A++++type+Output+%3D+Fut%3A%3AOutput%3B%0A%0A++++fn+poll%28mut+self%3A+Pin%3C%26mut+Self%3E%2C+cx%3A+%26mut+Context%29+-%3E+Poll%3CFut%3A%3AOutput%3E+%7B%0A++++++++if+rand%3A%3Arandom%28%29+%7B%0A++++++++++++self.inner.as_mut%28%29.poll%28cx%29%0A++++++++%7D+else+%7B%0A++++++++++++%2F%2F+XXX%3A+%60inner%60+might+have+requested+a+wakeup.+Returning+without+polling+it+here%0A++++++++++++%2F%2F+violates+the+%60Future%60+contract.%0A++++++++++++Poll%3A%3APending%0A++++++++%7D%0A++++%7D%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+mut+iteration+%3D+0%3B%0A++++loop+%7B%0A++++++++iteration+%2B%3D+1%3B%0A++++++++dbg%21%28iteration%29%3B%0A++++++++%2F%2F+This+deadlocks+25%25+of+the+time%2C+so+we+run+it+in+a+loop.+We+need+three+things+to+happen%3A%0A++++++++%2F%2F+++1.+%60select%21%60+polls+%60CoinFlip%60+first.+The+%60biased%60+keyword+guarantees+this.%0A++++++++%2F%2F+++2.+The+first+poll+of+%60CoinFlip%60+flips+%60true%60%2C+so+%60bar%60+acquires+%60LOCK%60.%0A++++++++%2F%2F+++3.+The+second+poll+of+%60CoinFlip%60+flips+%60false%60%2C+so+%60bar%60+never+releases+%60LOCK%60.%0A++++++++select%21+%7B%0A++++++++++++biased%3B%0A++++++++++++_+%3D+CoinFlip%3A%3Anew%28bar%28%29%29+%3D%3E+%7B%7D%0A++++++++++++_+%3D+baz%28%29+%3D%3E+%7B%7D+%2F%2F+Maybe+deadlock%21%0A++++++++%7D%0A++++%7D%0A%7D>

If we don't want to poll `inner` here, the `Future` contract requires us to
cancel it. We could wrap it in an `Option` or similar and clear it in the
`else` branch. That makes a `false` coin flip permanent, which might not be
what we wanted when we wrote `CoinFlip`, but there's no correct way to make it
temporary.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### What exactly does "promptly" mean?

The usual expected behavior is that an implementation of `poll` should call
`poll` on all of its child futures (if any) before returning. However, there
are other valid ways to do things. Some async runtimes may [track an execution
"budget"][budget], such that some of their futures return `Pending` earlier
than usual once that budget is used up. In that case they arrange to get polled
again as soon as other tasks have had a chance to run. Those other tasks'
`poll` functions are required to return quickly, so any work deferred to the
re-poll can still be considered prompt. Some futures might also offload their
work to other threads, and their `poll` function might return without waiting
for a completion notification from those threads.

[budget]: https://tokio.rs/blog/2020-04-preemption

What these cases have in common, though, is that barring program exit or the
power going out, future progress is guaranteed. There's no legitimate way for
user code to stop the runtime from working through its task list or freeze a
private worker thread.[^illegitimate] The deadlocks above are different: a
future is suspended across some arbitrary bit of user code that isn't
guaranteed to ever finish.

[^illegitimate]: _Illegitimate_ ways to interfere with the runtime include
    synchronous blocking in `poll`, calling unsafe functions like
    [`pthread_cancel`], or just corrupting memory. That said, the [userspace
    executors section][userspace_executors] does include a corner case where
    safe and reasonable-looking user code can pause a whole single-threaded
    `Runtime`. That might be worth preventing somehow, or it might be
    considered low-level enough that documenting it is sufficient.

A fully formal definition of "promptly" will probably end up with somewhat
unsatisfying wording like "a finite period of time". Consider this excerpt from
[the C++ atomic memory model][finite]:

> An implementation should ensure that the last value (in modification order)
> assigned by an atomic or synchronization operation will become visible to all
> other threads **in a finite period of time**.

[finite]: https://github.com/cplusplus/draft/blob/4358c6f6856ac8b392b601f820bce7bb134cbeed/source/basic.tex#L7291-L7293

The atomic memory model is concerned with operations that complete in
_nanoseconds_, but it's hard to give a specific number of nanoseconds, or even
a more abstract bound like "ticks" or "steps", without getting into details of
the hardware that the standard doesn't want to constrain. Similarly, async Rust
needs to accommodate all sorts of runtimes, evented IO models, and OS
environments, and that makes it hard for the formal, general rules to say much
about anything that goes on under the hood. Nonetheless, using terms like
"promptly" in our documentation is useful for building intuition about how
async programs work, and for helping different layers of the stack understand
each other's intent.

## Broken patterns
[broken_patterns]: #broken-patterns

There are many functions, types, and patterns in async Rust today that can
violate the strict `Future` contract. The following is an incomplete list of
these with example deadlocks for each. This list gets its own section, because
it's several things at once: it's a rationale (the problem is widespread), a
drawback (fixing it will be a lot of churn), and a catalog of future work (this
RFC doesn't prescribe specific fixes).

#### Cancellation by reference
[cancellation_by_reference]: #cancellation-by-reference

The `select!` deadlock [discussed above][motivation], and the `timeout` and
`try_join` versions from the same section, are all examples of what we might
call "cancellation by reference". We can cause a deadlock anywhere cancellation
occurs by having the cancelled future hold a lock and having its owner drive it
by reference ([playground link][timeout_deadlock]):

```rust
let bar_future = pin!(bar());
_ = timeout(Duration::from_millis(5), bar_future).await;
baz().await; // Deadlock!
```

Often in these situations, the programmer doesn't actually intend to cancel
anything, and instead they're (ab)using `select!`/`timeout`/etc. for _pausing_.
The problem is that those primitives rely on cancellation for correctness.

Since `Stream` and `AsyncIterator` both have a [similar blanket impl for
`Pin<&mut _>`][async_iter_blanket], streams can also cause this category of
deadlocks ([playground link][take_until_deadlock]):

[async_iter_blanket]: https://doc.rust-lang.org/std/async_iter/trait.AsyncIterator.html#impl-AsyncIterator-for-Pin%3CP%3E

[take_until_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Astream%3A%3A%7Bself%2C+StreamExt+as+_%7D%3B%0Ause+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+bar_stream+%3D+pin%21%28stream%3A%3Aonce%28bar%28%29%29%29%3B%0A++++bar_stream%0A++++++++.take_until%28sleep%28Duration%3A%3Afrom_millis%285%29%29%29%0A++++++++.for_each%28async+%7C_%7C+%7B%7D%29%0A++++++++.await%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++baz%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

```rust
let bar_stream = pin!(stream::once(bar()));
bar_stream
    .take_until(sleep(Duration::from_millis(5)))
    .for_each(async |_| {})
    .await;
baz().await; // Deadlock!
```

#### `StreamExt::next`

The [`.next()`][`StreamExt::next`] method on streams borrows the underlying
stream, so cancelling the [`Next`] future is effectively
cancellation-by-reference for the stream itself ([playground
link][cancelled_next_deadlock]):

[`Next`]: https://docs.rs/futures/latest/futures/stream/struct.Next.html

[cancelled_next_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Astream%3A%3A%7Bself%2C+StreamExt%7D%3B%0Ause+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%2C+timeout%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+mut+bar_stream+%3D+pin%21%28stream%3A%3Aonce%28bar%28%29%29%29%3B%0A++++_+%3D+timeout%28Duration%3A%3Afrom_millis%285%29%2C+bar_stream.next%28%29%29.await%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++baz%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

```rust
let mut bar_stream = pin!(stream::once(bar()));
_ = timeout(Duration::from_millis(5), bar_stream.next()).await;
baz().await; // Deadlock!
```

#### Concurrent streams

After a concurrent stream yields an item, nothing drives its other child
streams until the next item is requested. Here we'll use [`merge`] for
concurrency ([playground link][concurrent_for_each_deadlock]), but we could do
the same thing with [`buffered`] streams ([playground link][buffered_deadlock])
or [`FuturesUnordered`] ([playground link][futures_unordered_deadlock]):[^fair]

[`merge`]: https://docs.rs/tokio-stream/latest/tokio_stream/trait.StreamExt.html#method.merge
[`buffered`]: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.buffered
[`for_each`]: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.for_each

[^fair]: Note that in this case (though not in any of the examples before
    this), the deadlock happens because the lock we're using is "fair". The
    `bar` future on the right side of the merge has taken a "place in line" to
    acquire the lock, and even though it never actually gets a chance to
    acquire it, it still blocks `baz` from acquiring it later. If we want to
    produce the same deadlock with an unfair mutex, we need `foo` or `bar` to
    do a short extra sleep after unlocking, to give other callers a chance to
    acquire the lock ([playground link][unfair_mutex]).

```rust
stream::once(bar())
    .merge(stream::once(bar()))
    .for_each(async |_| {
        baz().await; // Deadlock!
    })
    .await;
```

[concurrent_for_each_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Astream%3A%3A%7Bself%2C+StreamExt+as+_%7D%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0Ause+tokio_stream%3A%3AStreamExt+as+_%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++stream%3A%3Aonce%28bar%28%29%29%0A++++++++.merge%28stream%3A%3Aonce%28bar%28%29%29%29%0A++++++++.for_each%28async+%7C_%7C+%7B%0A++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++baz%28%29.await%3B%0A++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++%7D%29%0A++++++++.await%3B%0A%7D>

[buffered_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3A%7BStreamExt%2C+stream%7D%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++stream%3A%3Aiter%28vec%21%5Bbar%28%29%2C+bar%28%29%5D%29%0A++++++++.buffered%282%29%0A++++++++.for_each%28async+%7C_%7C+%7B%0A++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++baz%28%29.await%3B%0A++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++%7D%29%0A++++++++.await%3B%0A%7D>

[futures_unordered_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Astream%3A%3A%7BFuturesUnordered%2C+StreamExt%7D%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+futures+%3D+FuturesUnordered%3A%3Anew%28%29%3B%0A++++futures.push%28bar%28%29%29%3B%0A++++futures.push%28bar%28%29%29%3B%0A++++futures%0A++++++++.for_each%28async+%7C_%7C+%7B%0A++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++baz%28%29.await%3B%0A++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++%7D%29%0A++++++++.await%3B%0A%7D>

[unfair_mutex]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Alock%3A%3AMutex%3B%0Ause+futures%3A%3Astream%3B%0Ause+futures%3A%3Astream%3A%3AStreamExt+as+_%3B%0Ause+std%3A%3Apin%3A%3Apin%3B%0Ause+std%3A%3Async%3A%3ALazyLock%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0Ause+tokio_stream%3A%3AStreamExt+as+_%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+This+mutex+implementation+is+unfair%21+%28It+also+doesn%27t+have+a+const+new+function%2C+so+we+need%0A++++%2F%2F+a+%60LazyLock%60+to+initialize+it.%29%0A++++static+LOCK%3A+LazyLock%3CMutex%3C%28%29%3E%3E+%3D+LazyLock%3A%3Anew%28%7C%7C+Mutex%3A%3Anew%28%28%29%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A++++%2F%2F+This+second+sleep+allows+the+second+%60bar%60+to+acquire+the+lock+before+the+first+%60bar%60%0A++++%2F%2F+finishes+below.+If+you+comment+it+out%2C+the+deadlock+no+longer+appears.%0A++++sleep%28Duration%3A%3Afrom_millis%285%29%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+bar_stream1+%3D+stream%3A%3Aonce%28bar%28%29%29%3B%0A++++let+bar_stream2+%3D+stream%3A%3Aonce%28bar%28%29%29%3B%0A++++let+merged_stream+%3D+pin%21%28bar_stream1.merge%28bar_stream2%29%29%3B%0A++++merged_stream%0A++++++++.for_each%28async+%7C_%7C+%7B%0A++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++baz%28%29.await%3B%0A++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++%7D%29%0A++++++++.await%3B%0A%7D>

#### `futures::future::select`

The [`select`][select_fn] _function_ (not the macro) returns the losing future
by value instead of dropping it. We can cause a deadlock if we retain it
([playground link][select_fn_deadlock]):

[select_fn]: https://docs.rs/futures/latest/futures/future/fn.select.html

[select_fn_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3AFutureExt%3B%0Ause+std%3A%3Afuture%3A%3Aready%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+%28_%2C+bar_future%29+%3D+futures%3A%3Afuture%3A%3Aselect%28bar%28%29.boxed%28%29%2C+ready%28%28%29%29%29%0A++++++++.await%0A++++++++.factor_first%28%29%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++baz%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A++++bar_future.await%3B%0A%7D>

```rust
let (_, bar_future) = futures::future::select(bar().boxed(), ready(()))
    .await
    .factor_first();
baz().await; // Deadlock!
bar_future.await;
```

#### `FutureExt::shared`

The [`.shared()`][shared] method lets us drive a future behind an `Arc`, which
acts like a shared reference. We can use it to reproduce the
cancellation-by-reference deadlock above ([playground link][shared_deadlock]):

[shared]: https://docs.rs/futures/latest/futures/future/trait.FutureExt.html#method.shared

[shared_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Afuture%3A%3AFutureExt%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%2C+timeout%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+bar_future+%3D+bar%28%29.shared%28%29%3B%0A++++_+%3D+timeout%28Duration%3A%3Afrom_millis%285%29%2C+bar_future.clone%28%29%29.await%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++baz%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

```rust
let bar_future = bar().shared();
_ = timeout(Duration::from_millis(5), bar_future.clone()).await;
baz().await; // Deadlock!
```

#### Userspace executors
[userspace_executors]: #userspace-executors

[`FuturesUnordered`] is both a concurrent `Stream` and an executor, and those
two things turn out to have a lot in common. Much like we were able to
reproduce the stream deadlocks above using `FuturesUnordered` ([playground
link][futures_unordered_deadlock]), we can also produce similar deadlocks using
other executors. Here's Tokio's [`LocalSet`] ([playground
link][localset_deadlock]):

[`LocalSet`]: https://docs.rs/tokio/latest/tokio/task/struct.LocalSet.html

[localset_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+local+%3D+tokio%3A%3Atask%3A%3ALocalSet%3A%3Anew%28%29%3B%0A++++local.spawn_local%28bar%28%29%29%3B%0A++++local.run_until%28sleep%28Duration%3A%3Afrom_millis%285%29%29%29.await%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++baz%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

```rust
let local = tokio::task::LocalSet::new();
local.spawn_local(bar());
local.run_until(sleep(Duration::from_millis(5))).await;
baz().await; // Deadlock!
```

We can also do this with the Tokio [`Runtime`] itself. We need the
single-threaded version, so that `bar` doesn't keep running in the background
([playground link][runtime_deadlock]):

[`Runtime`]: https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html

[runtime_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+tokio%3A%3Aruntime%3A%3ARuntime%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Afn+main%28%29+%7B%0A++++%2F%2F+Build+a+single-threaded+runtime.+If+we+used+%60new_multi_thread%60+or+%60Runtime%3A%3Anew%60+here%2C+then%0A++++%2F%2F+%60bar%60+would+keep+running+after+the+first+%60block_on%60+returned%2C+and+we+wouldn%27t+get+a%0A++++%2F%2F+deadlock.+See%3A+https%3A%2F%2Fdocs.rs%2Ftokio%2F1.53.1%2Ftokio%2Fruntime%2F%23driving-the-runtime%0A++++let+runtime+%3D+tokio%3A%3Aruntime%3A%3ABuilder%3A%3Anew_current_thread%28%29%0A++++++++.enable_time%28%29%0A++++++++.build%28%29%0A++++++++.unwrap%28%29%3B%0A++++runtime.spawn%28bar%28%29%29%3B%0A++++runtime.block_on%28async+%7B%0A++++++++sleep%28Duration%3A%3Afrom_millis%285%29%29.await%3B%0A++++%7D%29%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++Runtime%3A%3Anew%28%29.unwrap%28%29.block_on%28baz%28%29%29%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

```rust
fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    runtime.spawn(bar());
    runtime.block_on(async {
        sleep(Duration::from_millis(5)).await;
    });
    Runtime::new().unwrap().block_on(baz()); // Deadlock!
}
```

#### `poll!`

The [`poll!`] macro is somewhat low-level, but it's arguably the most direct
way to produce these deadlocks ([playground link][poll_deadlock]):

[`poll!`]: https://docs.rs/futures/latest/futures/macro.poll.html

[poll_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Apoll%3B%0Ause+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+bar_future+%3D+pin%21%28bar%28%29%29%3B%0A++++_+%3D+poll%21%28bar_future%29%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++baz%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

```rust
let bar_future = pin!(bar());
_ = poll!(bar_future);
baz().await; // Deadlock!
```

Streams have a similar helper, [`poll_immediate`]
([playground link][poll_immediate_deadlock]):

[`poll_immediate`]: https://docs.rs/futures/latest/futures/stream/fn.poll_immediate.html

[poll_immediate_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Astream%3A%3Apoll_immediate%3B%0Ause+futures%3A%3A%7BStreamExt%2C+stream%7D%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++poll_immediate%28stream%3A%3Aonce%28bar%28%29%29%29%0A++++++++.for_each%28%7C_%7C+async+%7B%0A++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++baz%28%29.await%3B%0A++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++%7D%29%0A++++++++.await%3B%0A%7D>

```rust
poll_immediate(stream::once(bar()))
    .for_each(|_| async {
        baz().await; // Deadlock!
    })
    .await;
```

## Drawbacks
[drawbacks]: #drawbacks

### A _lot_ of existing code snoozes futures

The [list of broken patterns above][broken_patterns] is long. Tons of existing
async Rust code uses these patterns, and most of it will need changes to the
caller as part of a fix. Even if we only add warnings for most of these, that's
a lot of proposed churn.

### Pausing things is useful, and it would've been nice to allow it.

Some applications might want to pause low-priority work when load is high.
Others might have an actual pause button (e.g. games, media) that they'd like
to implement by pausing futures. Those might not be our recommended
architectural choices, but effectively forbidding them at the language level
seems quite opinionated.[^forbid]

[^forbid]: Of course applications can ultimately do whatever they like,
    including pausing futures or killing threads. Part of what's at stake here
    is the question of who's at fault when such an application collides with a
    library ecosystem that uses locks.

Similarly, Windows has [`SuspendThread`] and [`TerminateThread`], and Unix has
[`pthread_cancel`], because many applications over the years have wanted to
non-cooperatively pause or cancel running threads.[^raymond_chen1] That's
understandable; passing a cancel flag around everywhere is inconvenient at
best, and it can be impossible when we're working with other people's code.
However, today we understand that those functions are _radioactive_. Outside of
a short list of low-level use cases, they tend to corrupt the entire
process.[^raymond_chen2] We generally ban them.

[^raymond_chen1]: "Originally, there was no `TerminateThread` function. The
    original designers felt strongly that no such function should exist because
    there was no safe way to terminate a thread, and there's no point having a
    function that cannot be called safely. But people screamed that they needed
    the `TerminateThread` function, even though it wasn't safe, so the
    operating system designers caved and added the function because people
    demanded it. Of course, those people who insisted that they needed
    `TerminateThread` now regret having been given it." - [Raymond
    Chen][raymond_chen1]

[raymond_chen1]: https://devblogs.microsoft.com/oldnewthing/20150814-00/?p=91811

[^raymond_chen2]: "These results are not specific to C#. The same logic applies
    to Win32 or any other threading model. In Win32, the process heap is a
    threadsafe object, and since it's hard to do very much in Win32 at all
    without accessing the heap, suspending a thread in Win32 has a very high
    chance of deadlocking your process." - [Raymond Chen][raymond_chen2]

[raymond_chen2]: https://devblogs.microsoft.com/oldnewthing/20031209-00/?p=41573

Unfortunately, pausing futures in async Rust has all the same problems. Taking
an async lock is far less common than allocating memory or printing, so the
symptoms aren't as noticeable today, but they'll get worse as the ecosystem
grows and private locks appear in more places. In [the original Futurelock
incident][incident], the culprit was a semaphore buried in the
`tokio::sync::mpsc` channel implementation. The channel in question wasn't even
visible at the point where pausing happened. The non-local nature of these bugs
forces us to take a position on pausing at the language/ecosystem level. (See
also ["Could we statically identify futures that hold locks?"][statically])

[incident]: https://github.com/oxidecomputer/omicron/issues/9259

Given all that, it's remarkable that non-cooperative[^noncoop] cancellation
works as well as it does in async Rust. [It has its
issues,][cancelling_async_rust] but it doesn't generally cause deadlocks, and
many applications use selects and timeouts routinely in production. That's
quite an achievement, and maybe an unexpected benefit of destructor-based
cleanup and by extension the borrow checker.

[^noncoop]: "Cooperative" vs "non-cooperative" has a couple different
    interpretations here. From the perspective of an executor thread that's
    calling `Future::poll`, everything is cooperative, because we can't force
    that function to ever return. On the other hand, a `poll` function that
    doesn't return promptly is gumming up the executor, and we have [tools for
    finding those][slow_poll]. If we take it for granted that every long `poll`
    bug eventually gets fixed, then we could think of cancellation in async
    Rust as _non_-cooperative. There's nothing a correct `async fn` can do to
    prevent it or delay it for very long.

[slow_poll]: https://docs.rs/tokio-metrics/latest/tokio_metrics/struct.TaskMonitor.html#method.with_slow_poll_threshold

[cancelling_async_rust]: https://sunshowers.io/posts/cancelling-async-rust/

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Fixing our broken `main` function
[fixing_main]: #fixing-our-broken-main-function

Here's the `select!` deadlock [we started with above][motivation] ([playground
link][select_deadlock]):

```rust
// While `bar` is running, call `baz` every 5 ms.
let mut bar_future = pin!(bar());
loop {
    select! {
        _ = &mut bar_future => break,
        _ = sleep(Duration::from_millis(5)) => baz().await, // Deadlock!
    }
}
```

To fix this, we'd like to factor out the `baz` loop into its own `async` block
and run it concurrently. We can't [`join`] that block with `bar`, though,
because it never returns. One option here is to reorganize our `select!` to put
the loop on the inside ([playground
link][select_baz_loop]):[^cancellation_token]

[`join`]: https://docs.rs/futures/latest/futures/future/fn.join.html

[^cancellation_token]: Another widely-used option is a [`CancellationToken`],
    but that's easier to get wrong, and it also uses heap allocation
    internally.

[`CancellationToken`]: https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html

[select_baz_loop]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+tokio%3A%3Aselect%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++%2F%2F+While+%60bar%60+is+running%2C+call+%60baz%60+every+5+ms.%0A++++let+baz_loop+%3D+async+%7B%0A++++++++loop+%7B%0A++++++++++++sleep%28Duration%3A%3Afrom_millis%285%29%29.await%3B%0A++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++baz%28%29.await%3B%0A++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++%7D%0A++++%7D%3B%0A++++select%21+%7B%0A++++++++_+%3D+bar%28%29+%3D%3E+%7B%7D%2C%0A++++++++_+%3D+baz_loop+%3D%3E+%7B%7D%2C%0A++++%7D%0A++++println%21%28%22...and+then+we+exit.%22%29%3B%0A%7D>

```rust
// While `bar` is running, call `baz` every 5 ms.
let baz_loop = async {
    loop {
        sleep(Duration::from_millis(5)).await;
        baz().await;
    }
};
select! {
    _ = bar() => {},
    _ = baz_loop => {},
}
```

This works, and it's nice (and not a coincidence) that it doesn't require
`pin!`. But a downside of this approach is that it looks like we're waiting for
either `bar` or the `baz_loop` to finish. We know that the `baz_loop` will
never finish, so the resulting behavior is correct, but `select!` doesn't
really capture our intent. It's also slightly awkward if we need the return
value of `bar`.

We could imagine a new helper function that fits this problem better.[^roll] It
would drive two futures concurrently but only wait for its first argument to
finish. Let's call it `join_maybe`:

[^roll]: This helper isn't available in any widely-used library crates today,
    but [sometimes programmers roll their own][roll].

[roll]: https://old.reddit.com/r/rust/comments/1risdcd/never_snooze_a_future/o89qor8/

```rust
/// Run a "definitely" future and a "maybe" future concurrently. Wait for the definitely future to
/// finish. If the maybe future is still pending at that point, cancel it. Return the definitely
/// output together with the maybe output if it wasn't cancelled.
async fn join_maybe<Definitely: Future, Maybe: Future>(
    definitely: Definitely,
    maybe: Maybe,
) -> (Definitely::Output, Option<Maybe::Output>)
```

Here's what the body of our `main` function would look like using `join_maybe`
instead of `select!` ([playground link][join_maybe]):

[join_maybe]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Afuture%3A%3AMaybeDone%3B%0Ause+pin_project_lite%3A%3Apin_project%3B%0Ause+std%3A%3Apin%3A%3APin%3B%0Ause+std%3A%3Atask%3A%3A%7BContext%2C+Poll%7D%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%2F%2F%2F+Run+a+%22definitely%22+future+and+a+%22maybe%22+future+concurrently.+Wait+for+the+definitely+future+to%0A%2F%2F%2F+finish.+If+the+maybe+future+is+still+pending+at+that+point%2C+cancel+it.+Return+the+definitely%0A%2F%2F%2F+output+together+with+the+maybe+output+if+it+wasn%27t+cancelled.%0Afn+join_maybe%3CDefinitely%3A+Future%2C+Maybe%3A+Future%3E%28%0A++++definitely%3A+Definitely%2C%0A++++maybe%3A+Maybe%2C%0A%29+-%3E+JoinMaybe%3CDefinitely%2C+Maybe%3E+%7B%0A++++JoinMaybe+%7B%0A++++++++definitely%2C%0A++++++++maybe%3A+MaybeDone%3A%3AFuture%28maybe%29%2C%0A++++%7D%0A%7D%0A%0Apin_project%21+%7B%0A++++struct+JoinMaybe%3CDefinitely%3A+Future%2C+Maybe%3A+Future%3E+%7B%0A++++++++%23%5Bpin%5D%0A++++++++definitely%3A+Definitely%2C%0A++++++++%23%5Bpin%5D%0A++++++++maybe%3A+MaybeDone%3CMaybe%3E%2C%0A++++%7D%0A%7D%0A%0Aimpl%3CDefinitely%3A+Future%2C+Maybe%3A+Future%3E+Future+for+JoinMaybe%3CDefinitely%2C+Maybe%3E+%7B%0A++++type+Output+%3D+%28Definitely%3A%3AOutput%2C+Option%3CMaybe%3A%3AOutput%3E%29%3B%0A%0A++++fn+poll%28self%3A+Pin%3C%26mut+Self%3E%2C+cx%3A+%26mut+Context%29+-%3E+Poll%3CSelf%3A%3AOutput%3E+%7B%0A++++++++let+mut+this+%3D+self.project%28%29%3B%0A++++++++let+definitely_poll+%3D+this.definitely.poll%28cx%29%3B%0A++++++++_+%3D+this.maybe.as_mut%28%29.poll%28cx%29%3B%0A++++++++if+let+Poll%3A%3AReady%28definitely_output%29+%3D+definitely_poll+%7B%0A++++++++++++let+maybe_output+%3D+this.maybe.as_mut%28%29.take_output%28%29%3B%0A++++++++++++%2F%2F+NB%3A+The+strict+Future+contract+doesn%27t+currently+guarantee+that+our+caller+will+drop%0A++++++++++++%2F%2F+us+promptly+after+we+return+Ready%2C+so+if+the+maybe+future+is+still+pending%2C+we+need%0A++++++++++++%2F%2F+to+explicitly+cancel+it.%0A++++++++++++this.maybe.set%28MaybeDone%3A%3AGone%29%3B%0A++++++++++++Poll%3A%3AReady%28%28definitely_output%2C+maybe_output%29%29%0A++++++++%7D+else+%7B%0A++++++++++++Poll%3A%3APending%0A++++++++%7D%0A++++%7D%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++%2F%2F+While+%60bar%60+is+running%2C+call+%60baz%60+every+5+ms.%0A++++let+baz_loop+%3D+async+%7B%0A++++++++loop+%7B%0A++++++++++++sleep%28Duration%3A%3Afrom_millis%285%29%29.await%3B%0A++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++baz%28%29.await%3B%0A++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++%7D%0A++++%7D%3B%0A++++join_maybe%28bar%28%29%2C+baz_loop%29.await%3B%0A++++println%21%28%22...and+then+we+exit.%22%29%3B%0A%7D>

```rust
// While `bar` is running, call `baz` every 5 ms.
let baz_loop = async {
    loop {
        sleep(Duration::from_millis(5)).await;
        baz().await;
    }
};
join_maybe(bar(), baz_loop).await;
```

That's a bit cleaner. But note that the `select!` macro is [extremely
flexible](#concurrency-syntax),[^flexible] and this is just one of its simplest
use cases. There's a wide open design space for other helpers that mix joining,
racing, cancellation, shared mutability, error handling, and `no_std` support
in different ways. For an example of another macro in this space, see
[`join_me_maybe::join!`][join_me_maybe].

[join_me_maybe]: https://docs.rs/join_me_maybe/latest/join_me_maybe/

[^flexible]: Some of the flexibility of `select!` depends on driving futures by
    reference and violating the strict `Future` contract. If we don't allow
    that, `select!` loses a lot of its flexibility.

### Could we statically identify futures that hold locks?
[statically]: #could-we-statically-identify-futures-that-hold-locks

Not reliably, no. Most futures don't currently hold locks across await points,
and it might've been nice to allow pausing the ones that don't, as long as we
could reliably identify the ones that do.[^onerous] Unfortunately, "lock
nature" isn't limited to a handful of primitive types. Any use of atomics might
turn out to be a synchronous spinlock, and we can build an async lock out of
any synchronous lock. A quick-and-dirty implementation [looks like
this][my_lock].

[^onerous]: Note that the set of "regular functions that never take regular
    locks" is small, because among other things `malloc` can take a lock. Most
    of the async ecosystem is async-lock-free for now, but it might not stay
    that way forever.

[my_lock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Afuture%3A%3Ajoin_all%3B%0Ause+std%3A%3Acell%3A%3AUnsafeCell%3B%0Ause+std%3A%3Amarker%3A%3APhantomData%3B%0Ause+std%3A%3Aops%3A%3A%7BDeref%2C+DerefMut%7D%3B%0Ause+std%3A%3Async%3A%3Aatomic%3A%3A%7BAtomicBool%2C+Ordering%7D%3B%0Ause+std%3A%3Atask%3A%3A%7BPoll%2C+Waker%7D%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Astruct+Spinlock%3CT%3E+%7B%0A++++value%3A+UnsafeCell%3CT%3E%2C%0A++++locked%3A+AtomicBool%2C%0A%7D%0A%0Astruct+SpinlockGuard%3C%27a%2C+T%3E+%7B%0A++++lock%3A+%26%27a+Spinlock%3CT%3E%2C%0A++++_marker%3A+PhantomData%3CT%3E%2C+%2F%2F+Sync+if+T+is+Sync%0A%7D%0A%0Aimpl%3CT%3E+Spinlock%3CT%3E+%7B%0A++++const+fn+new%28value%3A+T%29+-%3E+Self+%7B%0A++++++++Self+%7B%0A++++++++++++value%3A+UnsafeCell%3A%3Anew%28value%29%2C%0A++++++++++++locked%3A+AtomicBool%3A%3Anew%28false%29%2C%0A++++++++%7D%0A++++%7D%0A%0A++++fn+lock%28%26self%29+-%3E+SpinlockGuard%3C%27_%2C+T%3E+%7B%0A++++++++while+self.locked.swap%28true%2C+Ordering%3A%3AAcquire%29+%7B%0A++++++++++++std%3A%3Ahint%3A%3Aspin_loop%28%29%3B%0A++++++++%7D%0A++++++++SpinlockGuard+%7B%0A++++++++++++lock%3A+self%2C%0A++++++++++++_marker%3A+PhantomData%2C%0A++++++++%7D%0A++++%7D%0A%7D%0A%0Aunsafe+impl%3CT%3E+Send+for+Spinlock%3CT%3E+where+T%3A+Send+%7B%7D%0Aunsafe+impl%3CT%3E+Sync+for+Spinlock%3CT%3E+where+T%3A+Send+%7B%7D%0A%0Aimpl%3CT%3E+Drop+for+SpinlockGuard%3C%27_%2C+T%3E+%7B%0A++++fn+drop%28%26mut+self%29+%7B%0A++++++++self.lock.locked.store%28false%2C+Ordering%3A%3ARelease%29%3B%0A++++%7D%0A%7D%0A%0Aimpl%3CT%3E+Deref+for+SpinlockGuard%3C%27_%2C+T%3E+%7B%0A++++type+Target+%3D+T%3B%0A%0A++++fn+deref%28%26self%29+-%3E+%26Self%3A%3ATarget+%7B%0A++++++++unsafe+%7B+%26*self.lock.value.get%28%29+%7D%0A++++%7D%0A%7D%0A%0Aimpl%3CT%3E+DerefMut+for+SpinlockGuard%3C%27_%2C+T%3E+%7B%0A++++fn+deref_mut%28%26mut+self%29+-%3E+%26mut+Self%3A%3ATarget+%7B%0A++++++++unsafe+%7B+%26mut+*self.lock.value.get%28%29+%7D%0A++++%7D%0A%7D%0A%0Astruct+AsyncLock%3CT%3E+%7B%0A++++value%3A+Spinlock%3CT%3E%2C%0A++++state%3A+Spinlock%3CAsyncLockState%3E%2C%0A%7D%0A%0Astruct+AsyncLockState+%7B%0A++++locked%3A+bool%2C%0A++++wakers%3A+Vec%3CWaker%3E%2C%0A%7D%0A%0Astruct+AsyncLockGuard%3C%27a%2C+T%3E+%7B%0A++++lock%3A+%26%27a+AsyncLock%3CT%3E%2C%0A++++value%3A+SpinlockGuard%3C%27a%2C+T%3E%2C%0A%7D%0A%0Aimpl%3CT%3E+AsyncLock%3CT%3E+%7B%0A++++const+fn+new%28value%3A+T%29+-%3E+Self+%7B%0A++++++++Self+%7B%0A++++++++++++value%3A+Spinlock%3A%3Anew%28value%29%2C%0A++++++++++++state%3A+Spinlock%3A%3Anew%28AsyncLockState+%7B%0A++++++++++++++++locked%3A+false%2C%0A++++++++++++++++wakers%3A+Vec%3A%3Anew%28%29%2C%0A++++++++++++%7D%29%2C%0A++++++++%7D%0A++++%7D%0A%0A++++async+fn+lock%28%26self%29+-%3E+AsyncLockGuard%3C%27_%2C+T%3E+%7B%0A++++++++std%3A%3Afuture%3A%3Apoll_fn%28%7Ccx%7C+%7B%0A++++++++++++let+mut+state+%3D+self.state.lock%28%29%3B%0A++++++++++++if+state.locked+%7B%0A++++++++++++++++state.wakers.push%28cx.waker%28%29.clone%28%29%29%3B%0A++++++++++++++++std%3A%3Atask%3A%3APoll%3A%3APending%0A++++++++++++%7D+else+%7B%0A++++++++++++++++state.locked+%3D+true%3B%0A++++++++++++++++Poll%3A%3AReady%28AsyncLockGuard+%7B%0A++++++++++++++++++++lock%3A+self%2C%0A++++++++++++++++++++value%3A+self.value.lock%28%29%2C%0A++++++++++++++++%7D%29%0A++++++++++++%7D%0A++++++++%7D%29%0A++++++++.await%0A++++%7D%0A%7D%0A%0Aimpl%3CT%3E+Drop+for+AsyncLockGuard%3C%27_%2C+T%3E+%7B%0A++++fn+drop%28%26mut+self%29+%7B%0A++++++++let+mut+state+%3D+self.lock.state.lock%28%29%3B%0A++++++++state.locked+%3D+false%3B%0A++++++++%2F%2F+XXX%3A+Wakers+can+run+arbitrary+code%2C+so+in+theory+this+could+deadlock.+Technically+the%0A++++++++%2F%2F+same+applies+to+.clone%28%29+above.%0A++++++++for+waker+in+state.wakers.drain%28..%29+%7B%0A++++++++++++waker.wake%28%29%3B%0A++++++++%7D%0A++++%7D%0A%7D%0A%0Aimpl%3CT%3E+Deref+for+AsyncLockGuard%3C%27_%2C+T%3E+%7B%0A++++type+Target+%3D+T%3B%0A%0A++++fn+deref%28%26self%29+-%3E+%26Self%3A%3ATarget+%7B%0A++++++++%26self.value%0A++++%7D%0A%7D%0A%0Aimpl%3CT%3E+DerefMut+for+AsyncLockGuard%3C%27_%2C+T%3E+%7B%0A++++fn+deref_mut%28%26mut+self%29+-%3E+%26mut+Self%3A%3ATarget+%7B%0A++++++++%26mut+self.value%0A++++%7D%0A%7D%0A%0Astatic+X%3A+AsyncLock%3Ci32%3E+%3D+AsyncLock%3A%3Anew%280%29%3B%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++%2F%2F+Run+ten+concurrent+futures+that+all+increment+X.%0A++++let+mut+futures+%3D+Vec%3A%3Anew%28%29%3B%0A++++for+_+in+0..10+%7B%0A++++++++futures.push%28async+%7B%0A++++++++++++let+mut+guard+%3D+X.lock%28%29.await%3B%0A++++++++++++%2F%2F+Sleep+while+holding+the+lock%2C+to+test+the+Waker+handling+in+%60lock%60+and+%60drop%60+above.%0A++++++++++++sleep%28Duration%3A%3Afrom_millis%28100%29%29.await%3B%0A++++++++++++*guard+%2B%3D+1%3B%0A++++++++%7D%29%3B%0A++++%7D%0A++++join_all%28futures%29.await%3B%0A++++println%21%28%22The+final+value+of+X+is+%7B%7D.%22%2C+*X.lock%28%29.await%29%3B%0A%7D>

## Prior art
[prior-art]: #prior-art

- [Footgun lurking in `FuturesUnordered` and other concurrency-enabling
  streams](https://github.com/rust-lang/futures-rs/issues/2387)
- [Barbara battles buffered streams][barbara]
- [`for await` and the battle of buffered
  streams](https://tmandry.gitlab.io/blog/posts/for-await-buffered-streams/)
- [Future's liveness
  problem](https://skepfyr.me/blog/futures-liveness-problem/) - "If you call
  `poll` and it returns `Poll::Pending`, when the waker you provided is woken
  and the task is next polled, you must either call `poll` again or drop the
  future."
- [Futurelock]["Futurelock"]
- [Never snooze a future][snooze]

## Unresolved questions
[unresolved-questions]: #unresolved-questions

### Should we allow an indefinite delay between creation and polling?

In other words, should the following be allowed, or should it warn somehow?

```rust
let future1 = foo();
let future2 = foo();
future1.await;
future2.await;
```

We could say that `future2` is snoozed here across the first await. On the
other hand, `future2` has never been polled (or even pinned), and it's not
likely to be holding any exclusive resources in its initial state. We could
imagine giving `foo` e.g. a `MutexGuard` argument, but in that case the caller
could see what's going on. To create a truly non-local problem, we'd need to
write `foo` in a sync-then-async style, like this:

```rust
fn foo() -> impl Future<Output = ()> {
    static LOCK: Mutex<()> = Mutex::const_new(());
    // Try to acquire `LOCK` synchronously. If we get it, the returned future takes ownership of the guard.
    let mut _guard = LOCK.try_lock().ok();
    async move {
        if _guard.is_none() {
            _guard = Some(LOCK.lock().await);
        }
        sleep(Duration::from_millis(10)).await;
    }
}
```

This style is uncommon, and it's especially uncommon to combine it with eagerly
acquired locks, but [here's an example of Wasmer doing it][wasmer]. At the same
time, there are published `Future` extension methods (e.g. [`delay`] from
`async-std`) that are only correct if delayed initial polling is acceptable.

[`delay`]: https://docs.rs/async-std/latest/async_std/prelude/trait.FutureExt.html#method.delay
[wasmer]: https://github.com/wasmerio/wasmer/blob/dce84a907542c331661f201eff9d898ecb2fbe08/lib/virtual-net/src/rx_tx.rs#L171-L187

### Should we require callers to drop futures promptly after they return `Ready`?

In practice we almost always drop futures as soon as they return `Ready`.
`.await` expressions do that automatically, and combinators like `Join` often
do the same using `Fuse` or `MaybeDone` internally. However, this behavior
isn't required by the `Future` contract, so it's problematic to rely on it for
correctness. One place where we currently rely on it is [`Timeout`], which
[looks like this][timeout_src]:

[`Timeout`]: https://docs.rs/tokio/latest/tokio/time/fn.timeout.html
[timeout_src]: https://github.com/tokio-rs/tokio/blob/tokio-1.53.3/tokio/src/time/timeout.rs#L176-L181

```rust
pub struct Timeout<T> {
    #[pin]
    value: T,
    #[pin]
    delay: Sleep,
}
```

The future (`value`) is stored directly in the `Timeout` struct, not wrapped in
an `Option` or similar, so `Timeout` can't explicitly drop it. This works as
long as the caller drops `Timeout` as soon as it returns `Ready`, but again
that's not required. Here's an example where the caller doesn't drop `Timeout`
promptly, causing a deadlock ([playground link][ready_rule_playground]):

[ready_rule_playground]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+tokio%3A%3Aselect%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%2C+timeout%7D%3B%0A%0Aasync+fn+foo%28%29+%7B%0A++++%2F%2F+Acquire+a+global+lock%2C+sleep+briefly%2C+and+release+it.%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%2F%2F+A+couple+trivial+wrapper+functions%2C+to+make+the+deadlock+below+less+%22obvious%22.%0Aasync+fn+bar%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0Aasync+fn+baz%28%29+%7B%0A++++foo%28%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++select%21+%7B%0A++++++++biased%3B%0A++++++++Ok%28_%29+%3D+timeout%28Duration%3A%3Afrom_millis%285%29%2C+bar%28%29%29+%3D%3E+%7B%7D%0A++++++++_+%3D+async+%7B%0A++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++baz%28%29.await%3B%0A++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++%7D+%3D%3E+%7B%7D%0A++++%7D%0A%7D>

```rust
select! {
    biased;
    Ok(_) = timeout(Duration::from_millis(5), bar()) => {}
    _ = baz() => {} // Deadlock!
}
```

This example is especially subtle:

1. [The `biased` keyword][biased] means that the first arm always acquires
   `LOCK` first.
2. The `timeout` expires after 5 ms, and polling it returns `Ready(Err(_))`.
3. Normally `select!` would drop all its future arguments as soon as one of
   them is `Ready`. However, the `Ok(_)` pattern fails to match here, so
   `select!` keeps polling the second arm. As of Tokio v1.53.3, `select!` does
   not drop individual arms while others are still getting polled.
4. As we discussed above, `Timeout` still holds the `bar` future, which still
   holds the `LOCK` guard, so the `baz` future is deadlocked.

[biased]: https://docs.rs/tokio/1.53.3/tokio/macro.select.html#fairness

The behavior of `select!` in step #3 might be worth fixing,[^original_zip] but
the key question here is whether `Timeout` can _assume_ that its callers will
never behave like this. As written, the strict `Future` contract blames
`Timeout` for this deadlock, because it stopped polling `bar` while it was
still pending, without ensuring that it would get dropped promptly. We need to
fix `Timeout` by wrapping `value` in an `Option` or similar and explicitly
clearing it in the cancelled case.[^join_maybe] Unfortunately, that's not
compatible with existing public accessor methods like [`into_inner`], which
don't currently return `Option`.

[^join_maybe]: Note that [the `join_maybe` playground example
    above][join_maybe] drops the `maybe` future internally to conform to this
    rule. This is easy to get wrong in practice. On the other hand, as we see
    with the current behavior of `select!`, a requirement to drop ready futures
    promptly would also be easy to get wrong.

[`into_inner`]: https://docs.rs/tokio/latest/tokio/time/struct.Timeout.html#method.into_inner

Apart from that backwards compatibility issue, are we happy with this fix? Or
would we rather say that the `Timeout` struct definition above is correct, and
explicitly require callers to drop futures promptly after they return `Ready`?
Either approach could be reasonable. Another relevant data point here is that
`async fn` futures do _not_ rely on their callers to drop them; they drop local
variables internally before returning `Ready`.

[^original_zip]: The original implementation of the
    [`futures_lite::future::Zip`] combinator had a similar issue, which was
    [reported as a deadlock bug][zip_deadlock] and fixed in 2024.

[`futures_lite::future::Zip`]: https://docs.rs/futures-lite/latest/futures_lite/future/fn.zip.html
[zip_deadlock]: https://github.com/smol-rs/futures-lite/issues/105

If we do require callers to drop ready futures, it probably also makes sense to
require callers to drop futures that panic.

## Future work
[future-work]: #future-work

(No pun intended.)

We'll need to decide on a fix and implement it for [each of the broken patterns
above][broken_patterns]. This RFC doesn't try to make those decisions, and many
of the relevant APIs are in the crates ecosystem and not the standard library.
However, there are two major issues that the standard library will need to
address: the `Pin<&mut _>` blanket impl, and `AsyncIterator`.

### `Future` blanket impls

[The `Pin<&mut _>` blanket impl][blanket] is at the heart of the
[cancellation-by-reference][cancellation_by_reference] category of deadlocks.
As the [Motivation][motivation] section mentioned, we can't deprecate it today,
in part because `Pin<Box<_>>` needs to keep implementing `Future`. Maybe the
impl could be split up using unstable specialization features internally, or
maybe we could implement a more bespoke mechanism. But the hard problem here
isn't implementing a deprecation warning; it's rolling out new helper functions
and macros that we can recommend to programmers who see that warning.

We'll need a variety of tools, both simple helper functions and fancier macros.
We'll probably want `try_` versions that short-circuit with errors. We'll have
to come up with solutions for difficult cases like [modifying a stream while we
`select!` on it][mini_redis]. We'll bikeshed names for all of these things, and
then eventually we'll get some experience using them in production. Most of
this work will be done in `futures-rs`, Tokio, and other ecosystem crates.
Finally, we'll make a judgment call that the new patterns are stable enough to
warn on the old ones.

Note that [the blanket `Future` impl for unpinned `&mut`
references][unpin_blanket] is also incorrect for the same reason as the pinned
one. The `Unpin` bound makes it less relevant in practice, but in theory we
could reproduce all the deadlocks above using `&mut Pin<Box<_>>` without ever
instantiating a `Pin<&mut _>` reference ourselves.

[unpin_blanket]: https://doc.rust-lang.org/std/future/trait.Future.html#impl-Future-for-%26mut+F

### `AsyncIterator`

See [RFC 4020][poll_progress_rfc].

## Future possibilities
[future-possibilities]: #future-possibilities

### Concurrency syntax

One of the reasons `select!` is widely used is that [it's a workaround for
borrow checker limitations][mini_redis]. Here's a `join` example that doesn't
compile ([playground link][join_error]):

[join_error]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Afuture%3A%3Ajoin%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+mut+x+%3D+0%3B%0A++++let+fut1+%3D+async+%7B%0A++++++++sleep%28Duration%3A%3Afrom_secs%281%29%29.await%3B%0A++++++++x+%2B%3D+1%3B%0A++++%7D%3B%0A++++let+fut2+%3D+async+%7B%0A++++++++sleep%28Duration%3A%3Afrom_secs%281%29%29.await%3B%0A++++++++x+%2B%3D+1%3B+%2F%2F+error%3A+cannot+borrow+%60x%60+as+mutable+more+than+once+at+a+time%0A++++%7D%3B%0A++++join%28fut1%2C+fut2%29.await%3B%0A++++assert_eq%21%28x%2C+2%29%3B%0A%7D>

```rust
let mut x = 0;
let fut1 = async {
    sleep(Duration::from_secs(1)).await;
    x += 1;
};
let fut2 = async {
    sleep(Duration::from_secs(1)).await;
    x += 1; // error: cannot borrow `x` as mutable more than once at a time
};
join(fut1, fut2).await;
assert_eq!(x, 2);
```

The problem there is that both `fut1` and `fut2` implicitly borrow `&mut x`. We
can work around that problem with a `select!` ([playground
link][select_workaround]):

[select_workaround]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Afuture%3A%3A%7BFusedFuture%2C+FutureExt%7D%3B%0Ause+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Aselect%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+mut+x+%3D+0%3B%0A++++let+mut+fut1+%3D+pin%21%28sleep%28Duration%3A%3Afrom_secs%281%29%29.fuse%28%29%29%3B%0A++++let+mut+fut2+%3D+pin%21%28sleep%28Duration%3A%3Afrom_secs%281%29%29.fuse%28%29%29%3B%0A++++loop+%7B%0A++++++++select%21+%7B%0A++++++++++++_+%3D+%26mut+fut1%2C+if+%21fut1.is_terminated%28%29+%3D%3E+x+%2B%3D+1%2C%0A++++++++++++_+%3D+%26mut+fut2%2C+if+%21fut2.is_terminated%28%29+%3D%3E+x+%2B%3D+1%2C%0A++++++++++++else+%3D%3E+break%2C%0A++++++++%7D%0A++++%7D%0A++++assert_eq%21%28x%2C+2%29%3B%0A%7D>

```rust
let mut x = 0;
let mut fut1 = pin!(sleep(Duration::from_secs(1)).fuse());
let mut fut2 = pin!(sleep(Duration::from_secs(1)).fuse());
loop {
    select! {
        _ = &mut fut1, if !fut1.is_terminated() => x += 1,
        _ = &mut fut2, if !fut2.is_terminated() => x += 1,
        else => break,
    }
}
assert_eq!(x, 2);
```

Code like this is a pain to read and write. It uses `select!` to implement a
`join`, which is counterintuitive. It needs pinning, fusing, and a couple
non-obvious bits of macro syntax. Plus it's driving futures by reference, which
this RFC wants to ban. But these loops are surprisingly common in the wild,
because they're one of the only ways to do concurrent mutation without interior
mutability. They work because the `select!` macro lowers all its `=>`
expressions to a single `match`, and the borrow checker can see that only one
of the `match` arms runs.

What if there was something like the straightforward `join` example above that
the borrow checker could understand? Let's make up some hypothetical syntax:

```rust
let mut x = 0;
await all {
    sleep(Duration::from_secs(1)).await;
    x += 1;
} and {
    sleep(Duration::from_secs(1)).await;
    x += 1;
}
assert_eq!(x, 2);
```

The key idea here is that, even though both of the arms here mutate `x`,
neither of them retains `&mut x` across a suspension point, so they don't
conflict with each other. This wouldn't be sound if the arms were independent
futures that could escape to different threads, but in this syntax they're tied
together. In effect, the whole `await all` expression borrows `x` once, and the
individual arms reborrow `x` temporarily when they're polled. The borrow
checker could verify that.

Syntax bikeshedding for a feature like this would be intense, of course, but an
interesting feature of `await all`...`and`\[...`and`\] is that it suggests a
counterpart, `await any`...`or`\[...`or`\]. The former could have arms of
different types, and it would evaluate to a tuple, while the latter would
require arms of the same type, and it would evaluate to a single value. In
other words, `await all` would be a built-in `join!`, and `await any` would be
a built-in `select!`. `await any` could also allow conflicting _moves_ after
the final suspension point in each arm, since only one arm would make it that
far.

Another upside of syntax like this would be better ergonomics for
short-circuiting. It's annoying to use `return` or `?` in an `async` block,
because they only short-circuit the block, not the calling function. It's also
impossible to `break` or `continue` a loop outside the block. With concurrency
keywords, short-circuiting could work naturally. Another interesting
possibility would be `yield`ing from concurrent blocks in an `async gen fn`;
see [RFC 4020][poll_progress_rfc].

[barbara]: https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_battles_buffered_streams.html
["Futurelock"]: https://rfd.shared.oxide.computer/rfd/0609
[futurelock_diff]: https://github.com/oxidecomputer/omicron/pull/9268/changes?diff=split
[`pthread_cancel`]: https://man7.org/linux/man-pages/man3/pthread_cancel.3.html
[`TerminateThread`]: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminatethread
[`SuspendThread`]: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-suspendthread
[snooze]: https://jacko.io/snooze.html
[`select!`]: https://tokio.rs/tokio/tutorial/select
[`AsyncIterator`]: https://doc.rust-lang.org/core/async_iter/trait.AsyncIterator.html
[`Stream`]: https://docs.rs/futures/latest/futures/prelude/trait.Stream.html
[mini_redis]: https://smallcultfollowing.com/babysteps/blog/2022/06/13/async-cancellation-a-case-study-of-pub-sub-in-mini-redis/
[`StreamExt::next`]: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.next
[`FuturesUnordered`]: https://docs.rs/futures/latest/futures/stream/struct.FuturesUnordered.html
[blanket]: https://doc.rust-lang.org/std/future/trait.Future.html#impl-Future-for-Pin%3CP%3E
[poll_progress_rfc]: https://github.com/rust-lang/rfcs/pull/4020
[`Future`]: https://doc.rust-lang.org/std/future/trait.Future.html
