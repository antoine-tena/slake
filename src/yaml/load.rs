//! PyYAML's `safe_load`: YAML 1.1 scalars, anchors and aliases, `<<` merge keys.

use std::collections::HashMap;

use saphyr_parser::{Event, Parser, ScalarStyle, Tag as EventTag};

use super::construct::{construct, explicit_tag};
use super::resolve::{Tag, resolve};
use crate::data::{Data, Map};

enum Frame {
    Seq(Vec<Data>, usize),
    Map {
        pairs: Vec<(Data, Data, bool)>,
        key: Option<(Data, bool)>,
        anchor: usize,
    },
}

struct Composer {
    anchors: HashMap<usize, Data>,
    stack: Vec<Frame>,
    documents: Vec<Data>,
}

/// One document, as `yaml.safe_load`: an empty stream is `None`.
pub fn load(text: &str) -> Result<Data, String> {
    let mut documents = load_all(text)?;
    match documents.len() {
        0 => Ok(Data::Null),
        1 => Ok(documents.remove(0)),
        _ => Err("expected a single document in the stream".into()),
    }
}

/// Every document, as `yaml.safe_load_all`.
pub fn load_all(text: &str) -> Result<Vec<Data>, String> {
    let mut composer = Composer {
        anchors: HashMap::new(),
        stack: Vec::new(),
        documents: Vec::new(),
    };
    for event in Parser::new_from_str(text) {
        let (event, span) = event.map_err(|error| error.to_string())?;
        composer
            .event(event)
            .map_err(|error| format!("{error} (line {})", span.start.line()))?;
    }
    Ok(composer.documents)
}

fn tag_name(tag: &EventTag) -> String {
    if tag.handle == "!!" || tag.handle == "tag:yaml.org,2002:" {
        format!("tag:yaml.org,2002:{}", tag.suffix)
    } else {
        format!("{}{}", tag.handle, tag.suffix)
    }
}

impl Composer {
    fn event(&mut self, event: Event) -> Result<(), String> {
        match event {
            Event::Scalar(value, style, anchor, tag) => self.scalar(&value, style, anchor, tag),
            Event::Alias(id) => {
                let data = self
                    .anchors
                    .get(&id)
                    .cloned()
                    .ok_or("found undefined alias")?;
                self.node(data, 0, false)
            }
            Event::SequenceStart(anchor, tag) => {
                check_collection_tag(tag.as_deref(), "seq")?;
                self.stack.push(Frame::Seq(Vec::new(), anchor));
                Ok(())
            }
            Event::MappingStart(anchor, tag) => {
                check_collection_tag(tag.as_deref(), "map")?;
                let (pairs, key) = (Vec::new(), None);
                self.stack.push(Frame::Map { pairs, key, anchor });
                Ok(())
            }
            Event::SequenceEnd | Event::MappingEnd => self.end_collection(),
            Event::DocumentStart(_) => {
                self.anchors.clear();
                Ok(())
            }
            Event::StreamStart | Event::StreamEnd | Event::DocumentEnd | Event::Nothing => Ok(()),
        }
    }

    fn scalar(
        &mut self,
        value: &str,
        style: ScalarStyle,
        anchor: usize,
        tag: Option<std::borrow::Cow<'_, EventTag>>,
    ) -> Result<(), String> {
        let is_plain = matches!(style, ScalarStyle::Plain);
        let resolved = match tag {
            Some(tag) => explicit_tag(&tag_name(&tag), is_plain)?,
            None => resolve(value, is_plain),
        };
        let data = construct(value, resolved)?;
        self.node(data, anchor, resolved == Tag::Merge)
    }

    fn end_collection(&mut self) -> Result<(), String> {
        match self.stack.pop() {
            Some(Frame::Seq(items, anchor)) => self.node(Data::List(items), anchor, false),
            Some(Frame::Map { pairs, anchor, .. }) => {
                let map = flatten(pairs)?;
                self.node(Data::Map(map), anchor, false)
            }
            None => Err("unexpected end of a collection".into()),
        }
    }

    fn node(&mut self, data: Data, anchor: usize, merge: bool) -> Result<(), String> {
        if anchor != 0 {
            self.anchors.insert(anchor, data.clone());
        }
        match self.stack.last_mut() {
            None => self.documents.push(data),
            Some(Frame::Seq(items, _)) => items.push(data),
            Some(Frame::Map { pairs, key, .. }) => match key.take() {
                None => {
                    if matches!(data, Data::List(_) | Data::Map(_)) && !merge {
                        return Err("found unhashable key".into());
                    }
                    *key = Some((data, merge));
                }
                Some((key, merge)) => pairs.push((key, data, merge)),
            },
        }
        Ok(())
    }
}

fn check_collection_tag(tag: Option<&EventTag>, kind: &str) -> Result<(), String> {
    match tag {
        None => Ok(()),
        Some(tag) if tag_name(tag) == format!("tag:yaml.org,2002:{kind}") => Ok(()),
        Some(tag) => Err(format!("unsupported tag {}", tag_name(tag))),
    }
}

/// PyYAML's `flatten_mapping`: merged keys first, the mapping's own keys win.
fn flatten(pairs: Vec<(Data, Data, bool)>) -> Result<Map, String> {
    let mut merged: Vec<(Data, Data)> = Vec::new();
    let mut own: Vec<(Data, Data)> = Vec::new();
    for (key, value, merge) in pairs {
        if !merge {
            own.push((key, value));
            continue;
        }
        match value {
            Data::Map(map) => merged.extend(map),
            Data::List(items) => {
                let mut maps = Vec::new();
                for item in items {
                    match item {
                        Data::Map(map) => maps.push(map),
                        _ => return Err("expected a mapping for merging".into()),
                    }
                }
                for map in maps.into_iter().rev() {
                    merged.extend(map);
                }
            }
            _ => return Err("expected a mapping or list of mappings for merging".into()),
        }
    }
    Ok(merged.into_iter().chain(own).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> String {
        format!("{:?}", load(text).unwrap())
    }

    #[test]
    fn scalars_take_yaml_1_1_types() {
        assert_eq!(one("on"), "Bool(true)");
        assert_eq!(one("'on'"), "Str(\"on\")");
        assert_eq!(one("0755"), "Int(493)");
        assert_eq!(one("1_000"), "Int(1000)");
        assert_eq!(one("1:30"), "Int(90)");
        assert_eq!(one("1e3"), "Str(\"1e3\")");
        assert_eq!(one("1.5e+3"), "Float(1500.0)");
        assert_eq!(one("~"), "Null");
        assert_eq!(one("none"), "Str(\"none\")");
        assert_eq!(one("2026-10-26"), "Date(\"2026-10-26\")");
    }

    #[test]
    fn merge_keys_put_merged_keys_first() {
        let data = load("base: &b {x: 1, y: 2}\nchild:\n  y: 3\n  <<: *b\n").unwrap();
        let Data::Map(map) = data else { panic!() };
        assert_eq!(
            format!("{:?}", map.get("child").unwrap()),
            format!("{:?}", load("{x: 1, y: 3}").unwrap())
        );
    }
}
