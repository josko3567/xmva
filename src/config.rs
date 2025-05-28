use std::{path::{Path, PathBuf}, sync::{Arc, RwLock}};

use colored::Colorize;
use lazy_static::lazy_static;
use strum::{IntoEnumIterator, EnumProperty, EnumIter};
use serde::{Deserialize, Deserializer, Serialize};

use crate::preprocessor::{Preprocessable, PreprocessableName, PreprocessableString};

const MAX_REPEATS: usize = 10000; // so i dont accidentaly eat my entire ssd

#[derive(Debug)]
pub enum Error {
    IO   {file: PathBuf, message: String},
    TOML {file: PathBuf, message: String, line: Option<(usize, usize)>},
    // KeySerialization {message: String},
    // KeyCompilation {key: String, name: String, message: String},
    // KeyPreprocessing {key: String, name: String, message: String},
    // KeyMutualReferencing {key_names: Vec<(String, PreprocessableName)>}
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        _ = write!(f, "[config.rs] ");
        match self {
            Self::IO { file, message } => {
                write!(f, "in config file {:?}: {message}", file)
            }
            Self::TOML { file, message, line } => {
                write!(f, "in config file {:?}: {message}{}",
                    file, 
                    if line.is_some() {
                        format!(" between lines {}-{}", line.unwrap().0, line.unwrap().1)
                    } else {
                        "".to_owned()
                    }
                )
            }
            // Self::KeySerialization { message } => {
            //     write!(f, "error while compiling keys: `{message}`")
            // }
            // Self::KeyCompilation { key, name, message } => {
            //     write!(f, "error while trying to compile key `{key}` with name `{name}`: `{message}`")
            // } 
            // Self::KeyPreprocessing { key, name, message } => {
            //     write!(f, "error while trying to compile key `{key}` with name `{name}`: `{message}`")
            // } 
            // Self::KeyMutualReferencing { key_names } => {
            //     _ = write!(f, "could not compile all keys as some have a mutual reference. \n");
            //     _ = write!(f, "here is a list of uncompiled keys: \n");
            //     for (key, name) in key_names {
            //         _ = write!(f, "{{Key: {key}, Name: {:?}}}", name);
            //     }
            //     write!(f, "")
            // }
        }
    }
}

impl std::error::Error for Error {}

/// Common configuration values that can be referenced with their name
/// being the key.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommonKeyable {
    pub prefix: String
}

/// Common configuration values shared across the entire process of
/// preprocessing and compiling.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Common {
    #[serde(flatten)]
    pub keyable: CommonKeyable,

    /// Output file path, overwritten by CLI if a output file
    /// is given via. CLI.
    /// TODO: Make it so that if no output path is given (here or cli)
    /// take the config name and change the extension to .h for output.
    pub output:  Option<PathBuf>,

    /// No. of times the repeat pattern in the [Generator] is
    /// repeated.
    pub repeats: usize
}

/// [Tag]s that the user adds along side a `name` string, these 
/// get translated into [Todo]s which are just a list of things 
/// to do to a `name`.
/// 
/// [Tag]s can either remove preset [Todo]s or add new [Todo]s.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Tag {
    NoPrefix
}

/// A list of things to do to a `name`.
/// 
/// These get translated from [Tag] because [Todo]s
/// are only applied to a `name` while [Tag]s either 
/// remove preset [Todo]s or add new ones.
/// 
/// For example, [Tag::NoPrefix] will remove [Todo::ApplyPrefix] from the list
/// of actions that would normally be automatically applied to a `name`.
/// This means you can use tags to customize or override the default behavior
/// for how `name`s are processed.
/// 
/// Done cause it's much more readable than applying pure [Tag]s to a `name`,
/// and because this forces both [Tag] to be translated via. a `match`
/// statement and [Todo]s to be applied via. a `match` statement aswell
/// so that the programmer (me O_O) remembers to update all the apropriate
/// functions.
/// 
/// Adding a [strum::EnumProperty] named `preset` and setting it to `true`
/// will automatically apply this [Todo] to all `name`s unless removed
/// by a [Tag].
#[derive(EnumIter, EnumProperty, Clone, Copy, PartialEq, Eq)]
pub(self) enum Todo {
    #[strum(props(preset = true))]
    ApplyPrefix
}

