//! To declare a span:
//! ```
//! use no_nonsense_flamegraphs::outln;
//!
//! fn myExistingFunctionRelatedToKittens() {
//!   outln!("kittens"); // measures the span from now until it's dropped
//!   // existing kitten related functionality
//! }
//! ```
//!
//! To print a flame graph trace:
//! ```
//! use no_nonsense_flamegraphs::set_flame_graph_handler;
//!
//! set_flame_graph_handler(Box::new(|trace| {
//!   let flamegraph: String = trace.as_flame_graph_input().unwrap();
//!   println!("FLAMEGRAPH:\n{}", flamegraph);
//! }));
//! ```

#![feature(once_cell)]

mod vec_trie;

use std::cell::RefCell;
use std::fmt::Write;
use std::lazy::SyncOnceCell;
use std::time::{Duration, Instant};
use vec_trie::{Index, VecTrie, Visitor};

/// Declare a span to be traced. Takes a single `&'static str` argument.
///
/// The span begins when the macro is called, and ends when the guard it constructs is `drop`ed at
/// the end of the block.
#[macro_export]
macro_rules! outln {
    ($name:expr) => {
        let _span = $crate::Span::new($name);
    };
}

/*****************************************************************************
 * Global settings                                                           *
 *****************************************************************************/

static FLAME_GRAPH_CALLBACK: SyncOnceCell<LoggingCallback> = SyncOnceCell::new();

/// A function to call whenever a flame graph trace is complete.
pub type LoggingCallback = Box<dyn Fn(&FlameGraph) + Send + Sync>;

/// Set the callback to invoke whenever a flame graph trace is complete.
///
/// # Panics
///
/// Panics if you call it more than once. Dont' do that.
pub fn set_flame_graph_handler(handler: LoggingCallback) {
    match FLAME_GRAPH_CALLBACK.set(handler) {
        Ok(()) => (),
        Err(_) => panic!("set_flame_graph_handler() called more than once"),
    }
}

/*****************************************************************************
 * Measuring spans                                                           *
 *****************************************************************************/

/// Measures the start & end of a span. The start is when it is constructed, the end is when it is
/// dropped. You should not use this type directly; it is only public so that macros may use it.
#[derive(Debug)]
pub struct Span {
    index: Index,
}

impl Span {
    /// DO NOT CALL THIS DIRECTLY. Use [crate::outln!] instead.
    pub fn new(name: &'static str) -> Span {
        TRACE.with(|s| {
            let mut trace = s.borrow_mut();
            let index = trace.push_call(name);
            Span { index }
        })
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        TRACE.with(|s| s.borrow_mut().pop_call(self.index));
    }
}

/*****************************************************************************
 * Global trace storage                                                      *
 *****************************************************************************/

thread_local!(static TRACE: RefCell<FlameGraph> = RefCell::new(FlameGraph::new()));

type CallSite = &'static str;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Measurement {
    duration: Duration,
    num_invocations: usize,
}

/// As the flamegraph is being collected, it stores a stack of these StackFrames, reflecting the
/// program stack.
#[derive(Debug, Clone)]
struct StackFrame {
    /// Index into the trie, storing a [Measurement].
    index: Index,
    /// When this function was invoked. This only needs to be remembered until it's done, at which
    /// point it can be converted into a Duration and added to the appropriate [Measurement].
    start: Instant,
}

/// Global store of flame graph data.
pub struct FlameGraph {
    trie: VecTrie<CallSite, Measurement>,
    stack: Vec<StackFrame>,
}

impl FlameGraph {
    fn new() -> FlameGraph {
        FlameGraph {
            trie: VecTrie::new(),
            stack: Vec::new(),
        }
    }

    fn push_call(&mut self, call_site: &'static str) -> Index {
        let parent = self.stack.last().map(|frame| frame.index);
        let child = self.trie.insert_child(parent, call_site);
        self.stack.push(StackFrame {
            index: child,
            start: Instant::now(),
        });
        child
    }

    fn pop_call(&mut self, index: Index) {
        while let Some(frame) = self.stack.pop() {
            if frame.index == index {
                let duration = frame.start.elapsed();
                self.trie.value_mut(index).duration += duration;
                self.trie.value_mut(index).num_invocations += 1;
                break;
            }
        }
        if self.stack.is_empty() {
            self.finish_trace();
        }
    }

    fn finish_trace(&mut self) {
        if let Some(callback) = FLAME_GRAPH_CALLBACK.get() {
            callback(&self);
        }
        self.trie.clear();
    }

    /// The duration of the outermost call in this trace.
    pub fn total_duration(&self) -> Duration {
        if let Some(root) = self.trie.root() {
            root.value().duration
        } else {
            Duration::new(0, 0)
        }
    }

