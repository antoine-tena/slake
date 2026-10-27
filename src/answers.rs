//! The answers of a render, as Copier's `AnswersMap` gathers them, and the context every
//! template is rendered with.

use std::path::Path;

use minijinja::Value;

use crate::config::Config;
use crate::data::{Data, Map};
use crate::template::Revision;

/// Copier's `AnswersMap`.
#[derive(Default)]
pub struct Answers {
    /// The answers the questionnaire settled, cast.
    pub user: Map,
    /// The answers given (Copier's `data`), as they came.
    pub init: Map,
    /// `_src_path` and `_commit`.
    pub metadata: Map,
    /// The project's answers file, on an update.
    pub last: Map,
    /// The user's defaults.
    pub user_defaults: Map,
    /// The questions whose `when` is false: not recorded.
    pub hidden: Vec<String>,
}

impl Answers {
    /// Copier's `combined`, a `ChainMap` made a `dict`: keys in the order the sources of lowest
    /// priority first list them, each with the value of the source of highest priority.
    pub fn combined(&self) -> Map {
        let mut combined = Map::from_iter([(Data::str("_external_data"), Data::Map(Map::new()))]);
        for source in [
            &self.user_defaults,
            &self.last,
            &self.metadata,
            &self.init,
            &self.user,
        ] {
            for (key, value) in source.iter() {
                combined.insert(key.clone(), value.clone());
            }
        }
        combined
    }

    /// Copier's `_answers_to_remember`: `_commit`, `_src_path`, then the answer to every visible
    /// question whose value is of a JSON type (Copier checks the value itself, not its items).
    pub fn to_remember(&self, config: &Config) -> Map {
        let mut remembered = Map::new();
        for key in ["_commit", "_src_path"] {
            if let Some(value) = self.metadata.get(key) {
                remembered.insert(Data::str(key), value.clone());
            }
        }
        for (key, value) in self.combined() {
            let Data::Str(name) = &key else { continue };
            if config.has_question(name)
                && !name.starts_with('_')
                && !self.hidden.contains(name)
                && !matches!(value, Data::Date(_))
            {
                remembered.insert(key, value);
            }
        }
        remembered
    }
}

/// What every render context holds besides the answers.
pub struct Conf {
    pub dst_path: String,
    pub folder_name: String,
    pub vcs_ref: Option<String>,
    pub vcs_ref_hash: Option<String>,
    pub src_path: String,
    pub defaults: bool,
}

impl Conf {
    /// A render of `revision` into `destination`, every unanswered question taking its default.
    pub fn new(revision: &Revision, destination: &Path) -> Conf {
        Conf {
            dst_path: destination.to_string_lossy().into_owned(),
            folder_name: destination
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            vcs_ref: revision.vcs_ref.clone(),
            vcs_ref_hash: revision.sha.clone(),
            src_path: revision.src_path.clone(),
            defaults: true,
        }
    }
}

/// Copier's `_render_context`.
pub fn render_context(answers: &Answers, config: &Config, conf: &Conf, phase: &str) -> Value {
    let mut context = answers.combined();
    let remembered = answers.to_remember(config);
    context.insert(Data::str("_copier_answers"), Data::Map(remembered));
    context.insert(
        Data::str("_copier_conf"),
        Data::Map(copier_conf(answers, config, conf)),
    );
    context.insert(
        Data::str("_folder_name"),
        Data::str(conf.folder_name.as_str()),
    );
    context.insert(Data::str("_copier_phase"), Data::str(phase));
    Data::Map(context).to_value()
}

/// `_copier_conf`: the settings of the render, those a template could read.
fn copier_conf(answers: &Answers, config: &Config, conf: &Conf) -> Map {
    let optional = |value: &Option<String>| value.clone().map_or(Data::Null, Data::Str);
    let skip_if_exists = config
        .skip_if_exists
        .iter()
        .map(|item| Data::str(item.as_str()));
    let os = if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    let settings = [
        ("src_path", Data::str(conf.src_path.as_str())),
        ("dst_path", Data::str(conf.dst_path.as_str())),
        ("answers_file", Data::str(crate::template::ANSWERS_FILE)),
        ("vcs_ref", optional(&conf.vcs_ref)),
        ("vcs_ref_hash", optional(&conf.vcs_ref_hash)),
        ("data", Data::Map(answers.init.clone())),
        ("defaults", Data::Bool(conf.defaults)),
        ("user_defaults", Data::Map(answers.user_defaults.clone())),
        ("skip_if_exists", Data::List(skip_if_exists.collect())),
        ("sep", Data::str("/")),
        ("os", Data::str(os)),
    ];
    settings
        .into_iter()
        .map(|(key, value)| (Data::str(key), value))
        .collect()
}
