mod vec_trie;

use inferno::flamegraph;
use std::cell::RefCell;
use std::fmt::Write;
use std::fs::File;
use std::time::{Duration, Instant};
use vec_trie::{Index, VecTrie, Visitor};

/// Declare a span to be traced. Takes a single `&'static str` argument.
///
/// The span begins when the macro is called, and ends when the guard it constructs is `drop`ed at
/// the end of the block.
#[macro_export]
macro_rules! span {
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
    /// DO NOT CALL THIS DIRECTLY. Use [crate::span!] instead.
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
            self.save_flamegraph();
            self.trie.clear();
        }
    }

    fn save_flamegraph(&self) {
        // 1. Construct the flame graph input string.
        let root = match self.trie.root() {
            None => return, // nothing to save
            Some(root) => root,
        };
        let mut flame_graph_data = String::new();
        let mut stack = vec![root];
        if let Err(err) = write_flame_graph_input(&mut flame_graph_data, &mut stack) {
            eprintln!(
                "no_nonsense_flamegraphs: failed to write to string. {}",
                err
            );
            return;
        }

        // 2. Open a file for writing.
        let mut file = match File::create("flamegraph.svg") {
            Err(err) => {
                eprintln!(
                    "no_nonsense_flamegraphs: Failed to create file `flamegraph.svg`. {}",
                    err
                );
                return;
            }
            Ok(file) => file,
        };

        // 3. Convert the flame graph input string to an SVG image and save it as that file.
        let mut inferno_options = {
            let mut options = flamegraph::Options::default();
            // How on Earth is left-truncation the default? Everybody knows that if you truncate
            // text, you truncate on the right and put ellipses.
            options.text_truncate_direction = flamegraph::TextTruncateDirection::Right;
            // We're measuring time in microseconds, not "samples" like in `perf`.
            options.count_name = "μs".to_owned();
            options
        };
        if let Err(err) =
            flamegraph::from_lines(&mut inferno_options, flame_graph_data.lines(), &mut file)
        {
            eprintln!(
                "no_nonsense_flamegraphs: failed to construct flamegraph image. {}",
                err
            );
            return;
        }
    }
}

/// Construct a flame graph input string. This is (at least loosely) a standard.
///
/// The format is a sequence of lines. Each line consists of a stack snapshot, then a space, then
/// the duration. (The duration is in unspecified units; we use microseconds.) A stack snapshot is
/// a sequence of stack frame labels separated by semicolons.
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
