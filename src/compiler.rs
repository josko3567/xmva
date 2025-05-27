use std::{collections::HashMap, mem::discriminant, usize};

use colored::Colorize;
use strum::{EnumIter, EnumProperty};

use crate::{
    config::{Argument, Config}, 
    preprocessor::{Preprocessable, PreprocessableString},
    sigil::CompilerSigil
};

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorKind {
    DuplicateArgument,
    IllegalSymbol,
    EmptyReference,
    InvalidReference,
    InvalidToken,
    PoisonedLock,
    NotPreprocessed,
    NonExistantArgument
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    message: String
}

impl std::fmt::Display for Error {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Compiler encountered a error, [{:?}]: {}", self.kind, self.message)
    }

}

impl std::error::Error for Error {}

#[derive(Debug, PartialEq, Eq, EnumProperty, EnumIter)]
enum CompilerToken {
    #[strum(props(surface = true))]
    Raw(String),
    #[strum(props(surface = true))]
    NamedArgumentRef(String),
    UnamedArgumentRef(usize),
    Position,
    SkipLast(String)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerTokenizerState {
    Copying(String),
    CopyingNamedArgumentRef(String),
    CopyingUnamedArgumentRef(String),
    CopyingSkipLast(String),
    CopyingSkipLastEmbed(String),
    SigilFound,
    EmbedFound(String)
}

impl CompilerToken {

