//! PyYAML's serializer and emitter, for one document of one node tree, no anchors: the
//! context of each node, the indentation, the indicators and the choice of a scalar's style.

use super::analysis::{Analysis, analyze};
use super::{DumpOptions, Node, SEQ, STR, prepare_tag};
use crate::yaml::resolve::resolve;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Style {
    Plain,
    Single,
    Double,
}

pub(super) struct Emitter<'a> {
    pub(super) options: &'a DumpOptions,
    pub(super) out: String,
    indents: Vec<Option<usize>>,
    pub(super) indent: Option<usize>,
    flow_level: usize,
    pub(super) root_context: bool,
    mapping_context: bool,
    simple_key_context: bool,
    pub(super) column: usize,
    pub(super) whitespace: bool,
    pub(super) indention: bool,
    pub(super) open_ended: bool,
    best_indent: usize,
    pub(super) best_width: usize,
}

/// The document of `node`, then the end of the stream.
pub(super) fn emit(node: &Node, options: &DumpOptions) -> Result<String, String> {
    let mut emitter = Emitter::new(options);
    emitter.node(node, true, false, false)?;
    emitter.write_indent();
    if emitter.open_ended {
        emitter.write_indicator("...", true, false, false);
        emitter.write_indent();
    }
    Ok(emitter.out)
}

