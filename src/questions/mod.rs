//! Copier's questionnaire without a prompt (`_ask` with `--defaults`): each question in order,
//! answered by the given answers, the last ones or its default, cast to its type, checked
//! against its choices and its validator.

mod choices;
mod types;

use minijinja::Value;

use crate::answers::{Answers, Conf, render_context};
use crate::cast::cast_to_bool;
use crate::config::Config;
use crate::data::{Data, Map};
use crate::error::{Error, Kind, Result};
use crate::jinja::Engine;

/// One question of `copier.yml`, as Copier's `Question`.
struct Question<'a> {
    name: &'a str,
    details: &'a Map,
    context: Value,
    engine: &'a Engine,
}

impl Question<'_> {
    fn field(&self, key: &str) -> Option<&Data> {
        self.details.get(key)
    }

    fn render_with(&self, value: &Data, extra: Option<(&str, &Data)>) -> Result<Data> {
        match value {
            Data::Str(source) => {
                let context = match extra {
                    None => self.context.clone(),
                    Some((key, answer)) => merged_context(&self.context, key, answer),
                };
                self.engine
                    .render_str(source, &context)
                    .map(Data::Str)
                    .map_err(|error| Error::template(format!("question {}: {error}", self.name)))
            }
            Data::List(items) => Ok(Data::List(
                items
                    .iter()
                    .map(|item| self.render_with(item, extra))
                    .collect::<Result<_>>()?,
            )),
            other => Ok(other.clone()),
        }
    }

    fn render(&self, value: &Data) -> Result<Data> {
        self.render_with(value, None)
    }

    /// Copier's `validate_answer`: the validator renders nothing for a valid answer.
    fn validate(&self, answer: &Data) -> Result<()> {
        let Some(validator) = self.field("validator") else {
            return Ok(());
        };
        // Copier takes a validator that fails to render as the error message.
        let message = match self.render_with(validator, Some((self.name, answer))) {
            Ok(rendered) => rendered.to_string(),
            Err(error) => error.message,
        };
        let message = message.trim_matches(crate::jinja::is_space);
        if message.is_empty() {
            Ok(())
        } else {
            Err(Error::answer(self.name, message))
        }
    }

    fn when(&self) -> Result<bool> {
        match self.field("when") {
            None => Ok(true),
            Some(when) => Ok(cast_to_bool(&self.render(when)?)),
        }
    }

    /// Copier's `get_default`; `None` when the question has none.
    fn default(&self, answers: &Answers) -> Result<Option<Data>> {
        let given = [&answers.init, &answers.last, &answers.user_defaults]
            .into_iter()
            .find_map(|source| source.get(self.name));
        let result = match given {
            Some(value) => value.clone(),
            None => match self.field("default") {
                None => return Ok(None),
                Some(default) => self.render(default)?,
            },
        };
        let result = self.parse(&result)?;
        if self.when()? {
            self.validate(&result)?;
        }
        Ok(Some(result))
    }
}

fn merged_context(context: &Value, key: &str, answer: &Data) -> Value {
    let mut pairs: Vec<(Value, Value)> = Vec::new();
    for name in context.try_iter().into_iter().flatten() {
        if name.as_str() == Some(key) {
            continue;
        }
        let value = context.get_item(&name).unwrap_or_default();
        pairs.push((name, value));
    }
    pairs.push((Value::from(key), answer.to_value()));
    Value::from_pairs(pairs)
}

/// Copier's `_ask` with `defaults=True` and `skip_answered`: every question answered without a
/// prompt, or an `answers` error.
pub fn ask(
    engine: &Engine,
    config: &Config,
    answers: &mut Answers,
    conf: &Conf,
    skip_answered: bool,
) -> Result<()> {
    for (name, details) in &config.questions {
        let question = Question {
            name,
            details,
            context: render_context(answers, config, conf, "prompt"),
            engine,
        };
        forget_invalid_last(&question, answers);
        if !question.when()? {
            answers.hidden.push(name.clone());
            answers.last.remove(name);
            if question.default(answers)?.is_none() {
                continue;
            }
        }
        settle(&question, answers, skip_answered)?;
    }
    Ok(())
}

/// A last answer that no longer parses or validates is asked again.
fn forget_invalid_last(question: &Question, answers: &mut Answers) {
    if let Some(last) = answers.last.get(question.name).cloned()
        && question
            .parse(&last)
            .and_then(|answer| question.validate(&answer))
            .is_err()
    {
        answers.last.remove(question.name);
    }
}

/// The answer of a question: the one given, the last one, or its default.
fn settle(question: &Question, answers: &mut Answers, skip_answered: bool) -> Result<()> {
    let name = question.name;
    let answer = if let Some(given) = answers.init.get(name).cloned() {
        let answer = question.parse(&given)?;
        question.validate(&answer)?;
        answer
    } else if skip_answered && answers.last.contains_key(name) {
        return Ok(());
    } else {
        question
            .default(answers)?
            .ok_or_else(|| Error::new(Kind::Answers, format!("question {name} is required")))?
    };
    answers.user.insert(Data::str(name), answer);
    Ok(())
}