    fn tokenize(
        s: &str
    ) -> Result<Vec<CompilerToken>, Error> {

        let mut parts: Vec<CompilerToken> = vec![];
        let mut state: CompilerTokenizerState 
            = CompilerTokenizerState::Copying(String::new());
        let mut prev_state = state.clone();

        for ch in s.chars() {

            if discriminant(&prev_state) != discriminant(&state) {
                log::trace!(
                    "{}: {}",
                    format!("[CompilerToken::tokenize]").bold(),
                    format!("Curr state {:?}", prev_state).dimmed()
                );
            }
            prev_state = state.clone();

            match state {

                CompilerTokenizerState::Copying(ref mut buffer) => {
                    match CompilerSigil::from(ch) {
                        CompilerSigil::TokenStart => {
                            if !buffer.is_empty() {
                                parts.push(CompilerToken::Raw(buffer.clone()));
                            }
                            state = CompilerTokenizerState::SigilFound;
                        }
                        CompilerSigil::TokenEmbed => {
                            state = CompilerTokenizerState::EmbedFound(buffer.clone());
                        }
                        CompilerSigil::NamedArgumentRefOpen |
                        CompilerSigil::NamedArgumentRefClose |
                        CompilerSigil::UnamedArgumentRefOpen |
                        CompilerSigil::UnamedArgumentRefClose |
                        CompilerSigil::SkipLastOpen |
                        CompilerSigil::SkipLastClose |
                        CompilerSigil::PositionDot |
                        CompilerSigil::Non(_) => buffer.push(ch)
                    }
                }
                CompilerTokenizerState::EmbedFound(ref mut buffer) => {
                    match CompilerSigil::from(ch) {
                        CompilerSigil::TokenStart |
                        CompilerSigil::TokenEmbed => {
                            buffer.push(ch);
                        }
                        _ => {
                            return Err(Error{
                                kind: ErrorKind::IllegalSymbol,
                                message: format!(
                                    "Expected a {:?} symbol {:?} or {:?} symbol {:?} after '{ch}'",
                                        CompilerSigil::TokenStart,
                                        CompilerSigil::TokenStart.get_str("ch"),
                                        CompilerSigil::TokenEmbed,
                                        CompilerSigil::TokenEmbed.get_str("ch")
                                )
                            })
                        }
                    }
                    state = CompilerTokenizerState::Copying(buffer.clone());
                }
                CompilerTokenizerState::SigilFound => {
                    match CompilerSigil::from(ch) {  
                        CompilerSigil::TokenStart => {
                            return Err(Error{
                                kind: ErrorKind::IllegalSymbol,
                                message: format!(
                                    "Duplicate symbol '{}' in '{}' twice or more in a row", ch, s
                                )
                            })
                        }
                        CompilerSigil::PositionDot => {
                            parts.push(CompilerToken::Position);
                            state = CompilerTokenizerState::Copying(String::new())
                        }
                        CompilerSigil::NamedArgumentRefOpen => {
                            state = CompilerTokenizerState::CopyingNamedArgumentRef(String::new())
                        }
                        CompilerSigil::UnamedArgumentRefOpen => {
                            state = CompilerTokenizerState::CopyingUnamedArgumentRef(String::new())
                        }
                        CompilerSigil::SkipLastOpen => {
                            state = CompilerTokenizerState::CopyingSkipLast(String::new())
                        }
                        CompilerSigil::NamedArgumentRefClose |
                        CompilerSigil::UnamedArgumentRefClose |
                        CompilerSigil::SkipLastClose | 
                        CompilerSigil::TokenEmbed |
                        CompilerSigil::Non(_)=> {
                            return Err(Error {
                                kind: ErrorKind::IllegalSymbol,
                                message: format!(
                                    "Illegal character '{}' in '{}' after '{:?}' symbol '{:?}' ", 
                                    ch, s, CompilerSigil::TokenStart, CompilerSigil::TokenStart.get_str("ch")
                                )
                            })
                        }
                    }
                }
                CompilerTokenizerState::CopyingNamedArgumentRef(ref mut buffer_key) => {
                    match CompilerSigil::from(ch) {
                        CompilerSigil::NamedArgumentRefClose => {
                            if buffer_key.is_empty() {
                                return Err(Error {
                                    kind: ErrorKind::EmptyReference,
                                    message: format!(
                                        "Empty named argument reference `{}{}{}` inside of a compilable name `{s}`",
                                        CompilerSigil::TokenStart.get_str("ch").unwrap(),
                                        CompilerSigil::NamedArgumentRefOpen.get_str("ch").unwrap(),
                                        CompilerSigil::NamedArgumentRefClose.get_str("ch").unwrap(),
                                    )
                                })
                            }
                            parts.push(CompilerToken::NamedArgumentRef(buffer_key.clone()));
                            state = CompilerTokenizerState::Copying(String::new());
                        }
                        CompilerSigil::PositionDot |
                        CompilerSigil::Non(_) => buffer_key.push(ch),
                        _ => {
                            return Err(Error {
                                kind: ErrorKind::IllegalSymbol,
                                message: format!(
                                    "Illegal character '{}' in '{}', expected a '{:?}' symbol '{:?}'", 
                                    ch, s, CompilerSigil::NamedArgumentRefClose, CompilerSigil::NamedArgumentRefClose.get_str("ch")
                                )
                            })
                        }
                    }
                }
                CompilerTokenizerState::CopyingUnamedArgumentRef(ref mut buffer_key) => {
                    match CompilerSigil::from(ch) {
                        CompilerSigil::UnamedArgumentRefClose => {
                            if buffer_key.is_empty() {
                                return Err(Error {
                                    kind: ErrorKind::EmptyReference,
                                    message: format!(
                                        "Empty unamed argument reference `{}{}{}` inside of a compilable name `{s}`",
                                        CompilerSigil::TokenStart.get_str("ch").unwrap(),
                                        CompilerSigil::UnamedArgumentRefOpen.get_str("ch").unwrap(),
                                        CompilerSigil::UnamedArgumentRefClose.get_str("ch").unwrap(),
                                    )
                                })
                            }
                            let Ok(value) = buffer_key.clone().parse::<usize>() else {
                                return Err(Error {
                                    kind: ErrorKind::InvalidReference,
                                    message: format!(
                                        "Couldn't convert `{}` into a number for token {:?}", 
                                        buffer_key, CompilerToken::UnamedArgumentRef(0)
                                    )
                                })
                            };
                            parts.push(CompilerToken::UnamedArgumentRef(value));
                            state = CompilerTokenizerState::Copying(String::new());
                        }
                        CompilerSigil::PositionDot |
                        CompilerSigil::Non(_) => buffer_key.push(ch),
                        _ => {
                            return Err(Error {
                                kind: ErrorKind::IllegalSymbol,
                                message: format!(
                                    "Illegal character '{}' in '{}', expected a '{:?}' symbol '{:?}'", 
                                    ch, s, CompilerSigil::UnamedArgumentRefClose, CompilerSigil::UnamedArgumentRefClose.get_str("ch")
                                )
                            })
                        }
                    }
                }
                CompilerTokenizerState::CopyingSkipLast(ref mut buffer_key) => {
                    // log::trace!("sl: {ch}");
                    match CompilerSigil::from(ch) {
                        CompilerSigil::SkipLastClose => {
                            if buffer_key.is_empty() {
                                return Err(Error {
                                    kind: ErrorKind::EmptyReference,
                                    message: format!(
                                        "Empty skip last token `{}{}{}` inside of a compilable name `{s}`",
                                        CompilerSigil::TokenStart.get_str("ch").unwrap(),
                                        CompilerSigil::SkipLastOpen.get_str("ch").unwrap(),
                                        CompilerSigil::SkipLastClose.get_str("ch").unwrap(),
                                    )
                                })
                            }
                            parts.push(CompilerToken::SkipLast(buffer_key.clone()));
                            state = CompilerTokenizerState::Copying(String::new());
                        }
                        CompilerSigil::TokenEmbed => {
                            state = CompilerTokenizerState::CopyingSkipLastEmbed(buffer_key.to_owned())
                        }
                        _ => buffer_key.push(ch)
                    }
                }
                CompilerTokenizerState::CopyingSkipLastEmbed(ref mut buffer_key) => {
                    // log::trace!("sle: {ch}");
                    match CompilerSigil::from(ch) {
                        CompilerSigil::SkipLastClose |
                        CompilerSigil::TokenEmbed => {
                            buffer_key.push(ch);
                        }
                        _ => {
                            return Err(Error{
                                kind: ErrorKind::IllegalSymbol,
                                message: format!(
                                    "Expected a {:?} symbol {:?} or {:?} symbol {:?} after {ch}",
                                        CompilerSigil::SkipLastClose,
                                        CompilerSigil::SkipLastClose.get_str("ch"),
                                        CompilerSigil::TokenEmbed,
                                        CompilerSigil::TokenEmbed.get_str("ch")
                                )
                            })
                        }
                    }
                    state = CompilerTokenizerState::CopyingSkipLast(buffer_key.to_owned());
                }
            }
        }

        log::trace!(
            "{}: {}",
            format!("[CompilerToken::tokenize]").bold(),
            format!("Last state {:?}", state).dimmed()
        );
        match state {
            CompilerTokenizerState::Copying(buffer) => {
                if !buffer.is_empty() {
                    parts.push(CompilerToken::Raw(buffer))
                }
            }
            CompilerTokenizerState::EmbedFound(_) => {
                return Err(Error{
                    kind: ErrorKind::IllegalSymbol,
                    message: format!(
                        "Expected a {:?} symbol {:?} or {:?} symbol {:?} after {:?}",
                            CompilerSigil::TokenStart,
                            CompilerSigil::TokenStart.get_str("ch"),
                            CompilerSigil::TokenEmbed,
                            CompilerSigil::TokenEmbed.get_str("ch"),
                            CompilerSigil::TokenEmbed.get_str("ch")
                    )
                })
            }
            CompilerTokenizerState::SigilFound => {
                return Err(Error {
                    kind: ErrorKind::InvalidToken,
                    message: format!(
                        "'{:?}' symbol '{:?}' found with no body to go along side it in '{}'", 
                        CompilerSigil::TokenStart, CompilerSigil::TokenStart.get_str("ch"), s
                    )
                })
            }
            CompilerTokenizerState::CopyingNamedArgumentRef(_) |
            CompilerTokenizerState::CopyingUnamedArgumentRef(_) |
            CompilerTokenizerState::CopyingSkipLastEmbed(_) |
            CompilerTokenizerState::CopyingSkipLast(_) => {
                return Err(Error {
                    kind: ErrorKind::InvalidToken,
                    message: format!(
                        "Unfinished token at the end of a compilable '{}'", s)
                })
            }
        }

        Ok(parts)

    }

