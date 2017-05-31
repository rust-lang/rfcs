- Feature Name: generators
- Start Date: 2016-12-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This adds generators to Rust, which have the ability to await on values, yield and return. Awaiting and yielding will suspend the generator. Generator are also known as stackless coroutines or semicoroutines.

# Motivation
[motivation]: #motivation

The primary motivation for this is that it allows writing code that looks like code with blocking operations, but which actually suspends and does something else while that operation is happening. In order to do this in Rust currently, we need to either create another thread, which comes at a huge performance cost, create a manual state machine which is hard to reason about, or use future combinators to create the state machine, which is a lot better, but still hard and unfamiliar.

It's useful for writing iterators and high-scale servers where you wouldn't want an expensive thread per concurrent request. So this feature aligns well with the [proposed 2017 roadmap](https://github.com/rust-lang/rfcs/pull/1774).

# Detailed design
[design]: #detailed-design

Generators are functions which can suspend its execution and resume at a later point. Initially they start out suspended and return a value of an anonymous type which implement the `Generator` trait. Let's call this anonymous type the frame type for the specific generator. Instead of allocating arguments, local and temporary variables on the stack, we'll allocate storage in the frame type. can avoid storing the values in the frame type, if possible. We also need to keep track of where in the function we are, so we add an instruction pointer which initally points to the entry point of the function. The Rust compiler is free to optimize this representation as long as it preserves the semantics.

Let's look at the Generator trait:
```rust
pub enum State<Y, R, B> {
    Yielded(Y),
    Complete(R),
    Blocked(B),
}

pub trait Executor {
    type Blocked;
}

pub trait Generator<E: Executor> {
    type Yield;
    type Return;
    fn resume(&mut self, executor: &mut E) -> State<Self::Yield, Self::Return, E::Blocked>;
}
```

When `resume` is called on a generator we resume the function in the position the instruction pointer indicates and it can return any of the `State` enum variants, giving the result why the generator suspended. `Blocked` is returned by `await` expressions indicating that the value the generator is waiting on is not yet ready. `Yielded` is returned with the yielded value when generators yield. `Complete` is given with the return value when the generator finally finishes, and further resumptions will result in panics.

The associated `Yield` types gives the type of values that can be yielded and `Return` gives the type of return values.

Functions and closures will return a generator if they use `await` or `yield`. Closures that return a generator will still be able to capture variables.

## Executors

You may have noticed that the `resume` method takes an `executor` argument of type `E: Executor`. Executors are the entity responable for executoring generators. We'll introduce one such executor now.

Let `()` be a executor.
```rust
impl Executor for () {
    type Blocked = !;
}
```
Here we give `Blocked` the never type. This means that our executor can only run generators which don't return `Blocked` from the `State` enum; since there aren't any instances of `!`. Our executor can only run generators which cannot block. This is why we call `()` the synchronous executor.

We can now define a function which runs generators which do not yield and do not block using our executor. Note that generators which cannot block and cannot yield are just regular closures from a caller perspective.
```rust
fn run<T: Generator<(), Yield = !>>(mut generator: T) -> T::Return {
    match generator.resume(&mut ()) {
        State::Complete(r) => r
    }
}
```

Let's consider generators which yield values, cannot block and only return `()`. This is normal iterators, and we can implement them for all such generators:
```rust
impl<T: Generator<(), Return = ()>> Iterator for T {
    type Item = T::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        match self.resume(&mut ()) {
            State::Complete(..) => None,
            State::Yielded(v) => Some(v),
        }
    }
}
```
Such an implementation should be part of the standard library.

## Suspend points

Let's look at suspend points in detail. Inside a generator, `await`, `await for`, `yield` and `return` are all sugar built on top of a suspend operation, which suspends the generator and gives the passed value to the `resume` function. The suspend operation is not accessible for end users and is only an implementation detail.

`return <v>` expands to `suspend State::Complete(<v>); panic!("the generator had already completed")`

`yield <v>` expands to `suspend State::Yielded(<v>)`

`await <f>` expands to
```rust
let object = <f>;
loop {
    match Generator::<Yield=!>::resume(&mut object, executor) {
        State::Complete(v) => break v,
        State::Blocked(b) => suspend State::Blocked(b),
    }
}
```
Where `executor` is the executor argument passed to `resume`.

`await for <v> in <g> { <body> }` expands to
```rust
let generator = <g>;
'await: loop {
    let <v> = loop {
        match Generator::resume(&mut generator, executor) {
            State::Yielded(v) => break v,
            State::Complete(v) => break 'await v,
            State::Blocked(b) => suspend State::Blocked(b),
        }
    };

    <body>
}
```

## Examples

The `Generator` trait is designed so that it can be used with `impl Generator<R>`. So, if we wanted a coroutine which yields 10 values:
```rust
fn count_to_ten() -> impl Generator<(), Return=usize> {
	for i in 1...10 {
		yield i;
	}
}
```
Since we know that this doesn't block and returns `()`, we can substitute `Generator<()>` for `Iterator` and get this
```rust
fn count_to_ten() -> impl Iterator<Item=usize> {
	for i in 1...10 {
		yield i;
	}
}
```
Very pretty Iterator implementations result from this.

### Futures

Futures are just a simple wrapper on top of `Generator` where `Yield` is `!`.
```rust
pub trait Future<E: Executor>: Generator<E, Yield=!> {}

impl<E: Executor, T: Generator<E, Yield=!>> Future<E> for T {}
```

Now we can implement simple generators which waits for values:
```rust
fn sum<E: Executor, T: Future<E, Return=usize>>(a: T, b: T) -> impl Future<E, Return=usize> {
    await a + await b
}
```

### Streams

Streams are futures which can yield values. So streams are basically generators. We introduce this trait so we can call it `Stream` instead of `Generator`.
```rust
pub trait Stream<E: Executor>: Generator<E> {}

impl<E: Executor, T: Generator<E>> Stream<E> for T {}
```

We can use streams in `await for` loops. A simple example would be a function taking a stream a producing a future with the sum of the elements in the stream:
```rust
fn sum<E: Executor, S: Stream<E, Item=usize, Return=()>>(stream: S) -> impl Future<E, Return=usize> {
    let sum = 0;
    await for i in stream {
        sum += i;
    }
    sum
}
```
Another similar example where the function serves requests:
```rust
fn server<E: Executor>(socket: Socket) -> impl Future<E, Return=()> {
    await for client in socket {
        client.write("Hello");
    }
}
```

Here a funtion is taking a stream and returning a stream with the elements doubled:
```rust
fn double<E: Executor, R, S: Stream<E, Return=R, Yield=usize>>(stream: S) -> impl Stream<E, Return=R, Yield=usize> {
    await for i in stream {
        yield 2*i;
    }
}
```

### An asynchronous executor

Let's create an event loop which can process futures. We define:
```rust
type Task = Rc<RefCell<Future<EventLoop, Return=(), Yield=!>>>;
```
A `Task` is a reference counted future which doesn't yield and returns `()`.
```rust
pub struct EventLoop {
    current: Option<Task>,
    timers: Vec<Rc<Timer>>,
}
```
An `EventLoop` has a current or active task, and a list of timers.
```rust
pub struct Timer {
    remaining: Cell<u64>,
    task: Task,
}
```
`Timer` contains a reference to the task to wake up and a remaning time in milliseconds.

```rust
impl EventLoop {
    pub fn new() -> EventLoop {
        EventLoop {
            current: None,
            timers: Vec::new(),
        }
    }
    
    fn timer(&mut self, task: Task, delta: u64) -> Rc<Timer> {
        let timer = Rc::new(Timer {
            remaining: Cell::new(delta),
            task: task,
        });
        self.timers.push(timer.clone());
        timer
    }
}
```
We add a method to create timers for a task. It simply adds them to the list and gives out a reference to the caller. `new` just constructs an event loop.

```rust
impl EventLoop {
    fn run_task(&mut self, task: Task) {
        self.current = Some(task.clone());
        task.borrow_mut().resume(self);
        self.current = None;
    }

    pub fn run<F: Generator<Self, Return=(), Yield=!> + 'static>(&mut self, future: F) {
        let task = Rc::new(RefCell::new(future));

        if self.current.is_some() {
            // We are currently inside the event loop, add the task to the list of tasks to run
            self.timer(task, 0);
            return;
        }

        self.run_task(task);

        while !self.timers.is_empty() {
            let mut i = 0;

            while i < self.timers.len() {
                if self.timers[i].remaining.get() == 0 {
                    let task = self.timers[i].task.clone();
                    self.run_task(task);
                    self.timers.remove(i);
                } else {
                    let remaining = self.timers[i].remaining.get();
                    self.timers[i].remaining.set(remaining - 1);
                    i += 1;
                }
            }

            thread::sleep(Duration::from_millis(1));
        }
    }
}
```
`run` is the main method for the event loop. It runs a future passed as an argument and waits until all timers have elapsed, which are the only source of events here. In the outer loop we run tasks for expired timers and decrement the rest, we then sleep for 1 ms.
If we are inside the event loop when using `run`, it will instead add the future to the list of tasks to run.

```rust
impl Executor for EventLoop {
    type Blocked = ();
}
```
`Blocked` is `()` which is sufficient to indicate that an operation is blocked.

Now we would like a way to use these timers in generators. We create a `AsyncSleep` struct which allows us to await on timers.
```rust
enum SleepState {
    Pending(u64),
    Started(Rc<Timer>)
}

pub struct AsyncSleep(SleepState);

impl Generator<EventLoop> for AsyncSleep {
    type Return = ();
    type Yield = !;

    fn resume(&mut self, executor: &mut EventLoop) -> State<!, Self::Return, ()> {
        match self.0 {
            SleepState::Pending(delta) => {
                let task = executor.current.as_ref().unwrap().clone();
                self.0 = SleepState::Started(executor.timer(task, delta));
                State::Blocked(())
            }
            SleepState::Started(ref timer) => if timer.remaining.get() == 0 {
                State::Complete(())
            } else {
                State::Blocked(())
            }
        }
    }
}
```
We can then use them:
```rust
fn count_to_10() -> impl Stream<EventLoop, Return=(), Yield=usize> {
    for i in 1...10 {
        await AsyncSleep(SleepState::Pending(1000));
        yield i;
    }
}
```
Notice how we use both `await` and `yield` here.

### Executors generic over synchronous and asynchronous operations

Now the above example was no longer generic over executors, but required `EventLoop` since that allows sleeping. Wouldn't it be nice if we could remain generic here and let `count_to_10` have the ability to return an `Iterator`, although a slow one?

To achieve this, we must extend the `Executor` trait.
```rust
pub trait SampleExecutor: Executor where Self:Sized {
    type Sleep: Future<Self, Return=()>;
    fn sleep(delta: u64) -> Self::Sleep;
}
```
This trait adds a function `sleep` which when called returns a future which will sleep for `delta` ms before returning.

Now we must implement it for `EventLoop`
```rust
impl SleepExecutor for EventLoop {
    type Sleep = AsyncSleep;

    fn sleep(delta: u64) -> Self::Sleep {
        AsyncSleep(SleepState::Pending(delta))
    }
}
```
And also `()`:
```rust
impl SleepExecutor for () {
    type Sleep = SyncSleep;

    fn sleep(delta: u64) -> Self::Sleep {
        SyncSleep(delta)
    }
}

pub struct SyncSleep(u64);

impl Generator<()> for SyncSleep {
    type Return = ();
    type Yield = !;

    fn resume(&mut self, executor: &mut ()) -> State<!, Self::Return, !> {
        thread::sleep(Duration::from_millis(self.0));
        State::Complete(())
    }
}
```

Next is an example that is generic over synchronous and asynchronous operation. 
```rust
fn count_to_10<E: SleepExecutor>() -> impl Stream<E, Return=(), Yield=usize> {
    for i in 1...10 {
        await E::sleep(1000);
        yield i;
    }
}
```

### Interactions with `Result` and `?` 

Practical futures usually can result in errors. Luckily they compose well with `Result` types.
Say we have a function:
```rust
fn write_str(str: &str) -> impl Future<EventLoop, Return=Result<(), WriterError>>;
```
We then use it like this:
```rust
fn write_hello_world<F: Future<Self, Return=Writer>>() -> impl Future<EventLoop, Return=Result<(), WriterError>> {
    (await write_str("Hello "))?;
    (await write_str("world"))?;
}
```

For streams we can also use `Result` as the return type. We'll keep the yielding type the same. Say we have this stream which yields 1 to 10, with the possibility of a `CountingError`:
```rust
fn count_to_10() -> impl Stream<EventLoop, Return=Result<(), CountingError>, Yield=usize>;
```
We can then implement a function which doubles the output of the above stream, handling errors:
```rust
fn doubling_count_to_10() -> impl Stream<EventLoop, Return=Result<(), CountingError>, Yield=usize> {
    await for i in count_to_10() {
        yield 2*i;
    }?
}
```

### Passing in arguments at each resumption

Passing arguments to the `resume` function which the generator can access can be useful.

For futures you might try passing the event that caused the task to resume into the `resume` function. However since futures aren't require to resume all previously resumed contained futures, the argument might not reach the future which is actually waiting for the event. For streams you'd only pass such an event in if it was suspended due to blocked future. If it was suspended due to a yield expression, you'd want an unrelated type. So it appears passing arguments isn't useful in the case of futures and streams.

It is however useful for generators which only yield and return.

We define an synchronous executor which contains the arguments.
```rust
pub struct ArgsExecutor<T>(Option<T>);

impl<T> ArgsExecutor<T> {
    pub fn new(args: T) -> Self {
        ArgsExecutor(Some(args))
    }
}

impl<T> Executor for ArgsExecutor<T> {
    type Blocked = !;
}
```
Also we need a way to extract the arguments, so we create a generator which returns the contained arguments.
```rust
pub struct ArgsExtractor;

impl<T> Generator<ArgsExecutor<T>> for ArgsExtractor {
    type Yield = !;
    type Return = T;

    fn resume(&mut self, executor: &mut ArgsExecutor<T>) -> State<!, Self::Return, !> {
        State::Complete(executor.0.take().expect("arguments already extracted"))
    }
}
```
We can then use `ArgsExecutor::new` to pass in arguments to `resume` and `await ArgsExtractor` to access them.

We'll adapt [this example](https://github.com/vadimcn/rfcs/blob/coroutines2/text/0000-coroutines.md#double-ended-iterators) to see how we'll pass and extract arguments.
```rust
enum IterEnd {
    Head,
    Tail
}

impl<T,F> Iterator for F where F: Generator<ArgsExecutor<IterEnd>, Yield=T, Return=()> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self.resume(&mut ArgsExecutor::new(IterEnd::Tail)) {
            Yield(x) => Some(x),
            Return(..) => None
        }
    }
}

impl<T,F> DoubleEndedIterator for F: Generator<ArgsExecutor<IterEnd>, Yield=T, Return=()> {
    fn next_back(&mut self) -> Option<T> {
        match self.resume(&mut ArgsExecutor::new(IterEnd::Head)) {
            Yield(x) => Some(x),
            Return(..) => None
        }
    }
}

impl<T> [T] {
    fn iter(&'a self) -> impl DoubleEndedIterator<T> + 'a {
        let mut i = 0;
        let mut j = self.len();
        while i < j {
            match await ArgsExtractor {
                Tail => {
                     yield self[i];
                     i += 1;
                },
                Head => {
                    j -= 1;
                    yield self[j];
                }
            }
        }
    }
}
```

## Expressions as both closures and generators

In the above examples we've only seen function being generators, but closures can also be generators, and they can still capture variables.

For example:
```rust
fn test() -> impl Future<EventLoop, Return=usize> {
    let shared = Rc::new(Cell::new(0usize));
    let shared_future = shared.clone();

    let future = move || -> impl Future<EventLoop, Return=usize> {
        return shared_future.get()
    };

    shared.set(4);

    return await future;
}
```

## Implementation

### Desugaring to HIR

The suspend point sections gave some expansions for the relevant expressions. These expansions should be applied when generating HIR, with the exception of the `return` expansion, which should stay intact in order to avoid duplicating the panic code.

### Implications for type inference

Type inference for generators would work similarly to type inference for closures. We have some expression which has an anonymous type which implements a trait.

When we do type inference for functions or closures which are generators, we create 3 type variables, one for the type of the executor (which does not appear in the signature), one for the return type (since the type in the signature is the generator to return), and one for the yield type, which defaults to `!`. The anonymous type implements `Generator<E, Return=R, Yield=Y>` where `E` is the type variables created for the executor, `R` is the return type variable and `Y` is the yield type variable. The type of values in the yield expression unifies with the yield type variable.

### Changes to the borrow checker

Since the stack frame of a generator can move while it is suspended, all references to data inside will be invalidated. Because of this, we require the borrow checker to reject references to local variables, temporary variables, and arguments which lifetimes cross suspension points.

The minimal change would be to reject all references with lifetimes crossing suspension points, but this is quite restrictive, so we would like a more precise solution.

One such a solution would be to use dataflow analysis to detect when variables contains loans to local values and use the result of that to calculate if loans are to local values. Finally we reject only loans which are to local values and cross a suspension point.

### State machine transformation

We do the state machine transformation after borrow checking so that we avoid having to translate errors from the post-transformation program to the pre-transformation program.

After MIR generation of generators, we can run MIR optimizations which are aware of suspend statements. Then we split the function into 2 parts. One which contains the body and is the `resume` function of the implemenatation of the `Generator` trait. The other function corresponds to the declared function and will just constructs an instance of the anonymous generator type, passing in the arguments which are needed after the first suspend operation.

We will move the storage of variables which are live across suspend points into the anonymous type and generated loads and stores as needed.

The anonymous type will also contain a state variable which is initialized as `EntryPoint`. There will be a switch on the top of the function which selects the basic block to go to based on this state variaible. `EntryPoint` goes to what was the previous entry point of the function. Suspend statements will expand to a statement setting the state variable to a new state `S` and a regular return statement returning the value passed to the suspend statement. The point after the suspend statement is turned into a new basic block and `S` will map to it.

Return statements are expanded to a statement setting the state variable to `Complete` and a return statement returning `State::Complete(r)` where `r` is the value passed to the return statement. A single basic block is added per function which `Complete` maps to. It contains the code which panics. This way, there isn't any duplication of panic code.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

The name generator is chosen over stackless coroutines since these generators actually live on the stack, as opposed to C++'s stackless coroutine. Also that is done to avoid confusion with stackful coroutines, for which there exists libraries already.

The concepts of generators is likely to be known to programmers already since a number of other languages have this feature. It should still be introduced as a concept in the book, possibly with the viewpoints of functions being suspended and as a transformation into a state machine.
Some examples for iterators, futures and streams should be added to the book and _Rust by Example_.

# Drawbacks
[drawbacks]: #drawbacks

This adds lot of complexity to the language. It also doesn't allow support for `futures-rs`, due to current trait coherence rules.

# Alternatives
[alternatives]: #alternatives

An alternative is [this RFC](https://github.com/rust-lang/rfcs/pull/1823). It focuses on minimal language changes at the cost of ergonomics. It lacks the `Blocked` variant from the generator result, which means users dealing with asynchronous operations will have to use an enum on top of that. It also doesn't allow functions to be generators, but require a nested closures.

# Unresolved questions
[unresolved]: #unresolved-questions

Should we add an `args: Args` parameter to `resume` and `Args` to `Generator` to make passing arguments more ergonomic and efficient? This would make wrapper implementations (like implementing `SleepExecutor`) for `ArgsExecutor` unnecessary too.

Should we let the generator return a new `State::Completed` variant instead of panicking?

Does a `IntoGenerator` trait, analogus to IntoIterator for `for`-loops make sense? Would future changes to trait coherence rules allow support for `futures-rs` if we add this?

Should we require an attribute or custom syntax for the generator transformation to be applied?

What should the syntax for await expressions be?