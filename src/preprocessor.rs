use std::{collections::HashMap, mem::discriminant, sync::{Arc, Mutex, RwLock}};

use colored::Colorize;
use serde::{Deserialize, Serialize};
use strum::EnumProperty;

use crate::{
    config::{
        Argument, CommonKeyable, Config, Name, StringWithTags
    }, 
    sigil::PreprocessorSigil
};

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidToken,
    IllegalSymbol,
    Serialization,
    PoisonedLock,
    NonExistantReference,
    MutualReferences,
    EmptyReference,
    DuplicateKey
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub(crate) message: String
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
/// we can convert it into [PreprocessorToken]s since every type has to implement
/// [Preprocess].
/// 
/// When loading the config, all preprocessable types start as 
/// [Preprocessable::NotPreprocessed] except the values from [CommonKeyable].
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

/// [Preprocess] is implemented on every type that can be preprocessed
/// and that are a part of [Preprocessable]. 
pub trait Preprocess {

    /// Convert the type into a [Vec]<[PreprocessorToken]>,
    /// used since we have key references in preprocessable types
    /// that have to be first detected as a token (here) and then
    /// read from the table of keys.
    ///
    /// Init
    /// ----
    /// Types from [Preprocessable] are not initialized (like [StringWithTags])
    /// and here they are meant to be initialized before they are processed
    /// into tokens.
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
        
        // init
        let s = &s_w_tags.apply_tags(keys);

        preprocessor_string_tokenizer(s)
        
    }

}

/// Preprocessor tokens that will be processed and combined together intož
/// a finished preprocessed string.
/// `Raw` hold a raw string that has no special characteristics.
/// `Key` holds a string that a name of a key. 
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum PreprocessorToken {
    Raw(String),
    Key(String)
}

#[derive(Debug, Clone)]
pub enum PreprocessorTokenizerState {
    Copying(String),
    CopyingKey(String),
    SigilFound,
    EmbedFound(String)
}