    fn tokenize_surface(
        s: &str
    ) -> Result<Vec<CompilerToken>, Error> {

        let mut tokens = Self::tokenize(s)?;

        for token in tokens.iter_mut() {

            match token.get_bool("surface") {
                Some(true) => (),
                None | Some(false) => {
                    log::trace!(
                        "{}: {}",
                        format!("[CompilerToken::tokenize_surface]").bold(),
                        format!("Untokenized token: {:?}", token).dimmed()
                    );
                    *token = CompilerToken::Raw(token.untokenize())
                }
            }

        }

        Ok(tokens)

    }

    fn untokenize(&self) -> String {
        match self {
            Self::Raw(value) => value
                .replace( // first.
                    format!("{}", 
                        CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                    ).as_str(), 
                    format!("{}{}",
                        CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                        CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                    ).as_str()
                )
                .replace(
                    format!("{}",
                        CompilerSigil::TokenStart.get_str("ch").unwrap()
                    ).as_str(), 
                    format!("{}{}",
                        CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                        CompilerSigil::TokenStart.get_str("ch").unwrap()
                    ).as_str()
                ).to_owned(),
            Self::Position => 
                format!("{}{}", 
                    CompilerSigil::TokenStart.get_str("ch").unwrap(),
                    CompilerSigil::PositionDot.get_str("ch").unwrap()
                ),
            Self::NamedArgumentRef(value) => 
                CompilerSigil::TokenStart.get_str("ch").unwrap().to_owned() +
                CompilerSigil::NamedArgumentRefOpen.get_str("ch").unwrap() +
                value.to_string().as_str() + 
                CompilerSigil::NamedArgumentRefClose.get_str("ch").unwrap(),
            Self::UnamedArgumentRef(value) => 
                CompilerSigil::TokenStart.get_str("ch").unwrap().to_owned() +
                CompilerSigil::UnamedArgumentRefOpen.get_str("ch").unwrap() +
                value.to_string().as_str() + 
                CompilerSigil::UnamedArgumentRefClose.get_str("ch").unwrap(),
            Self::SkipLast(value) => 
                CompilerSigil::TokenStart.get_str("ch").unwrap().to_owned() +
                CompilerSigil::SkipLastOpen.get_str("ch").unwrap() +
                value
                    .replace( // first.
                        format!("{}", 
                            CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                        ).as_str(), 
                        format!("{}{}",
                            CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                            CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                        ).as_str()
                    )
                    .replace(
                        format!("{}", 
                            CompilerSigil::SkipLastClose.get_str("ch").unwrap(),
                        ).as_str(), 
                        format!("{}{}",
                            CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                            CompilerSigil::SkipLastClose.get_str("ch").unwrap(),
                        ).as_str()
                    )
                    .as_str() + 
                CompilerSigil::SkipLastClose.get_str("ch").unwrap()
            
        }
    }

}




fn compile_surface_string(
    compilable_string: PreprocessableString,
    named: &HashMap<String, PreprocessableString>
) -> Result<(), Error> {

    log::trace!("{}", 
        format!("Attempting to surface compile `{:?}`.", compilable_string)
        .dimmed()
    );

    let string_guard = compilable_string.read()
        .map_err(|err| Error {
            kind: ErrorKind::PoisonedLock,
            message: err.to_string()
        })?;

    let inner = match &*string_guard {
        Preprocessable::NotPreprocessed(_) => {
            return Err(Error { 
                kind: ErrorKind::NotPreprocessed, 
                message: 
                format!(
                    "Recived a string that was not preprocessed during the compilation process: {:?}",
                    string_guard
                )
            })
        }
        Preprocessable::Preprocessed(value) => value
    };

    let tokens =  CompilerToken::tokenize_surface(inner)?;
    let mut compiled_surface_string = String::new();

    for token in tokens {

        match token {
            CompilerToken::Raw(ref value) => compiled_surface_string.push_str(value),
            CompilerToken::NamedArgumentRef(ref value) => {
                let Some(entry) = named.get(value) else {
                    return Err(Error { 
                        kind: ErrorKind::NonExistantArgument, 
                        message: format!("Argument with key '{}' does not exist, occured when trying to compile '{:?}'", value, inner)
                    })
                };
                let entry_guard = entry.read()
                    .map_err(|err| Error {
                        kind: ErrorKind::PoisonedLock,
                        message: err.to_string()
                    })?;
                let entry_inner = match &*entry_guard {
                    Preprocessable::NotPreprocessed(_) => {
                        return Err(Error { 
                            kind: ErrorKind::NotPreprocessed, 
                            message: 
                            format!(
                                "Recived a string that was not preprocessed during the compilation process: {:?}",
                                string_guard
                            )
                        })
                    }
                    Preprocessable::Preprocessed(value) => value
                };
                compiled_surface_string.push_str(entry_inner);
            }
            _ => unreachable!()
        }

    }

    log::trace!("{}",
        format!("Surface compiled from `{inner}` -> `{compiled_surface_string}`.")
        .cyan().dimmed()
    );

    drop(string_guard);

    let mut string_writer = compilable_string.write()
        .map_err(|err| Error {
            kind: ErrorKind::PoisonedLock,
            message: err.to_string()
        })?;
    
    *string_writer = Preprocessable::Preprocessed(compiled_surface_string);


    Ok(())

}


fn compile_surface_strings(
    compilable_strings: Vec<PreprocessableString>,
    named: &HashMap<String, PreprocessableString>
) -> Result<(), Error> {

    for compilable in compilable_strings {
        compile_surface_string(
            compilable,
            named
        )?;
    }

    Ok(())

}

impl Config {

