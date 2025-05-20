use std::{collections::HashMap, sync::{Arc, Mutex, RwLock}};

use colored::Colorize;
use serde::{Deserialize, Serialize};
use strum::EnumProperty;

use crate::{config::{CommonKeyable, Config, Name, StringWithTags}, sigil::Sigil};

#[derive(Debug)]
pub enum ErrorKind {
    InvalidToken,
    IllegalSymbol,
    Serialization,
    PoisonedLock,
    NonexistantReference,
    MutualRefrences,
    DuplicateKey
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String
}

impl std::fmt::Display for Error {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Encountered a error, [{:?}]: {}", self.kind, self.message)
    }

}

impl std::error::Error for Error {}

/// A preprocessable object that can either be a [Preprocessable::NotPreprocessed] 
/// object or a [Preprocessable::Preprocessed] [String].
/// 
/// Whats important is that whatever [Preprocessable::NotPreprocessed] is
/// we can convert it into parts 
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Preprocessable<T>
where T: Preprocess
{
    NotPreprocessed(T),
    Preprocessed(String)
}

impl Default for Preprocessable<Name> {

    fn default() -> Self {
        Self::NotPreprocessed(Name::default())
    }

}

impl Default for Preprocessable<String> {

    fn default() -> Self {
        Self::NotPreprocessed(String::new())
    }

}

pub trait Preprocess {

    fn into_preprocessor_tokens(
        &self,
        keys: &CommonKeyable
    ) -> Result<Vec<PreprocessorToken>, Error>;

}

impl Preprocess for String {

    fn into_preprocessor_tokens(
        &self,
        _: &CommonKeyable
    ) -> Result<Vec<PreprocessorToken>, Error> {
        
        preprocessor_string_tokenizer(self)

    }

} 