lazy_static! {
    /// A list of preset [Todo]s executed for every `name` unless
    /// a [Tag] removes it in [Todo::from_tags_with_preset]. 
    static ref PRESET_TODO: Vec<Todo> = {
        let mut preset_todo_vec: Vec<Todo> = vec![];
        for todo in Todo::iter() {
            if todo.get_bool("preset").is_some_and(|preset| preset == true) {
                preset_todo_vec.push(todo);
            }
        }
        preset_todo_vec
    };
}

impl Todo {

    /// Convert a [Vec] of [Tag] into a [Vec] of [Todo].
    fn from_tags_with_presets(tags: &Vec<Tag>) -> Vec<Self> {
        
        let mut todo_vec: Vec<Self> = PRESET_TODO.clone();

        for tag in tags {

            match *tag {
                Tag::NoPrefix => {
                    if todo_vec.contains(&Todo::ApplyPrefix) {
                        todo_vec.retain(|todo| 
                            *todo != Todo::ApplyPrefix
                        );
                    }
                }
            }

        }

        todo_vec
    }

}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StringWithTags {
    pub tags: Vec<Tag>,
    pub string: String,
}

impl StringWithTags {

    // Translates [Tag]s into [Todo]s and applies
    // the [Todo]s to the string.
    // `common_keys` is needed for certaint [Tag]s.
    pub fn apply_tags(
        &self,
        common_keys: &CommonKeyable
    ) -> String {

        let mut tagged_string = self.string.clone();

        let todo_vec = Todo::from_tags_with_presets(&self.tags);

        for todo in todo_vec {

            match todo {
                Todo::ApplyPrefix => {
                    tagged_string = common_keys.prefix.to_owned() + &tagged_string
                }
            }

        }

        tagged_string

    }

}

/// A name of either a [Definition] or a [Key].
/// Names can come included with [Tag]s to disable or add some [Todo]s
/// to be done to the name.
/// In [Definition] they are the name of the definition while in [Key] they
/// can litteraly refer to anything that this program generates or anything
/// external (in C).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Name {
    Raw(String),
    Tagged(StringWithTags)
}

impl Default for Name {
    fn default() -> Self {
        Self::Raw(String::new())
    }
}

/// This deserializer flattens [PreprocessableName] 
/// and automatically stores the [Name] inside of
/// [Preprocessable::NotPreprocessed].
fn preprocessable_name_deserializer<'de, D>(
    deserializer: D
) -> Result<PreprocessableName, D::Error>
where
    D: Deserializer<'de>,
{
    let unprocessed_name = Name::deserialize(deserializer)?;
    Ok(Arc::new(RwLock::new(Preprocessable::NotPreprocessed(unprocessed_name))))
}

/// This deserializer flattens [PreprocessableString] 
/// and automatically stores the [String] inside of
/// [Preprocessable::NotPreprocessed].
fn preprocessable_string_deserializer<'de, D>(
    deserializer: D
) -> Result<PreprocessableString, D::Error>
where
    D: Deserializer<'de>,
{
    let unprocessed_string = String::deserialize(deserializer)?;
    Ok(Arc::new(RwLock::new(Preprocessable::NotPreprocessed(unprocessed_string))))
}

/// Same as [preprocessable_string_deserializer] but with a [Option].
fn preprocessable_option_string_deserializer<'de, D>(
    deserializer: D
) -> Result<Option<PreprocessableString>, D::Error>
where
    D: Deserializer<'de>,
{
    let optional_unprocessed_string = Option::<String>::deserialize(deserializer)?;
    match optional_unprocessed_string {
        Some(string) => Ok(Some(Arc::new(RwLock::new(Preprocessable::NotPreprocessed(string))))),
        None => Ok(None)
    }
}