    fn load_named_arguments(
        &self,
    ) -> Result<HashMap<String, PreprocessableString>, Error> {

        let mut table: HashMap<String, PreprocessableString> = HashMap::new();
        for arg in self.core.args.iter() { 
            match arg {
                &Argument::Named(ref named) => {
                    let result = table.insert(named.key.clone(), named.name.clone());
                    if result.is_some() {
                        return Err(Error {
                            kind: ErrorKind::DuplicateArgument,
                            message: format!("duplicate argument for the main xmva: {:?}", result.unwrap())
                        })
                    }
                }
                &Argument::Varadict { varadict: _ } => ()
            }
        }

        log::trace!("{}",
            format!("Named arguments: {:#?}", table)
            .dimmed()
        );

        Ok(table)

    }

    fn load_surface_compile_strings(
        &self
    ) -> Vec<PreprocessableString> {

        let mut compilable_strings: Vec<PreprocessableString> = vec![];

        for generator in self.generator.iter() {

            compilable_strings.push(generator.fallbacks.empty.clone());
            compilable_strings.push(generator.fallbacks.unparity.clone());
            compilable_strings.push(generator.postamble.clone());
            compilable_strings.push(generator.preamble.clone());
            compilable_strings.push(generator.repeat.clone());

        }

        log::trace!("{}",
            format!("Surface compilable string: {:#?}", compilable_strings)
            .dimmed()
        );
        
        return compilable_strings

    }

