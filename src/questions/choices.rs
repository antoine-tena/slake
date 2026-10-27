//! A question's choices, as Copier's `_formatted_choices`, and `parse_answer`, which only takes
//! a choice that is not disabled.

use super::Question;
use crate::data::Data;
use crate::error::{Error, Result};
use crate::pyfmt::repr_data;

struct Choice {
    value: Data,
    /// Why the choice cannot be taken: its validator's message, empty when it can.
    disabled: String,
}

impl Question<'_> {
    /// Copier's `parse_answer`: cast, then one of the choices that are not disabled.
    pub(super) fn parse(&self, answer: &Data) -> Result<Data> {
        let answer = self.cast(answer)?;
        let choices = self.choices()?;
        if choices.is_empty() {
            return Ok(answer);
        }
        let mut valid = Vec::new();
        let mut refusal = String::new();
        for choice in &choices {
            let value = self.cast(&choice.value)?;
            if choice.disabled.is_empty() {
                valid.push(repr_data(&value));
            }
            if answer.py_eq(&value) {
                if choice.disabled.is_empty() {
                    return Ok(answer);
                }
                if refusal.is_empty() {
                    refusal = choice.disabled.clone();
                }
            }
        }
        if refusal.is_empty() {
            refusal = format!("{} is not in [{}]", repr_data(&answer), valid.join(", "));
        }
        Err(Error::answer(
            self.name,
            format!("Invalid choice: {refusal}"),
        ))
    }

    /// Copier's `_formatted_choices`: each choice's value, rendered and checked against the type.
    fn choices(&self) -> Result<Vec<Choice>> {
        let mut choices = Vec::new();
        for (name, value, is_pair) in self.choice_pairs()? {
            // Copier renders every choice's name, and fails on one that does not render.
            self.render(&name)?;
            let (value, disabled) = match value {
                Data::Map(extended) if is_pair => self.extended_choice(&extended)?,
                value => (value, String::new()),
            };
            let value = self.render(&value)?;
            self.cast(&value)?;
            choices.push(Choice { value, disabled });
        }
        Ok(choices)
    }

    /// The choices as `(name, value, is_pair)`: a mapping, or a list of values or of pairs.
    fn choice_pairs(&self) -> Result<Vec<(Data, Data, bool)>> {
        let template =
            |message: &str| Error::template(format!("question {}: {message}", self.name));
        match self.raw_choices()? {
            Data::Map(map) => Ok(map
                .into_iter()
                .map(|(name, value)| (name, value, true))
                .collect()),
            Data::List(items) => items
                .into_iter()
                .map(|item| match item {
                    Data::List(pair) if pair.len() == 2 => {
                        Ok((pair[0].clone(), pair[1].clone(), true))
                    }
                    Data::List(_) => Err(template("a choice pair needs two items")),
                    other => Ok((other.clone(), other, false)),
                })
                .collect(),
            Data::Null => Ok(Vec::new()),
            _ => Err(template("choices must be a list or a mapping")),
        }
    }

    /// The `choices` field; a string is rendered, then read as YAML.
    fn raw_choices(&self) -> Result<Data> {
        match self.field("choices") {
            None | Some(Data::Null) => Ok(Data::Null),
            Some(Data::Str(source)) => {
                let rendered = self.render(&Data::Str(source.clone()))?;
                crate::yaml::load(rendered.as_str().unwrap_or_default())
                    .map_err(|error| Error::answer(self.name, error))
            }
            Some(other) => Ok(other.clone()),
        }
    }

    /// A choice given as a mapping: its `value`, and its `validator` rendered.
    fn extended_choice(&self, extended: &crate::data::Map) -> Result<(Data, String)> {
        let value = extended.get("value").cloned().ok_or_else(|| {
            Error::template(format!(
                "question {}: property 'value' is required",
                self.name
            ))
        })?;
        let disabled = match extended.get("validator") {
            None => String::new(),
            Some(Data::Str(validator)) => self.render(&Data::Str(validator.clone()))?.to_string(),
            Some(_) => return Err(Error::template("property 'validator' must be a string")),
        };
        Ok((value, disabled))
    }
}
