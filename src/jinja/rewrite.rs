//! Python's `list.append`, which minijinja's immutable lists cannot do: the statement
//! `{% set _ = items.append(x) %}` becomes `{% set items = items + [x] %}`, the same outside a
//! loop or a macro, where Jinja2 and minijinja scope `set` alike.

use std::sync::LazyLock;

use regex::Regex;

static APPEND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{%(-?)\s*set\s+_\s*=\s*([A-Za-z_][A-Za-z_0-9]*)\.append\((.*?)\)\s*(-?)%\}")
        .expect("append regex")
});
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{%-?\s*(\w+)").expect("tag regex"));

const SCOPES: [(&str, &str); 5] = [
    ("for", "endfor"),
    ("macro", "endmacro"),
    ("call", "endcall"),
    ("filter", "endfilter"),
    ("with", "endwith"),
];

pub fn appends(source: &str) -> Result<String, String> {
    if !source.contains(".append(") {
        return Ok(source.to_string());
    }
    let mut out = String::with_capacity(source.len());
    let mut last = 0;
    for found in APPEND.captures_iter(source) {
        let statement = found.get(0).expect("whole match");
        if open_scopes(&source[..statement.start()]) > 0 {
            return Err("list.append() is only supported outside loops, macros and blocks".into());
        }
        let name = &found[2];
        out.push_str(&source[last..statement.start()]);
        out.push_str(&format!(
            "{{%{} set {name} = {name} + [{}] {}%}}",
            &found[1], &found[3], &found[4]
        ));
        last = statement.end();
    }
    out.push_str(&source[last..]);
    if out.contains(".append(") {
        return Err("list.append() is only supported as `{% set _ = items.append(x) %}`".into());
    }
    Ok(out)
}

fn open_scopes(text: &str) -> i32 {
    let mut depth = 0;
    for tag in TAG.captures_iter(text) {
        let word = &tag[1];
        if SCOPES.iter().any(|(open, _)| *open == word) {
            depth += 1;
        } else if SCOPES.iter().any(|(_, close)| *close == word) {
            depth -= 1;
        }
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::appends;

    #[test]
    fn an_append_becomes_a_concatenation() {
        let source = "{%- if a %}{% set _ = items.append(('uv', dirs)) %}{% endif %}";
        assert_eq!(
            appends(source).unwrap(),
            "{%- if a %}{% set items = items + [('uv', dirs)] %}{% endif %}"
        );
    }

    #[test]
    fn an_append_in_a_loop_is_refused() {
        assert!(appends("{% for x in y %}{% set _ = items.append(x) %}{% endfor %}").is_err());
        assert!(appends("{{ items.append(1) }}").is_err());
    }
}