    pub fn compile(
        &self
    ) -> Result<String, Error> {

        log::debug!("Starting to compile the config.");

        log::debug!("Loading named arguments...");
        let named = self.load_named_arguments()?;
        log::debug!("Loading all surface ŋcompilable strings...");
        let compilable_strings = self.load_surface_compile_strings();
        log::debug!("Compiling only named arguments into compilable strings...");
        compile_surface_strings(
            compilable_strings,
            &named
        )?;






        


        unimplemented!()

    }

}


mod tests {

    #[allow(unused_imports)]
    use std::io::empty;
    #[allow(unused_imports)]
    use strum::IntoEnumIterator;
    #[allow(unused_imports)]
    use super::*;

    /// The tokneizer is both the only thing that interacts with user strings
    /// and the most complex part of the preprocessor.
    /// Everything else is pretty simple and relies on enums to guide
    /// the code.

    #[test]
    fn tokenizer_string_simple() {

        // Simple
        assert_eq!(
            CompilerToken::tokenize(
                "hello world${argument}"
            ).unwrap(),
            vec![
                CompilerToken::Raw("hello world".to_owned()),
                CompilerToken::NamedArgumentRef("argument".to_owned())
            ]
        );

    }

    #[test]
    fn tokenizer_complex() {

        // Complex
        assert_eq!(
            CompilerToken::tokenize(
                "$.$.[HELLO_ ## ${NAME} ## _ ## $(000)] = \"\\$$(01)$[,\\]]\""
            ).unwrap(),
            vec![
                CompilerToken::Position,
                CompilerToken::Position,
                CompilerToken::Raw("[HELLO_ ## ".to_owned()),
                CompilerToken::NamedArgumentRef("NAME".to_owned()),
                CompilerToken::Raw(" ## _ ## ".to_owned()),
                CompilerToken::UnamedArgumentRef(0),
                CompilerToken::Raw("] = \"$".to_owned()),
                CompilerToken::UnamedArgumentRef(1),
                CompilerToken::SkipLast(",]".to_owned()),
                CompilerToken::Raw("\"".to_owned())
            ]
        );
    
    }

