//! PyYAML's implicit resolver: the type a plain scalar takes in YAML 1.1.

use std::sync::LazyLock;

use regex::Regex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Bool,
    Float,
    Int,
    Merge,
    Null,
    Timestamp,
    Value,
    Yaml,
    Str,
}

impl Tag {
    pub fn uri(self) -> &'static str {
        match self {
            Tag::Bool => "tag:yaml.org,2002:bool",
            Tag::Float => "tag:yaml.org,2002:float",
            Tag::Int => "tag:yaml.org,2002:int",
            Tag::Merge => "tag:yaml.org,2002:merge",
            Tag::Null => "tag:yaml.org,2002:null",
            Tag::Timestamp => "tag:yaml.org,2002:timestamp",
            Tag::Value => "tag:yaml.org,2002:value",
            Tag::Yaml => "tag:yaml.org,2002:yaml",
            Tag::Str => "tag:yaml.org,2002:str",
        }
    }
}

// Python's `$` also matches before a final newline: `\n?\z` does the same here.
static RESOLVERS: LazyLock<Vec<(Tag, Regex, &'static str)>> = LazyLock::new(|| {
    let rules: [(Tag, &str, &str); 8] = [
        (
            Tag::Bool,
            r"^(?:yes|Yes|YES|no|No|NO|true|True|TRUE|false|False|FALSE|on|On|ON|off|Off|OFF)\n?\z",
            "yYnNtTfFoO",
        ),
        (
            Tag::Float,
            r"^(?:[-+]?(?:[0-9][0-9_]*)\.[0-9_]*(?:[eE][-+][0-9]+)?|\.[0-9][0-9_]*(?:[eE][-+][0-9]+)?|[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+\.[0-9_]*|[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN))\n?\z",
            "-+0123456789.",
        ),
        (
            Tag::Int,
            r"^(?:[-+]?0b[0-1_]+|[-+]?0[0-7_]+|[-+]?(?:0|[1-9][0-9_]*)|[-+]?0x[0-9a-fA-F_]+|[-+]?[1-9][0-9_]*(?::[0-5]?[0-9])+)\n?\z",
            "-+0123456789",
        ),
        (Tag::Merge, r"^(?:<<)\n?\z", "<"),
        (Tag::Null, r"^(?:~|null|Null|NULL|)\n?\z", "~nN"),
        (
            Tag::Timestamp,
            r"^(?:[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]|[0-9][0-9][0-9][0-9]-[0-9][0-9]?-[0-9][0-9]?(?:[Tt]|[ \t]+)[0-9][0-9]?:[0-9][0-9]:[0-9][0-9](?:\.[0-9]*)?(?:[ \t]*(?:Z|[-+][0-9][0-9]?(?::[0-9][0-9])?))?)\n?\z",
            "0123456789",
        ),
        (Tag::Value, r"^(?:=)\n?\z", "="),
        (Tag::Yaml, r"^(?:!|&|\*)\n?\z", "!&*"),
    ];
    rules
        .into_iter()
        .map(|(tag, pattern, first)| (tag, Regex::new(pattern).expect("resolver regex"), first))
        .collect()
});

/// The tag PyYAML gives a plain scalar (`plain`), or the default one for a quoted scalar.
pub fn resolve(value: &str, plain: bool) -> Tag {
    if !plain {
        return Tag::Str;
    }
    match value.chars().next() {
        // Only the null resolver is registered for the empty string.
        None => Tag::Null,
        Some(first) => RESOLVERS
            .iter()
            .filter(|(_, _, firsts)| firsts.contains(first))
            .find(|(_, regex, _)| regex.is_match(value))
            .map(|(tag, _, _)| *tag)
            .unwrap_or(Tag::Str),
    }
}