    /// The name of the outermost call in this trace.
    pub fn outermost_call(&self) -> &'static str {
        if let Some(root) = self.trie.root() {
            root.key()
        } else {
            "Empty call stack"
        }
    }

    /// Turn this trace into input for a flame graph library. Returns `None` if the trace is empty,
    /// or if there was an error writing to a String (which seems exceedingly unlikely).
    pub fn as_flame_graph_input(&self) -> Option<String> {
        let root = match self.trie.root() {
            None => return None,
            Some(root) => root,
        };

        let mut flame = String::new();
        let mut stack = vec![root];
        match write_flame_graph(&mut flame, &mut stack) {
            Err(_) => None,
            Ok(()) => Some(flame),
        }
    }
}

fn write_flame_graph<W: Write>(
    writer: &mut W,
    stack: &mut Vec<Visitor<CallSite, Measurement>>,
) -> Result<(), std::fmt::Error> {
    for (i, node) in stack.iter().enumerate() {
        let name = node.key();
        let calls = node.value().num_invocations;
        write!(writer, "{} ({} calls)", name, calls)?;
        if i + 1 < stack.len() {
            write!(writer, ";")?;
        }
    }
    if let Some(node) = stack.last() {
        let mut duration = node.value().duration;
        for child in node.children() {
            duration -= child.value().duration;
        }
        writeln!(writer, " {}", duration.as_micros())?;
        for child in node.children() {
            stack.push(child);
            write_flame_graph(writer, stack)?;
            stack.pop();
        }
    }
    Ok(())
}

#[cfg(test)]
fn write_flamegraph_for_testing<W: Write>(
    writer: &mut W,
    node: Visitor<CallSite, Measurement>,
) -> Result<(), std::fmt::Error> {
    let name = node.key();
    let calls = node.value().num_invocations;
    write!(writer, "{} ({} calls)", name, calls)?;
    let childless = node.children().next().is_none();
    if childless {
        write!(writer, "; ")?;
    } else {
        write!(writer, " {{ ")?;
    }
    for child in node.children() {
        write_flamegraph_for_testing(writer, child)?;
    }
    if !childless {
        write!(writer, "}}")?;
    }
    Ok(())
}

#[test]
fn test_tracing() {
    use std::sync::atomic::AtomicBool;

    macro_rules! outln_test {
        ($name:expr) => {
            let _span = Span::new($name);
        };
    }

    // Expository purposes only. Don't ever `outln!` recursive functions!
    fn fib(n: usize) -> usize {
        outln_test!("fib");
        if is_small(n) {
            n
        } else {
            fib(n - 1) + fib(n - 2)
        }
    }

    fn is_small(n: usize) -> bool {
        outln_test!("is_small");
        n <= 2
    }

    static IT_RAN: AtomicBool = AtomicBool::new(false);
    set_flame_graph_handler(Box::new(|traces: &FlameGraph| {
        let mut actual = String::new();
        let root = traces.trie.root().unwrap();
        write_flamegraph_for_testing(&mut actual, root).unwrap();
        assert_eq!(
            actual,
            "fib (1 calls) { is_small (1 calls); fib (2 calls) { is_small (2 calls); }}"
        );
        IT_RAN.store(true, std::sync::atomic::Ordering::Relaxed);
    }));

    fib(3);
    assert!(IT_RAN.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
#[ignore]
fn test_perf() {
    // Remember to run with --release. Also, disable the above test; they interfere with each other.
    //
    // A big/complex flame graph had the following performance:
    // - 1ms to construct a very large flame graph. (800 lines, avg. of 15 calls, 60kb.) As far as I
    //   can tell, this time is spent formatting strings.
    // - 70ns/call to outln!, assuming that 100% of time was spent in tracing. This is for 5.4*10^6
    //   calls to outln!.
    //
    // A reasonably sized flame graph (60 lines) had the following performance:
    // - 120μs to construct a reasonably sized flame graph (60 lines, average line has 15 calls).
    // - 70ns/call to outln!, assuming that 100% of time was spent in tracing. This is for 5.4*10^6
    //   calls to outln!.

    // Expository purposes only. Don't ever `outln!` recursive functions!
    fn fib(n: usize) -> usize {
        outln!("fib");
        if is_small(n) {
            2
        } else {
            fib(n - 1) + fib(n - 2) + 2
        }
    }

    fn is_small(n: usize) -> bool {
        outln!("is_small");
        n <= 2
    }

    fn fan(n: usize) -> usize {
        let name: &'static str = Box::leak(Box::new(format!("f{}", n)));
        outln!(name);
        fib(n) + 1
    }

    fn fanout(n: usize) -> usize {
        outln!("fanout");
        let mut sum = 1;
        for i in 0..n {
            sum += fan(i);
        }
        sum
    }

    set_flame_graph_handler(Box::new(|traces: &FlameGraph| {
        println!(
            "Total tracing duration: {}ms",
            traces.total_duration().as_millis()
        );
        let now = Instant::now();
        let flamegraph = traces.as_flame_graph_input().unwrap();
        println!("FlameGraph size in bytes: {}", flamegraph.len());
        println!(
            "Time taken to construct flame graph: {}μs",
            now.elapsed().as_micros()
        );
    }));

    println!("Total num calls: {}", fanout(30));
    panic!("flamegraph::test_perf finished. Remember to disable it again.");
}
