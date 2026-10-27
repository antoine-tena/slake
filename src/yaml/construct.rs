//! PyYAML's `SafeConstructor` for scalars: the value of a resolved or explicit tag.

use super::resolve::Tag;
use crate::data::Data;

pub fn explicit_tag(name: &str, plain: bool) -> Result<Tag, String> {
    Ok(match name {
        "!" => Tag::Str,
        "tag:yaml.org,2002:str" => Tag::Str,
        "tag:yaml.org,2002:int" => Tag::Int,
        "tag:yaml.org,2002:float" => Tag::Float,
        "tag:yaml.org,2002:bool" => Tag::Bool,
        "tag:yaml.org,2002:null" => Tag::Null,
        "tag:yaml.org,2002:timestamp" => Tag::Timestamp,
        "tag:yaml.org,2002:merge" if plain => Tag::Merge,
        other => return Err(format!("unsupported tag {other}")),
    })
}

pub fn construct(value: &str, tag: Tag) -> Result<Data, String> {
    Ok(match tag {
        Tag::Str | Tag::Merge => Data::Str(value.to_string()),
        Tag::Null => Data::Null,
        Tag::Bool => Data::Bool(matches!(
            value.to_lowercase().as_str(),
            "yes" | "true" | "on"
        )),
        Tag::Int => Data::Int(construct_int(value)?),
        Tag::Float => Data::Float(construct_float(value)?),
        Tag::Timestamp => construct_timestamp(value)?,
        Tag::Value | Tag::Yaml => {
            return Err(format!(
                "could not determine a constructor for the tag {}",
                tag.uri()
            ));
        }
    })
}

fn split_sign(value: &str) -> (i64, &str) {
    match value.as_bytes().first() {
        Some(b'-') => (-1, &value[1..]),
        Some(b'+') => (1, &value[1..]),
        _ => (1, value),
    }
}

fn construct_int(value: &str) -> Result<i64, String> {
    let cleaned = value.replace('_', "");
    let (sign, digits) = split_sign(&cleaned);
    let too_large = || format!("integer too large: {value}");
    let parsed = if digits == "0" {
        0
    } else if let Some(binary) = digits.strip_prefix("0b") {
        i64::from_str_radix(binary, 2).map_err(|_| too_large())?
    } else if let Some(hex) = digits.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).map_err(|_| too_large())?
    } else if digits.starts_with('0') {
        i64::from_str_radix(digits, 8).map_err(|_| too_large())?
    } else if digits.contains(':') {
        let mut total: i64 = 0;
        for part in digits.split(':') {
            let part: i64 = part.parse().map_err(|_| too_large())?;
            total = total
                .checked_mul(60)
                .and_then(|t| t.checked_add(part))
                .ok_or_else(too_large)?;
        }
        total
    } else {
        digits.parse().map_err(|_| too_large())?
    };
    Ok(sign * parsed)
}

fn construct_float(value: &str) -> Result<f64, String> {
    let cleaned = value.replace('_', "").to_lowercase();
    let (sign, digits) = split_sign(&cleaned);
    let sign = sign as f64;
    Ok(match digits {
        ".inf" => sign * f64::INFINITY,
        ".nan" => f64::NAN,
        _ if digits.contains(':') => {
            let mut total = 0.0;
            for part in digits.split(':') {
                total = total * 60.0 + part.parse::<f64>().map_err(|error| error.to_string())?;
            }
            sign * total
        }
        _ => sign * digits.parse::<f64>().map_err(|error| error.to_string())?,
    })
}

fn construct_timestamp(value: &str) -> Result<Data, String> {
    let parts: Vec<&str> = value.trim_end_matches('\n').split('-').collect();
    if parts.len() == 3
        && parts
            .iter()
            .all(|part| part.chars().all(|ch| ch.is_ascii_digit()))
    {
        let numbers: Vec<u32> = parts.iter().map(|part| part.parse().unwrap_or(0)).collect();
        if chrono::NaiveDate::from_ymd_opt(numbers[0] as i32, numbers[1], numbers[2]).is_none() {
            return Err(format!("invalid date {value}"));
        }
        return Ok(Data::Date(format!(
            "{:04}-{:02}-{:02}",
            numbers[0], numbers[1], numbers[2]
        )));
    }
    Err(format!("timestamps with a time are not supported: {value}"))
}