/// A `#define` from C.
/// 
/// [Definition::key] is a reference to [Definition::name].
/// 
/// Reasoning for [`Definition::key`]
/// ---------------------------------
/// There are 2 reasons to use [Definition::key]: 
/// 1. `Refactorability`: If we ever change [Definition::name] 
///    we don't have to update other parts of our config that 
///    reference that [Definition].
/// 2. `Consistency`: [Definition::name]s will change due to [Todo]s,
///    so we can't be sure what the final name will be. [Definition::key] 
///    can only be changed by the user and nothing else.
/// 
/// Example
/// -------
/// ```C
/// /* This is how a Definition will look in C */
/// 
/// /* With Definition::parameters being Some */
/// #define name(parameters) expansion
/// 
/// /* With Definition::parameters being None */
/// #define name expansion
/// ```
#[derive(Deserialize, Debug, Clone)]
pub struct Definition {
    pub key:        String,
    #[serde(deserialize_with = "preprocessable_name_deserializer")]
    pub name:       PreprocessableName,
    pub parameters: Option<Vec<String>>,
    #[serde(deserialize_with = "preprocessable_string_deserializer")]
    pub expansion:  PreprocessableString,
}

/// Keys that might reference anything from another C file or the
/// the code generated with this executable and a config.
#[derive(Deserialize, Debug, Clone)]
pub struct Key {
    pub key:  String,
    #[serde(deserialize_with = "preprocessable_name_deserializer")]
    pub name: PreprocessableName
}

/// Custom preamble that is inserted as is (first preprocessed tho).
#[derive(Deserialize, Debug, Clone)]
pub struct Preamble {
    #[serde(deserialize_with = "preprocessable_option_string_deserializer")]
    pub raw:  Option<PreprocessableString>,
    pub keys: Option<Vec<Key>>,
}

/// Fallbacks the [Generator] uses when encountering strange varadict
/// argument counts.
#[derive(Deserialize, Debug, Clone)]
pub struct Fallbacks {
    #[serde(deserialize_with = "preprocessable_string_deserializer")]
    /// What to do when the varadict argument count is not a multiple
    /// of [Paramaters::Varadict] in [Core::args].
    pub unparity: PreprocessableString,

    #[serde(deserialize_with = "preprocessable_string_deserializer")]
    /// What to do when the varadict argument count is 0?
    pub empty: PreprocessableString,
}

/// In this XMVA macro i've invisioned there is but one catch,
/// there must exist a x-macro for every possible varadict argument
/// count that the XMVA macro may encounter.
/// 
/// This structure configures a generator that generates these
/// x-macros that then handle the creation of your code for every
/// varadict argument count up to [`Common::repeats`] amount of
/// varadict arguments.
#[derive(Deserialize, Debug, Clone)]
pub struct Generator {
    /// On strange varadict argument counts, set what the generated
    /// x-macro will write out.
    pub fallbacks: Fallbacks,
    
    /// What to write before the repeat part.
    #[serde(deserialize_with = "preprocessable_string_deserializer")]
    pub preamble: PreprocessableString,
    
    /// Repeat represents a string that can contain arguments passed into
    /// the `xmva` itself and that will be repeated multiple times in the
    /// generated code.
    /// 
    /// This in sense is the core of a `xmva`.
    /// 
    /// Depending on the amount of varadict arguments a `xmva` receives
    /// it will call a x-macro in which this  string is repeated a
    /// certaint amount of times (it will repeat for 
    /// [Argument::Varadict]/`varadict argument count` the `xmva` recived).
    /// 
    /// Special sigils
    /// --------------
    /// It itself is a [PreprocessableString] meaning it can contain special
    /// sigils from [crate::sigil::PreprocessorSigil] but it also can contain 
    /// special sigils from [crate::sigil::CompilerSigil]:
    /// 
    /// - `${...}`
    ///     tells us where to place a named argument:
    ///     `... ${lowercase_name} ... ${UPPERCASE_NAME}`
    /// 
    /// - `$(...)`
    ///     tells us where to place a varadict argument: 
    ///     `... $(0) ... $(1) ...`
    /// 
    /// - `$[...]`
    ///     tells us to repeat this character except on the last repeat:
    ///     `... $[,] ... $[peepee poopoo] ...`
    /// 
    /// Example
    /// -------
    /// ```TOML
    /// # Lets say that args -> {varadict = 2}.
    /// # Which means we require a minimum of 2 varadict arguments.
    /// # Here our first argument out of our varadict argument pairs
    /// # will be placed before a comma.
    /// # This pattern will repeat like this:
    /// # _00
    /// # _00, _02
    /// # _00, _02, 04
    /// # ...
    /// repeat = "$(0)[,]"
    /// 
    /// # Heres something a tad bit more complex.
    /// # Let's say ${prefix} is `YA_`.
    /// # This pattern will repeat like this:
    /// # [YA_ ## _00] = _01
    /// # [YA_ ## _00] = _01, [YA_ ## _02] = _03
    /// # [YA_ ## _00] = _01, [YA_ ## _02] = _03, [YA_ ## _04] = _05
    /// # ...
    /// repeat = "[@{prefix} ## $(0)] = $(1)$[,]"
    /// ```
    #[serde(deserialize_with = "preprocessable_string_deserializer")]
    pub repeat: PreprocessableString,

