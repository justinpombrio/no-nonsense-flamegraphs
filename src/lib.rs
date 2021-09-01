mod vec_trie;

use std::cell::RefCell;
use std::fmt::Write;
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
 * Measuring spans                                                           *
 *****************************************************************************/

/// Measures the start & end of a span. The start is when it is constructed, the end is when it is
/// dropped. You should not use this type directly; it is only public so that macros may use it.
#[doc(hidden)]
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
        use inferno::flamegraph::{from_lines, Options, TextTruncateDirection};
        use std::fs::File;

        if let Some(flame_graph_data) = self.as_flame_graph_input() {
            let mut options = Options::default();
            // How on Earth is left-truncation the default? Everybody knows that if you truncate
            // text, you truncate on the right and put ellipses.
            options.text_truncate_direction = TextTruncateDirection::Right;
            // We're measuring time in microseconds, not "samples" like in `perf`.
            options.count_name = "μs".to_owned();

            let mut file = File::create("flamegraph.svg").unwrap();
            let _ = from_lines(&mut options, flame_graph_data.lines(), &mut file);
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

    /// Turn this trace into input for a flame graph library. Returns `None` if the trace is empty,
    /// or if there was an error writing to a String (which seems exceedingly unlikely).
    fn as_flame_graph_input(&self) -> Option<String> {
        let root = match self.trie.root() {
            None => return None,
            Some(root) => root,
        };

        let mut flame = String::new();
        let mut stack = vec![root];
        match write_flame_graph_input(&mut flame, &mut stack) {
            Err(_) => None,
            Ok(()) => Some(flame),
        }
    }
}

fn write_flame_graph_input<W: Write>(
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
        writeln!(writer, " {}", duration.as_micros() + 1)?;
        for child in node.children() {
            stack.push(child);
            write_flame_graph_input(writer, stack)?;
            stack.pop();
        }
    }
    Ok(())
}