impl Preprocess for Name {
    fn into_preprocessor_tokens(
        &self,
        keys: &CommonKeyable
    ) -> Result<Vec<PreprocessorToken>, Error> {

        let s_w_tags = match self {
            Self::Raw(s) => StringWithTags{tags: vec![], string: s.clone()},
            Self::Tagged(swt) => swt.clone()
        };

        let s = &s_w_tags.apply_tags(keys);

        preprocessor_string_tokenizer(s)
        
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PreprocessorToken {
    Raw(String),
    Key(String)
}

pub enum PreprocessorTokenizerState {
    Copying(String),
    CopyingKey(String),
    Skip(Sigil),
    SigilFound,
}

fn preprocessor_string_tokenizer(
    s: &str
) -> Result<Vec<PreprocessorToken>, Error> {

    let mut parts: Vec<PreprocessorToken> = vec![];
    let mut state: PreprocessorTokenizerState 
        = PreprocessorTokenizerState::Copying(String::new());

    for ch in s.chars() {

        match state {

            PreprocessorTokenizerState::Copying(ref mut buffer) => {
                match Sigil::from(ch) {
                    Sigil::TokenStart => {
                        parts.push(PreprocessorToken::Raw(buffer.clone()));
                        state = PreprocessorTokenizerState::SigilFound;
                    }
                    Sigil::PreprocessorKeyRefOpen |
                    Sigil::PreprocessorKeyRefClose |
                    Sigil::CompilerSkipLastOpen |
                    Sigil::CompilerSkipLastClose |
                    Sigil::CompilerArgumentRefOpen |
                    Sigil::CompilerArgumentRefClose => {
                        return Err(Error{
                            kind: ErrorKind::IllegalSymbol,
                            message: format!(
                                "Illegal symbol '{}' in '{}' cannot appear before the '{:?}' symbol '{:?}'",
                                ch, s, Sigil::TokenStart, Sigil::TokenStart.get_str("ch")
                            )
                        })
                    }
                    Sigil::Non(ch) => buffer.push(ch)
                }
            }
            PreprocessorTokenizerState::SigilFound => {
                match Sigil::from(ch) {  
                    Sigil::TokenStart => {
                        return Err(Error{
                            kind: ErrorKind::IllegalSymbol,
                            message: format!(
                                "Duplicate symbol '{}' in '{}' twice or more in a row", ch, s
                            )
                        })
                    }
                    Sigil::PreprocessorKeyRefOpen => {
                        state = PreprocessorTokenizerState::CopyingKey(String::new())
                    }
                    Sigil::CompilerSkipLastOpen => {
                        state = PreprocessorTokenizerState::Skip(Sigil::CompilerSkipLastClose)
                    }
                    Sigil::CompilerArgumentRefOpen => {
                        state = PreprocessorTokenizerState::Skip(Sigil::CompilerArgumentRefClose)
                    }
                    Sigil::PreprocessorKeyRefClose |
                    Sigil::CompilerSkipLastClose |
                    Sigil::CompilerArgumentRefClose |
                    Sigil::Non(_)=> {
                        return Err(Error {
                            kind: ErrorKind::IllegalSymbol,
                            message: format!(
                                "Illegal character '{}' in '{}' after '{:?}' symbol '{:?}' ", 
                                ch, s, Sigil::TokenStart, Sigil::TokenStart.get_str("ch")
                            )
                        })
                    }
                }
            }
            PreprocessorTokenizerState::Skip(expected_sigil) => {
                if expected_sigil == Sigil::from(ch) {
                    state = PreprocessorTokenizerState::Copying(String::new())
                }
                match Sigil::from(ch) {
                    Sigil::Non(_) => (),
                    _ => {
                        return Err(Error {
                            kind: ErrorKind::IllegalSymbol,
                            message: format!(
                                "Illegal character '{}' in '{}', expected a '{:?}' symbol '{:?}'", 
                                ch, s, expected_sigil, expected_sigil.get_str("ch")
                            )
                        })
                    }
                }   
            }
            PreprocessorTokenizerState::CopyingKey(ref mut buffer_key) => {
                match Sigil::from(ch) {
                    Sigil::PreprocessorKeyRefClose => {
                        parts.push(PreprocessorToken::Key(buffer_key.clone()));
                        state = PreprocessorTokenizerState::Copying(String::new());
                    }
                    Sigil::Non(ch) => buffer_key.push(ch),
                    _ => {
                        return Err(Error {
                            kind: ErrorKind::IllegalSymbol,
                            message: format!(
                                "Illegal character '{}' in '{}', expected a '{:?}' symbol '{:?}'", 
                                ch, s, Sigil::PreprocessorKeyRefClose, Sigil::PreprocessorKeyRefClose.get_str("ch")
                            )
                        })
                    }
                }
            }
        }
    }

    match state {
        PreprocessorTokenizerState::Copying(buffer) => {
            parts.push(PreprocessorToken::Raw(buffer))
        }
        PreprocessorTokenizerState::SigilFound => {
            return Err(Error {
                kind: ErrorKind::InvalidToken,
                message: format!(
                    "'{:?}' symbol '{:?}' found with no body to go along side it in '{}'", 
                    Sigil::TokenStart, Sigil::TokenStart.get_str("ch"), s
                )
            })
        }
        PreprocessorTokenizerState::Skip(expected_sigil) => {
            return Err(Error {
                kind: ErrorKind::InvalidToken,
                message: format!(
                    "Unfinished token in '{}', that was supposed to end with a '{:?}' symbol '{:?}'", 
                    s, expected_sigil.clone(), expected_sigil.get_str("ch")
                )
            })
        }
        PreprocessorTokenizerState::CopyingKey(_) => {
            return Err(Error {
                kind: ErrorKind::InvalidToken,
                message: format!(
                    "Unfinished `key reference` token in preprocessable '{}'", s)
            })
        }
    }

    Ok(parts)

}

/// Simple wrapper to hold all Preprocessable types.
#[derive(Debug, Clone)]
pub enum AnyPreprocessable {
    Name(Arc<RwLock<Preprocessable<Name>>>),
    String(Arc<RwLock<Preprocessable<String>>>)
}

pub fn preprocessor_token_assembly_attempt(
    tokens: Vec<PreprocessorToken>,
    keys: &HashMap<String, AnyPreprocessable>
) -> Result<Option<String>, Error> {

    let mut assembled_string = String::new();

    for token in tokens.iter() {

        match token {
            PreprocessorToken::Raw(s) => {
                assembled_string.push_str(&s);
            }
            PreprocessorToken::Key(key) => {
                let Some(preprocessable) = keys.get(key) else {
                    return Err(Error { 
                        kind: ErrorKind::NonexistantReference, 
                        message: format!(
                            "string was seperated into tokens \n{:?}\nbut token {:?} cannot be preprocessed",
                            tokens, token
                        )
                    })
                };
                match preprocessable {
                    AnyPreprocessable::Name(preprocessable_name) => {
                        let unguarded_preprocessable_name = preprocessable_name.read()
                            .map_err(|err| Error {
                                kind: ErrorKind::PoisonedLock,
                                message: err.to_string() 
                            })?;

                        match &*unguarded_preprocessable_name {
                            Preprocessable::NotPreprocessed(_) => {
                                return Ok(None)
                            },
                            Preprocessable::Preprocessed(name) => {
                                assembled_string.push_str(name);
                            }
                        }
                    }
                    AnyPreprocessable::String(preprocessable_string) => {
                        let unguarded_preprocessable_string = preprocessable_string.read()
                            .map_err(|err| Error {
                                kind: ErrorKind::PoisonedLock,
                                message: err.to_string() 
                            })?;

                        match &*unguarded_preprocessable_string {
                            Preprocessable::NotPreprocessed(_) => {
                                return Ok(None)
                            },
                            Preprocessable::Preprocessed(string) => {
                                assembled_string.push_str(string);
                            }
                        }
                    } 
                }
            }
        }

    }

    Ok(Some(assembled_string))

}

pub fn preprocess_key_name_pairs(
    keys: &HashMap<String, AnyPreprocessable>,
    common_keys: &CommonKeyable
) -> Result<(), Error> {

    let mut left = keys.len();
    
    while left != 0 {

        let now_left: Mutex<usize> = Mutex::new(0);
        for (key, preprocessable) in keys.iter() {

            let tokens = match preprocessable {
                AnyPreprocessable::Name(name) => {
                    let name_kind = name.read()
                        .map_err(|err| Error {
                            kind: ErrorKind::PoisonedLock,
                            message: err.to_string() 
                        })?;
                    match &*name_kind {
                        Preprocessable::NotPreprocessed(name) => {
                            name.into_preprocessor_tokens(common_keys)?
                        }
                        Preprocessable::Preprocessed(name) => {
                            log::trace!("{}", 
                                format!("Key `{key}` with name `{:?}` is already preprocessed.", name)
                                .dimmed().strikethrough()
                            );
                            continue
                        }
                    }
                }
                AnyPreprocessable::String(preprocessable_s) => {
                    let s_kind = preprocessable_s.read()
                        .map_err(|err| Error {
                            kind: ErrorKind::PoisonedLock,
                            message: err.to_string() 
                        })?;
                    match &*s_kind {
                        Preprocessable::NotPreprocessed(s) => {
                            s.into_preprocessor_tokens(common_keys)?
                        }
                        Preprocessable::Preprocessed(s) => {
                            log::trace!("{}", 
                                format!("Key `{key}` with name `{:?}` is already preprocessed.", s)
                                .dimmed().strikethrough()
                            );
                            continue
                        }
                    }
                }
            };

            log::trace!("{}", 
                format!("Attempting to preprocess key `{key}` with name `{:?}`.", preprocessable)
                .dimmed()
            );

            let Some(preprocessed_string) = preprocessor_token_assembly_attempt(
                tokens,
                &keys
            )? else {
                log::trace!("{}",
                    format!("Key was not preprocessed successfully as it has dependencies that are not preprocessed themselves.")
                    .truecolor(255, 165, 0).dimmed()
                );
                let mut guard = now_left.lock().unwrap();
                *guard += 1;
                continue;
            };

            log::trace!("{}",
                format!("Key was preprocessed successfully -> key `{key}` with name `{:?}`.", preprocessed_string)
                .cyan().dimmed()
            );

            match preprocessable {
                AnyPreprocessable::Name(preprocessable) => {
                    let mut write_guard = preprocessable.write()
                        .map_err(|err| Error {
                            kind: ErrorKind::PoisonedLock,
                            message: err.to_string() 
                        })?;
                    *write_guard = Preprocessable::Preprocessed(preprocessed_string);
                }
                AnyPreprocessable::String(preprocessable) => {
                    let mut write_guard = preprocessable.write()
                        .map_err(|err| Error {
                            kind: ErrorKind::PoisonedLock,
                            message: err.to_string() 
                        })?;
                    *write_guard = Preprocessable::Preprocessed(preprocessed_string);
                }
            }           
        }

        let guard_left= now_left.lock().unwrap();

        if &*guard_left >= &left {
            let key_names: Vec<(String, Name)> = keys
                .clone()
                .into_iter()
                .filter_map(|(k, v)| {
                    match v {
                        AnyPreprocessable::Name(preprocessable_name) => {
                            let read_guard = match preprocessable_name.read() {
                                Ok(guard) => guard,
                                Err(_) => return None
                            };
                            match &*read_guard {
                                Preprocessable::NotPreprocessed(not_preprocessed) => {
                                    Some((k, not_preprocessed.clone()))
                                }
                                Preprocessable::Preprocessed(_) => None
                            }
                        }
                        AnyPreprocessable::String(_) => None
                        // Preprocessable::NotPreprocessed(uncompiled_name) => Some((k, uncompiled_name)),
                        // Preprocessable::Preprocessed(_) => None
                    }
                })
                .collect();
            return Err(Error {
                kind: ErrorKind::MutualRefrences,
                message: format!(
                    "the following keys could not be preprocessed, they probably have mutual references or reference themselves: \n{:#?}",
                    key_names
                )
            })
        }

        left = *guard_left;
        log::trace!("{left} keys left to preprocess")

    }

    Ok(())
}
    
impl Config {

    /// Loads all key name pairs like all the [config::Preamble] -> Vec<[config::Key]>
    /// and Vec<[config::Definition]> key name pairs.
    /// 
    /// Important
    /// ---------
    /// This also loads key name pairs from [config::CommonKeyable] where the `key`
    /// is the variable name and the `name` is the value inside the variable.
    /// 
    /// This is important to the preprocessing process since we only read keys
    /// from the hash map that is returned from this function.
    /// 
    /// Also worthy of noting, the value of the [HashMap] is [AnyPreprocessable]
    /// which holds a [Arc]<[RwLock]<>> of the name data, meaning that any change
    /// done within the [RwLock] is reflected on the config itself.
    pub fn load_key_name_pairs(&self) -> Result<HashMap<String, AnyPreprocessable>, Error> {
        let mut keys: HashMap<String, AnyPreprocessable> = HashMap::new();

        // Vrijednosti iz CommonKeyable mogu se pojaviti kao ključevi unutar
        // imena.
        let common_keys: Vec<(String, serde_json::Value)>  = serde_json::to_value(&self.common.keyable)
            .map_err(|_| Error {
                kind: ErrorKind::Serialization,
                message: "Failed to serialize keyable common values.".to_owned() 
            })?
            .as_object()
            .ok_or_else(|| Error {
                kind: ErrorKind::Serialization,
                message: "Failed to create object from serialized keyable common values.".to_owned() 
            })?
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();

        for (k, v) in common_keys {
            if let serde_json::Value::String(s) = v {
                if keys.contains_key(&k) {
                    return Err(Error{
                        kind: ErrorKind::DuplicateKey,
                        message: format!(
                            "Common key {k} must be unique, but multiple keys with the same name were found."
                        )
                    })
                }
                // Common varijable su uvijek čiste od kljuceva unutar sebe
                // te ih mozemo odma staviti kao preprocesirane.
                keys.insert(k, 
                    AnyPreprocessable::String(
                        Arc::new(RwLock::new(Preprocessable::Preprocessed(s)))
                    )
                );
            }
        }

        if let Some(preamble) = self.preamble.as_ref() {
            if let Some(preamble_keys) = preamble.keys.as_ref() {
                for key in preamble_keys {
                    if keys.contains_key(&key.key) {
                        return Err(Error{
                            kind: ErrorKind::DuplicateKey,
                            message: format!(
                                "Key {} must be unique, but multiple keys with the same name were found.",
                                key.key
                            )
                        })
                    }
                    keys.insert(key.key.clone(), AnyPreprocessable::Name(key.name.clone()));
                }
            }
        }

        if let Some(definitions) = self.definition.as_ref() {
            for definition in definitions {
                if keys.contains_key(&definition.key) {
                    return Err(Error{
                        kind: ErrorKind::DuplicateKey,
                        message: format!(
                            "Key {} must be unique, but multiple keys with the same name were found.",
                        definition.key
                        )
                    })
                }
                keys.insert(definition.key.clone(),AnyPreprocessable::Name(definition.name.clone()));
            }
        }



        Ok(keys)

    }
    

    pub fn preprocess(self) -> Result<Self, Error> {

        log::debug!("Starting to preprocess the config.");

        log::debug!("Loading key name pairs...");
        let keys = self.load_key_name_pairs()?;
        log::debug!("Loaded key name pairs!");
        
        log::debug!("Preprocessing key name pairs...");
        preprocess_key_name_pairs(&keys, &self.common.keyable)?;
        log::debug!("Preprocessed key name pairs!");

        println!("{:#?}", keys);

        unimplemented!()

    }

}