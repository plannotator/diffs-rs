//! Borrowed diagnostic access to already compiled inner-engine instances.

use crate::{vm::Insn, RaRegex, Regex, RegexImpl, RegexOptions, SyntaxConfig};

/// One inner instance, not a deduplicated allocation or search-cache measurement.
pub struct Engine<'a> {
    /// Already compiled engine. Inspection must not perform a search.
    pub inner: &'a RaRegex,
    /// Actual expression passed to the inner compiler.
    pub pattern: &'a str,
    /// Syntax configuration passed to the inner compiler.
    pub syntax: SyntaxConfig,
    /// Delegate instruction, absent for an entirely delegated expression.
    pub instruction: Option<usize>,
    /// First and last outer capture group represented by a VM delegate.
    pub capture_groups: Option<(usize, usize)>,
    /// Whether the wholly delegated expression has an explicit match-boundary group.
    pub explicit_capture_group_0: bool,
}

impl core::fmt::Debug for Engine<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Engine")
            .field("pattern", &self.pattern)
            .field("instruction", &self.instruction)
            .field("capture_groups", &self.capture_groups)
            .field("explicit_capture_group_0", &self.explicit_capture_group_0)
            .finish_non_exhaustive()
    }
}

impl Regex {
    /// Visits borrowed engines without compiling, cloning, searching, or acquiring scratch.
    ///
    /// Distinct instances can share strategy allocations when a caller clones a Regex.
    /// An instance's `memory_usage()` must not be treated as uniquely owned heap bytes.
    pub fn visit_engines<'a>(&'a self, mut visit: impl FnMut(Engine<'a>)) {
        match &self.inner {
            RegexImpl::Wrap {
                inner,
                options,
                explicit_capture_group_0,
                debug_pattern,
            } => visit(Engine {
                inner,
                pattern: debug_pattern,
                syntax: options.syntaxc,
                instruction: None,
                capture_groups: None,
                explicit_capture_group_0: *explicit_capture_group_0,
            }),
            RegexImpl::Fancy { prog, .. } => {
                for (instruction, insn) in prog.body.iter().enumerate() {
                    if let Insn::Delegate(delegate) = insn {
                        visit(Engine {
                            inner: &delegate.inner,
                            pattern: &delegate.pattern,
                            // Compiler::new constructs default options for VM delegates.
                            syntax: RegexOptions::default().syntaxc,
                            instruction: Some(instruction),
                            capture_groups: Some((delegate.start_group, delegate.end_group)),
                            explicit_capture_group_0: false,
                        });
                    }
                }
            }
        }
    }
}