    #[test]
    fn tokenizer_surface() {

        // Complex
        assert_eq!(
            CompilerToken::tokenize_surface(
                "$.$.[HELLO_ ## ${NAME} ## _ ## $(000)] = \"\\$$(01)$[,\\]]\""
            ).unwrap(),
            vec![
                CompilerToken::Raw("$.".to_owned()),
                CompilerToken::Raw("$.".to_owned()),
                CompilerToken::Raw("[HELLO_ ## ".to_owned()),
                CompilerToken::NamedArgumentRef("NAME".to_owned()),
                CompilerToken::Raw(" ## _ ## ".to_owned()),
                CompilerToken::Raw("$(0)".to_owned()),
                CompilerToken::Raw("] = \"$".to_owned()),
                CompilerToken::Raw("$(1)".to_owned()),
                CompilerToken::Raw("$[,\\]]".to_owned()),
                CompilerToken::Raw("\"".to_owned())
            ]
        );

    }

    #[test]
    fn untokenizer() {
        let variants: Vec<CompilerToken> = CompilerToken::iter()
            .collect();

        for ref variant in variants {
            match variant {
                CompilerToken::Raw(value) => {
                    assert_eq!(value.to_owned(), variant.untokenize())
                }
                CompilerToken::Position => {
                    assert_eq!(
                        format!("{}{}",
                            CompilerSigil::TokenStart.get_str("ch").unwrap(),
                            CompilerSigil::PositionDot.get_str("ch").unwrap(),
                        ),
                        variant.untokenize()
                    )
                }
                CompilerToken::NamedArgumentRef(value) => {
                    assert_eq!(
                        format!("{}{}{value}{}",
                            CompilerSigil::TokenStart.get_str("ch").unwrap(),
                            CompilerSigil::NamedArgumentRefOpen.get_str("ch").unwrap(),
                            CompilerSigil::NamedArgumentRefClose.get_str("ch").unwrap()
                        ), 
                        variant.untokenize()
                    )
                }
                CompilerToken::UnamedArgumentRef(value) => {
                    assert_eq!(
                        format!("{}{}{value}{}",
                            CompilerSigil::TokenStart.get_str("ch").unwrap(),
                            CompilerSigil::UnamedArgumentRefOpen.get_str("ch").unwrap(),
                            CompilerSigil::UnamedArgumentRefClose.get_str("ch").unwrap()
                        ), 
                        variant.untokenize()
                    )
                }
                CompilerToken::SkipLast(value) => {
                    assert_eq!(
                        format!("{}{}{value}{}",
                            CompilerSigil::TokenStart.get_str("ch").unwrap(),
                            CompilerSigil::SkipLastOpen.get_str("ch").unwrap(),
                            CompilerSigil::SkipLastClose.get_str("ch").unwrap()
                        ), 
                        variant.untokenize()
                    )
                }
            }
        }
    }