/// Regular [PreprocessorToken] tokenizer, meant to be run on all [Preprocessable]s.
/// This also includes the [crate::config::Generator::repeat] [Preprocessable]
/// but it skips special sigils like [Sigil::CompilerSkipLastOpen]/[Sigil::CompilerSkipLastClose]
/// and [Sigil::CompilerArgumentRefOpen]/[Sigil::CompilerArgumentRefClose].
fn preprocessor_string_tokenizer(
    s: &str
) -> Result<Vec<PreprocessorToken>, Error> {

    let mut parts: Vec<PreprocessorToken> = vec![];
    let mut state: PreprocessorTokenizerState 
        = PreprocessorTokenizerState::Copying(String::new());
    let mut prev_state = state.clone();

    for ch in s.chars() {

        if discriminant(&prev_state) != discriminant(&state) {
            log::trace!(
                "{}: {}",
                format!("[preprocessor_string_tokenizer]").bold(),
                format!("Curr state {:?}", prev_state).dimmed()
            );
        }
        prev_state = state.clone();

        match state {

            PreprocessorTokenizerState::Copying(ref mut buffer) => {
                match PreprocessorSigil::from(ch) {
                    PreprocessorSigil::TokenStart => {
                        if !buffer.is_empty() {
                            parts.push(PreprocessorToken::Raw(buffer.clone()));
                        }
                        state = PreprocessorTokenizerState::SigilFound;
                    }
                    PreprocessorSigil::TokenEmbed => {
                        state = PreprocessorTokenizerState::EmbedFound(buffer.clone());
                    }
                    PreprocessorSigil::KeyRefOpen |
                    PreprocessorSigil::KeyRefClose |
                    PreprocessorSigil::Non(_) => buffer.push(ch)
                }
            }
            PreprocessorTokenizerState::EmbedFound(ref mut buffer) => {
                match PreprocessorSigil::from(ch) {
                    PreprocessorSigil::TokenStart |
                    PreprocessorSigil::TokenEmbed => {
                        buffer.push(ch);
                    }
                    _ => {
                        return Err(Error{
                            kind: ErrorKind::IllegalSymbol,
                            message: format!(
                                "Expected a {:?} symbol {:?} or {:?} symbol {:?} after '{ch}'",
                                    PreprocessorSigil::TokenStart,
                                    PreprocessorSigil::TokenStart.get_str("ch"),
                                    PreprocessorSigil::TokenEmbed,
                                    PreprocessorSigil::TokenEmbed.get_str("ch")
                            )
                        })
                    }
                }
                state = PreprocessorTokenizerState::Copying(buffer.clone());
            }
            PreprocessorTokenizerState::SigilFound => {
                match PreprocessorSigil::from(ch) {  
                    PreprocessorSigil::TokenStart => {
                        return Err(Error{
                            kind: ErrorKind::IllegalSymbol,
                            message: format!(
                                "Duplicate symbol '{}' in '{}' twice or more in a row", ch, s
                            )
                        })
                    }
                    PreprocessorSigil::KeyRefOpen => {
                        state = PreprocessorTokenizerState::CopyingKey(String::new())
                    }
                    PreprocessorSigil::KeyRefClose |
                    PreprocessorSigil::TokenEmbed |
                    PreprocessorSigil::Non(_)=> {
                        return Err(Error {
                            kind: ErrorKind::IllegalSymbol,
                            message: format!(
                                "Illegal character '{}' in '{}' after '{:?}' symbol '{:?}' ", 
                                ch, s, PreprocessorSigil::TokenStart, PreprocessorSigil::TokenStart.get_str("ch")
                            )
                        })
                    }
                }
            }
            PreprocessorTokenizerState::CopyingKey(ref mut buffer_key) => {
                match PreprocessorSigil::from(ch) {
                    PreprocessorSigil::KeyRefClose => {
                        if buffer_key.is_empty() {
                            return Err(Error {
                                kind: ErrorKind::EmptyReference,
                                message: format!(
                                    "Empty key reference `{}{}{}` inside of a preprocessable name `{s}`",
                                    PreprocessorSigil::TokenStart.get_str("ch").unwrap(),
                                    PreprocessorSigil::KeyRefOpen.get_str("ch").unwrap(),
                                    PreprocessorSigil::KeyRefClose.get_str("ch").unwrap(),
                                )
                            })
                        }
                        parts.push(PreprocessorToken::Key(buffer_key.clone()));
                        state = PreprocessorTokenizerState::Copying(String::new());
                    }
                    PreprocessorSigil::Non(ch) => buffer_key.push(ch),
                    _ => {
                        return Err(Error {
                            kind: ErrorKind::IllegalSymbol,
                            message: format!(
                                "Illegal character '{}' in '{}', expected a '{:?}' symbol '{:?}'", 
                                ch, s, PreprocessorSigil::KeyRefClose, PreprocessorSigil::KeyRefClose.get_str("ch")
                            )
                        })
                    }
                }
            }
        }
    }

    log::trace!(
        "{}: {}",
        format!("[preprocessor_string_tokenizer]").bold(),
        format!("Last state {:?}", state).dimmed()
    );

    match state {
        PreprocessorTokenizerState::Copying(buffer) => {
            if !buffer.is_empty() {
                parts.push(PreprocessorToken::Raw(buffer))
            }
        }
        PreprocessorTokenizerState::EmbedFound(_) => {
            return Err(Error{
                kind: ErrorKind::IllegalSymbol,
                message: format!(
                    "Expected a {:?} symbol {:?} or {:?} symbol {:?} after '{:?}'",
                        PreprocessorSigil::TokenStart,
                        PreprocessorSigil::TokenStart.get_str("ch"),
                        PreprocessorSigil::TokenEmbed,
                        PreprocessorSigil::TokenEmbed.get_str("ch"),
                        PreprocessorSigil::TokenStart.get_str("ch")
                )
            })
        }
        PreprocessorTokenizerState::SigilFound => {
            return Err(Error {
                kind: ErrorKind::InvalidToken,
                message: format!(
                    "'{:?}' symbol '{:?}' found with no body to go along side it in '{}'", 
                    PreprocessorSigil::TokenStart, PreprocessorSigil::TokenStart.get_str("ch"), s
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

/// Wrapper for [Preprocessable]<[Name]>, see [AnyPreprocessable] and
/// [Preprocessable] for more info. 
pub type PreprocessableName   = Arc<RwLock<Preprocessable<Name>>>;

/// Wrapper for [Preprocessable]<[String]>, see [AnyPreprocessable] and
/// [Preprocessable] for more info. 
pub type PreprocessableString = Arc<RwLock<Preprocessable<String>>>;

/// Simple wrapper to hold all [Preprocessable] types.
/// All variants are first wrapped with a [Arc] [RwLock].
/// 
/// Why [Arc] & [RwLock]?
/// ---------------------
/// Very simply put whenever we attempt to preprocess them instead of trying 
/// to find the variable that the [AnyPreprocessable] came from, we instead
/// write directly into the variable and the change will be reflected inside
/// of the [Config] itself. 
#[derive(Debug, Clone)]
pub enum AnyPreprocessable {
    Name(PreprocessableName),
    String(PreprocessableString)
}

/// Attempt to assemble a [Vec] of [PreprocessorToken].
/// `keys` are a set of key name pairs from the [Config] and they are used for
/// processing [PreprocessorToken::Key] tokens.
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
                        kind: ErrorKind::NonExistantReference, 
                        message: format!(
                            "string was seperated into tokens: {:?}... but the token {:?} contains a key that doesn't exist",
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

/// Preprocess key name pairs from `keys` and finialize them.
/// 
/// Since the unpreprocessed key name pairs are stored in a [AnyPreprocessable] 
/// they can be written to and the changes will be reflected in the [Config] they 
/// came from.
/// 
/// Which is also the reason we return a `Ok(())` meaning we successfully
/// preprocessed all the key name pairs from `keys` and written the results 
/// back into the [AnyPreprocessable].
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
                    }
                })
                .collect();
            return Err(Error {
                kind: ErrorKind::MutualReferences,
                message: format!(
                    "the following keys could not be preprocessed, they probably have mutual references or reference themselves: \n{:#?}",
                    key_names
                )
            })
        }

        left = *guard_left;
        log::trace!("{}", format!("{left} keys left to preprocess.").dimmed())

    }

    Ok(())
}

pub fn preprocess_strings(
    preprocessable_strings: Vec<PreprocessableString>,
    keys: &HashMap<String, AnyPreprocessable>,
    common_keys: &CommonKeyable
) -> Result<(), Error> {
    
    for ps in preprocessable_strings {

        let ps_read = ps.read() 
            .map_err(|err| Error {
                kind: ErrorKind::PoisonedLock,
                message: err.to_string() 
            })?;

        let tokens = match &*ps_read {
            Preprocessable::NotPreprocessed(s) => {
                s.into_preprocessor_tokens(common_keys)?
            }
            Preprocessable::Preprocessed(_) => continue
        };

        let preprocessed = match preprocessor_token_assembly_attempt(tokens, keys) {
            Ok(Some(s)) => s,
            Ok(None) => unreachable!(),
            Err(err) => return Err(err)
        };

        log::trace!("{}",
            format!("Preprocessed string from '{:?}' -> '{}'", ps_read, preprocessed)
            .dimmed()
        );

        drop(ps_read);

        let mut ps_write = ps.write() 
            .map_err(|err| Error {
                kind: ErrorKind::PoisonedLock,
                message: err.to_string() 
            })?;
        
        *ps_write = Preprocessable::Preprocessed(preprocessed);

    }

    Ok(())
}

    
impl Config {

    /// Loads all preprocessable strings from the config that are not
    /// key name pairs.
    fn load_preprocessable_strings(&self) -> Vec<PreprocessableString> {

        let mut preprocessables: Vec<PreprocessableString> = vec![];

        if let Some(preamble) = &self.preamble {
            if let Some(raw) = &preamble.raw {
                preprocessables.push(raw.clone());
            }
        }

        if let Some(definitions) = &self.definition {
            for def in definitions {
                preprocessables.push(def.expansion.clone());
            }
        }

        preprocessables.push(self.core.xmva.clone());

        for arg in self.core.args.iter() {
            match arg {
                &Argument::Named(ref named) => {
                    preprocessables.push(named.name.clone())
                }
                &Argument::Varadict { varadict: _ } => ()
            }
        }

        for generator in &self.generator {
            preprocessables.push(generator.preamble.clone());
            preprocessables.push(generator.repeat.clone());
            preprocessables.push(generator.postamble.clone());
            preprocessables.push(generator.fallbacks.unparity.clone());
            preprocessables.push(generator.fallbacks.empty.clone());
        }

        preprocessables

    }

    /// Loads all key name pairs like all the [crate::config::Preamble] -> Vec<[crate::config::Key]>
    /// and Vec<[crate::config::Definition]> key name pairs.
    /// 
    /// Important
    /// ---------
    /// This also loads key name pairs from [crate::config::CommonKeyable] where the `key`
    /// is the variable name and the `name` is the value inside the variable.
    /// 
    /// This is important to the preprocessing process since we only read keys
    /// from the hash map that is returned from this function when trying to process
    /// key references inside of a [Preprocessable].
    /// 
    /// Also worthy of noting, the value of the [HashMap] is [AnyPreprocessable]
    /// which holds a [Arc]<[RwLock]<>> of the name data, meaning that any change
    /// done within the [RwLock] is reflected on the config itself.
    fn load_preprocessable_key_name_pairs(&self) -> Result<HashMap<String, AnyPreprocessable>, Error> {
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
    

    pub fn preprocess(&self) -> Result<(), Error> {

        log::debug!("Starting to preprocess the config.");

        log::debug!("Loading key name pairs...");
        let keys = self.load_preprocessable_key_name_pairs()?;
        log::trace!("{}",
            format!("Loaded keys: {:#?}", keys).dimmed()
        );
        
        log::debug!("Preprocessing key name pairs...");
        preprocess_key_name_pairs(&keys, &self.common.keyable)?;

        log::debug!("Loading all preprocessable strings...");
        let preprocessable_strings = self.load_preprocessable_strings();
        log::trace!("{}",
            format!("Loaded preprocessable strings: {:#?}", preprocessable_strings).dimmed()
        );

        log::debug!("Preprocessing strings...");
        preprocess_strings(preprocessable_strings, &keys, &self.common.keyable)?;

        return Ok(())

    }

}

mod tests {
    
    #[allow(unused_imports)]
    use super::*;

    /// The tokneizer is both the only thing that interacts with user strings
    /// and the most complex part of the preprocessor.
    /// Everything else is pretty simple and relies on enums to guide
    /// the code.

    #[test]
    fn tokenizer_simple() {

        // Simple
        assert_eq!(
            preprocessor_string_tokenizer(
                "hello world@{prefix}"
            ).unwrap(),
            vec![
                PreprocessorToken::Raw("hello world".to_owned()),
                PreprocessorToken::Key("prefix".to_owned())
            ]
        );

    }

    #[test]
    fn tokenizer_complex() {

        // Complex
        assert_eq!(
            preprocessor_string_tokenizer(
                "@{#$%\"\"!23O1''???ŠSĆDsl😍💕😳****}\\@{destroyer}\\\\@{beyonce}#$%\"\"!23O1''???ŠSĆDsl😍💕😳****@{prefix}@{dufus}\\\\"
            ).unwrap(),
            vec![
                PreprocessorToken::Key("#$%\"\"!23O1''???ŠSĆDsl😍💕😳****".to_owned()),
                PreprocessorToken::Raw("@{destroyer}\\".to_owned()),
                PreprocessorToken::Key("beyonce".to_owned()),
                PreprocessorToken::Raw("#$%\"\"!23O1''???ŠSĆDsl😍💕😳****".to_owned()),
                PreprocessorToken::Key("prefix".to_owned()),
                PreprocessorToken::Key("dufus".to_owned()),
                PreprocessorToken::Raw("\\".to_owned()),
            ]
        );
    
    }

    // Error cases:
    #[test]
    fn tokenizer_check_embed() {

        // Check if // is properly handled across various scenarios
        assert_eq!(
            preprocessor_string_tokenizer(
                // Handle embeding, and not embeding both self, a random character
                // and another token and check if at the edge case (lol) is
                // handled
                "\\@ \\\\\\\\"
            ).unwrap(),
            vec![
                PreprocessorToken::Raw("@ \\\\".to_owned())
            ]
        );

        assert!(
            preprocessor_string_tokenizer(
                "\\"
            ).is_err()
        );

        assert!(
            preprocessor_string_tokenizer(
                "\\$"
            ).is_err()
        );

        assert!(
            preprocessor_string_tokenizer(
                "\\\\n"
            ).is_ok()
        );

    }

    #[test]
    fn tokenizer_check_no_empty_raws() {

        assert_eq!(
            preprocessor_string_tokenizer(
                // Check that we dont create random empty raws between these
                // PreprocessorToken::Key.
                "@{hello}@{hi}@{byebye}"
            ).unwrap(),
            vec![
                PreprocessorToken::Key("hello".to_owned()),
                PreprocessorToken::Key("hi".to_owned()),
                PreprocessorToken::Key("byebye".to_owned()),
            ]
        );

    }

    #[test]
    fn tokenizer_check_no_empty_reference() {

        assert_eq!(
            preprocessor_string_tokenizer(
                // Check that we throw a error on a empty reference.
                "@{}"
            ).unwrap_err().kind,
            ErrorKind::EmptyReference
        );

    }

    #[test]
    fn tokenizer_check_illegal_symbol_in_reference() {

        assert_eq!(
            preprocessor_string_tokenizer(
                // Check that cant have sigils inside of a reference.
                "@{@}"
            ).unwrap_err().kind,
            ErrorKind::IllegalSymbol
        );

        assert_eq!(
            preprocessor_string_tokenizer(
                "@{{}"
            ).unwrap_err().kind,
            ErrorKind::IllegalSymbol
        );

        assert!(
            preprocessor_string_tokenizer(
                "@{a}}"
            ).is_ok() 
        );

    }


}