    // What to write after the repeat part.
    #[serde(deserialize_with = "preprocessable_string_deserializer")]
    pub postamble: PreprocessableString
}

#[derive(Deserialize, Debug, Clone)]
pub struct NamedArgument {
    pub key: String,
    #[serde(deserialize_with = "preprocessable_string_deserializer")]
    pub name: PreprocessableString
}

/// Types of parameters we pass to our `xmva`.
/// 
/// [Argument::Named] is a named parameter the `xmva` will accept and 
/// have available in the [Generator::repeat] section.
/// 
/// [Argument::Varadict] represents the minimum pair size of varadict arguments.
/// If [Argument::Varadict] =  `2` for example, we can accept 
/// 2n arguments where n >= 1 and represents a argument pair.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Argument {
    Named(NamedArgument),
    Varadict {varadict: usize}
}

/// The [Core] which holds the main XMVA name and arguments.
#[derive(Deserialize, Debug, Clone)]
pub struct Core {
    /// The name of the `xmva` we want to create.
    #[serde(deserialize_with = "preprocessable_string_deserializer")]
    pub xmva: PreprocessableString,
    /// List of paramaters the `xmva` will accept
    /// including named parameters and the number of
    /// varadict arguments.
    pub args: Vec<Argument>,
}

/// The main config structure.
/// Each part of the [Config] and what they do are explained in their own docs.
/// 
/// Init
/// ----
/// This structure can be initialized from a `.xmva.toml` file.
/// ```
/// let config: Config = Config::load(&Path::new("example.xmva.toml"));
/// ```
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub common:     Common, 
    pub preamble:   Option<Preamble>,
    pub definition: Option<Vec<Definition>>,
    pub core:       Core,
    pub generator:  Vec<Generator>,
}

impl Config { 

    pub fn load(path: &Path) -> Result<Self, Error>{
        
        log::debug!("Starting to load config.");

        let file_contents = std::fs::read_to_string(path)
            .map_err(|fs_err| Error::IO { 
                file: path.to_owned(),
                message: fs_err.to_string() 
            })?;

        log::debug!("Loaded file into memory.");

        let mut config: Self = toml::from_str(&file_contents)
            .map_err(|toml_err| Error::TOML { 
                file: path.to_owned(),
                message: toml_err.message().to_owned(), 
                line: if toml_err.span().is_some() {
                    let offset_start = toml_err.span().unwrap().start;
                    let offset_end   = toml_err.span().unwrap().end;
                    let line_start   = file_contents[..offset_start].lines().count();
                    let line_end     = file_contents[..offset_end].lines().count();
                    Some((line_start, line_end))
                } else {
                    None
                }
            })?;

        // limit repeats
        config.common.repeats = std::cmp::min(MAX_REPEATS, config.common.repeats);

        if config.common.output.is_none() {
            config.common.output = Some(path.to_owned());
        }
        
        log::trace!("{}",
            format!("Config loaded: {:#?}", config)
            .dimmed()
        );

        Ok(config)

    }

}