    #[test] 
    // probably the most comprehensive test here, usually the first one 
    // to fail if something went wrong or we added a new feature.
    // be sure to not use stupid numbers like 00001 instead of 1 since the
    // compiler cant read your thoughts only what you wrote.
    fn tokenize_and_untokenize() {

        let s = "$.$.[HELLO_ ## ${NAME} ## _ ## $(0)] = \"\\$$(1)$[,\\]\\\\]\"";

        assert_eq!(
            s.to_owned(),
            //Tokenize and untokenize
            CompilerToken::tokenize(s)
                .unwrap()
                .into_iter()
                .map(|x| x.untokenize())
                .collect::<Vec<String>>()
                .join("")
        );

        let s1 = "\\$$.\\\\{[gello${wo.rld💔🔥}${..skadkAK100'0'💔💔💔🔥@..}]}$[,,,💔🔥.sfak\\]\\]\\]]$.$(12031000)$.\\$$(1000)$[\\\\\\]\\]\\]]\\\\\\$";
        
        assert_eq!(
            s1.to_owned(),
            //Tokenize and untokenize
            CompilerToken::tokenize(s1)
                .unwrap()
                .into_iter()
                .map(|x| x.untokenize())
                .collect::<Vec<String>>()
                .join("")
        );

    }

    #[test]
    fn tokenizer_errors() {

        let empty_named = "${}";
        let empty_unamed = "$()";
        let empty_skip = "$[]";
        let illegal_sigil_named = "${{}";
        let illegal_sigil_unamed = "$(()";
        let illegal_sigil_skip = "$[[]";

        assert_eq!(
            CompilerToken::tokenize(empty_named).unwrap_err().kind,
            //Tokenize and untokenize
            ErrorKind::EmptyReference
        );

        assert_eq!(
            CompilerToken::tokenize(empty_unamed).unwrap_err().kind,
            //Tokenize and untokenize
            ErrorKind::EmptyReference
        );

        assert_eq!(
            CompilerToken::tokenize(empty_skip).unwrap_err().kind,
            //Tokenize and untokenize
            ErrorKind::EmptyReference
        );

        assert_eq!(
            CompilerToken::tokenize(illegal_sigil_named).unwrap_err().kind,
            //Tokenize and untokenize
            ErrorKind::IllegalSymbol
        );

        assert_eq!(
            CompilerToken::tokenize(illegal_sigil_unamed).unwrap_err().kind,
            //Tokenize and untokenize
            ErrorKind::IllegalSymbol
        );

        assert!(
            CompilerToken::tokenize(illegal_sigil_skip).is_ok()
        );

    }

    #[test]
    fn tokenizer_embed() {

        assert_eq!(
            CompilerToken::tokenize("").unwrap(),
            vec![]
        );

        assert!(
            CompilerToken::tokenize("\\").is_err()
        );

        assert!(
            CompilerToken::tokenize("\\s").is_err()
        );

        assert_eq!(
            CompilerToken::tokenize("\\\\").unwrap(),
            vec![CompilerToken::Raw("\\".to_owned())]
        );

        assert_eq!(
            CompilerToken::tokenize("\\$").unwrap(),
            vec![CompilerToken::Raw("$".to_owned())]
        );

        assert_eq!(
            CompilerToken::tokenize("\\\\n\\$\\\\\\\\%\\\\").unwrap(),
            vec![CompilerToken::Raw("\\n$\\\\%\\".to_owned())]
        );

        assert_eq!(
            CompilerToken::tokenize("$[\\\\n\\\\$\\\\\\\\%\\]]").unwrap(),
            vec![CompilerToken::SkipLast("\\n\\$\\\\%]".to_owned())]
        );


    }



}