impl<'a> Emitter<'a> {
    fn new(options: &'a DumpOptions) -> Emitter<'a> {
        let best_indent = if 1 < options.indent && options.indent < 10 {
            options.indent
        } else {
            2
        };
        let best_width = if options.width > best_indent * 2 {
            options.width
        } else {
            80
        };
        Emitter {
            options,
            out: String::new(),
            indents: Vec::new(),
            indent: None,
            flow_level: 0,
            root_context: false,
            mapping_context: false,
            simple_key_context: false,
            column: 0,
            whitespace: true,
            indention: true,
            open_ended: false,
            best_indent,
            best_width,
        }
    }

    fn increase_indent(&mut self, flow: bool, indentless: bool) {
        self.indents.push(self.indent);
        match self.indent {
            None => self.indent = Some(if flow { self.best_indent } else { 0 }),
            Some(indent) if !indentless => self.indent = Some(indent + self.best_indent),
            Some(_) => {}
        }
    }

    fn pop_indent(&mut self) {
        self.indent = self.indents.pop().flatten();
    }

    fn node(
        &mut self,
        node: &Node,
        root: bool,
        mapping: bool,
        simple_key: bool,
    ) -> Result<(), String> {
        self.root_context = root;
        self.mapping_context = mapping;
        self.simple_key_context = simple_key;
        match node {
            Node::Scalar { tag, value } => self.scalar(tag, value),
            Node::Seq { tag, items, flow } => self.sequence(tag, items, *flow)?,
            Node::Map { pairs, flow, .. } => {
                if self.flow_level > 0 || *flow || pairs.is_empty() {
                    self.flow_mapping(pairs)?;
                } else {
                    self.block_mapping(pairs)?;
                }
            }
        }
        Ok(())
    }

    fn scalar(&mut self, tag: &str, value: &str) {
        let chars: Vec<char> = value.chars().collect();
        let analysis = analyze(&chars, self.options.allow_unicode);
        let is_plain_implicit = resolve(value, true).uri() == tag;
        let style = self.choose_style(&analysis, is_plain_implicit);
        let is_implicit = if style == Style::Plain {
            is_plain_implicit
        } else {
            tag == STR
        };
        if !is_implicit {
            self.write_indicator(&prepare_tag(tag), true, false, false);
        }
        self.increase_indent(true, false);
        let split = !self.simple_key_context;
        match style {
            Style::Plain => self.write_plain(&chars, split),
            Style::Single => self.write_single_quoted(&chars, split),
            Style::Double => self.write_double_quoted(&chars, split),
        }
        self.pop_indent();
    }

    fn sequence(&mut self, tag: &str, items: &[Node], flow: bool) -> Result<(), String> {
        if tag != SEQ {
            self.write_indicator(&prepare_tag(tag), true, false, false);
        }
        if self.flow_level > 0 || flow || items.is_empty() {
            self.flow_sequence(items)
        } else {
            self.block_sequence(items)
        }
    }

    fn flow_sequence(&mut self, items: &[Node]) -> Result<(), String> {
        self.write_indicator("[", true, true, false);
        self.flow_level += 1;
        self.increase_indent(true, false);
        for (index, item) in items.iter().enumerate() {
            if index > 0 {
                self.write_indicator(",", false, false, false);
            }
            if self.column > self.best_width {
                self.write_indent();
            }
            self.node(item, false, false, false)?;
        }
        self.pop_indent();
        self.flow_level -= 1;
        self.write_indicator("]", false, false, false);
        Ok(())
    }

    fn block_sequence(&mut self, items: &[Node]) -> Result<(), String> {
        let indentless = self.mapping_context && !self.indention;
        self.increase_indent(false, indentless);
        for item in items {
            self.write_indent();
            self.write_indicator("-", true, false, true);
            self.node(item, false, false, false)?;
        }
        self.pop_indent();
        Ok(())
    }

    fn flow_mapping(&mut self, pairs: &[(Node, Node)]) -> Result<(), String> {
        self.write_indicator("{", true, true, false);
        self.flow_level += 1;
        self.increase_indent(true, false);
        for (index, (key, value)) in pairs.iter().enumerate() {
            if index > 0 {
                self.write_indicator(",", false, false, false);
            }
            if self.column > self.best_width {
                self.write_indent();
            }
            if self.check_simple_key(key) {
                self.node(key, false, true, true)?;
                self.write_indicator(":", false, false, false);
            } else {
                self.write_indicator("?", true, false, false);
                self.node(key, false, true, false)?;
                if self.column > self.best_width {
                    self.write_indent();
                }
                self.write_indicator(":", true, false, false);
            }
            self.node(value, false, true, false)?;
        }
        self.pop_indent();
        self.flow_level -= 1;
        self.write_indicator("}", false, false, false);
        Ok(())
    }

    fn block_mapping(&mut self, pairs: &[(Node, Node)]) -> Result<(), String> {
        self.increase_indent(false, false);
        for (key, value) in pairs {
            self.write_indent();
            if self.check_simple_key(key) {
                self.node(key, false, true, true)?;
                self.write_indicator(":", false, false, false);
            } else {
                self.write_indicator("?", true, false, true);
                self.node(key, false, true, false)?;
                self.write_indent();
                self.write_indicator(":", true, false, true);
            }
            self.node(value, false, true, false)?;
        }
        self.pop_indent();
        Ok(())
    }

    fn check_simple_key(&self, key: &Node) -> bool {
        match key {
            Node::Scalar { tag, value } => {
                let chars: Vec<char> = value.chars().collect();
                let analysis = analyze(&chars, self.options.allow_unicode);
                prepare_tag(tag).chars().count() + chars.len() < 128
                    && !analysis.empty
                    && !analysis.multiline
            }
            Node::Seq { tag, items, .. } => {
                prepare_tag(tag).chars().count() < 128 && items.is_empty()
            }
            Node::Map { tag, pairs, .. } => {
                prepare_tag(tag).chars().count() < 128 && pairs.is_empty()
            }
        }
    }

    fn choose_style(&self, analysis: &Analysis, is_plain_implicit: bool) -> Style {
        if is_plain_implicit
            && !(self.simple_key_context && (analysis.empty || analysis.multiline))
            && ((self.flow_level > 0 && analysis.allow_flow_plain)
                || (self.flow_level == 0 && analysis.allow_block_plain))
        {
            return Style::Plain;
        }
        if analysis.allow_single_quoted && !(self.simple_key_context && analysis.multiline) {
            return Style::Single;
        }
        Style::Double
    }
}
