use std::{clone, path::PathBuf, sync::{Arc, RwLock}};

use backtrace::Backtrace;
use lazy_static::lazy_static;
use serde::{Deserialize, Deserializer, Serialize};
use strum::{IntoEnumIterator, EnumIter, EnumProperty};
use toml::Spanned;
use crate::{compiler::Compilable, error::Error, preprocessor::Preprocessable};

/// Reflective is just a fancy name for a `Arc<RwLock<T>>`
/// both [Reflective::read] and [Reflective::write] are
/// integrated with [crate::error::Error] so we don't need
/// to do a `map_err` every damn time.
#[derive(Debug, Clone)]
pub struct Reflective<T>(Arc<RwLock<T>>);

impl<T> Reflective<T> {

    pub fn new(value: T) -> Reflective<T> {
        return Reflective(Arc::new(RwLock::new(value)))
    }

    pub fn read(&self) -> miette::Result<T> 
    where  
        T: Clone,
    {
        Ok(
            self.0
                .read()
                .map_err(|x| { 
                miette::Report::new(
                    Error::PoisonedLock { 
                        error: x.to_string(), 
                        backtrace: crate::backtrace!(Backtrace::new()) 
                    }
                )
                })?
                .clone()
        )
    }

    /// Write-lock and replace the inner value.
    pub fn write(&self, value: T) -> Result<(), crate::error::Error> {

        let mut inner = self.0
            .write()
            .map_err(|x| {
                Error::PoisonedLock { 
                    error: x.to_string(), 
                    backtrace: crate::backtrace!(Backtrace::new()) 
                }.into()
            })?;

        *inner = value;
        Ok(())
              
    }

}

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

////////////////////////////////////////////////////////////
// Custom de.
 
fn reflective_preprocessable_spanned_name_de<'de, D>(
    deserializer: D
) -> Result<Reflective<Preprocessable<Spanned<Name>>>, D::Error>
where D: Deserializer<'de>,
{
    let name = Spanned::<Name>::deserialize(deserializer)?;
    
    Ok(
        Reflective::new(Preprocessable::new(name))
    )
}

fn reflective_preprocessable_spanned_string_de<'de, D>(
    deserializer: D
) -> Result<Reflective<Preprocessable<Spanned<String>>>, D::Error>
where D: Deserializer<'de>,
{
    let string = Spanned::<String>::deserialize(deserializer)?;
    
    Ok(
        Reflective::new(Preprocessable::new(string))
    )
}

fn optional_reflective_preprocessable_spanned_string_de<'de, D>(
    deserializer: D
) -> Result<Option<Reflective<Preprocessable<Spanned<String>>>>, D::Error>
where D: Deserializer<'de>,
{
    let optional = Option::<Spanned<String>>::deserialize(deserializer)?;
    match optional {
        Some(string) => Ok(Some(Reflective::new(Preprocessable::new(string)))),
        None => Ok(None)
    }
}

// Custom de. end
////////////////////////////////////////////////////////////




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
    #[serde(deserialize_with = "reflective_preprocessable_spanned_name_de")]
    pub name:       Reflective<Preprocessable<Spanned<Name>>>,
    pub parameters: Option<Vec<String>>,
    #[serde(deserialize_with = "reflective_preprocessable_spanned_string_de")]
    pub expansion:  Reflective<Preprocessable<Spanned<String>>>,
}

/// Keys that might reference anything from another C file or the
/// the code generated with this executable and a config.
#[derive(Deserialize, Debug, Clone)]
pub struct Key {
    pub key:  String,
    #[serde(deserialize_with = "reflective_preprocessable_spanned_name_de")]
    pub name: Reflective<Preprocessable<Spanned<Name>>>
}

/// Custom preamble that is inserted as is (first preprocessed tho).
#[derive(Deserialize, Debug, Clone)]
pub struct Preamble {
    #[serde(deserialize_with = "optional_reflective_preprocessable_spanned_string_de")]
    pub raw:  Option<Reflective<Preprocessable<Spanned<String>>>>,
    pub keys: Option<Vec<Key>>,
}

/// Fallbacks the [Generator] uses when encountering strange varadict
/// argument counts.
#[derive(Deserialize, Debug, Clone)]
pub struct Fallbacks {
    #[serde(deserialize_with = "preproc")]
    /// What to do when the varadict argument count is not a multiple
    /// of [Paramaters::Varadict] in [Core::args].
    pub unparity: Reflective<Compilable<Spanned<String>>>,

    #[serde(deserialize_with = "preprocessable_string_deserializer")]
    /// What to do when the varadict argument count is 0?
    pub empty: PreprocessableString,
}

