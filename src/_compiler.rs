use std::{collections::HashMap, mem::discriminant, usize};

use colored::Colorize;
use strum::{EnumIter, EnumProperty};

use crate::{
    _config::{Argument, Common, Config, Core, Generator}, preprocessor::{Preprocessable, PreprocessableString}, sigil::CompilerSigil
};

const REPEAT_SECTION_SUFFIX: &'static str = "__ARGS__";
const GENERATOR_SUFFIX: &'static str = "__GENERATOR__";


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
    pub(crate) message: String
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

fn generate_repeat_name(
    common: &Common,
    n: usize,
    suffix: usize
) -> String {

    let mut name = String::new();
    name.push_str(&common.keyable.prefix);
    name.push_str(REPEAT_SECTION_SUFFIX);
    name.push_str(suffix.to_string().as_str());
    name.push('_');
    name.push_str(n.to_string().as_str());
    name

}

fn generate_repeat_picker_macro_name(
    common: &Common,
    suffix: usize
) -> String {

    let mut name = String::new();
    name.push_str(&common.keyable.prefix);
    name.push_str(REPEAT_SECTION_SUFFIX);
    name.push_str(suffix.to_string().as_str());
    name

}

fn compile_and_assemble_repeat_string(
    generator: &Generator,
    common:    &Common,
    core:      &Core,
    suffix:    usize
) -> Result<String, Error> {

    let read_guard = generator.fallbacks.empty.read()
         .map_err(|err| Error {
            kind: ErrorKind::PoisonedLock,
            message: err.to_string()
        })?;

    match &*read_guard {
        Preprocessable::NotPreprocessed(_) => {
            return Err(Error { 
                kind: ErrorKind::NotPreprocessed, 
                message: 
                format!(
                    "Recived a string that was not preprocessed during the compilation process: {:?}",
                    read_guard
                )
            })
        }
        Preprocessable::Preprocessed(s) => s.clone()
    };
    drop(read_guard);

    let read_guard = generator.fallbacks.unparity.read()
         .map_err(|err| Error {
            kind: ErrorKind::PoisonedLock,
            message: err.to_string()
        })?;

    let fallback_unparity = match &*read_guard {
        Preprocessable::NotPreprocessed(_) => {
            return Err(Error { 
                kind: ErrorKind::NotPreprocessed, 
                message: 
                format!(
                    "Recived a string that was not preprocessed during the compilation process: {:?}",
                    read_guard
                )
            })
        }
        Preprocessable::Preprocessed(s) => s.clone()
    };
    drop(read_guard);

    let read_guard = generator.fallbacks.empty.read()
         .map_err(|err| Error {
            kind: ErrorKind::PoisonedLock,
            message: err.to_string()
        })?;

    let fallback_empty = match &*read_guard {
        Preprocessable::NotPreprocessed(_) => {
            return Err(Error { 
                kind: ErrorKind::NotPreprocessed, 
                message: 
                format!(
                    "Recived a string that was not preprocessed during the compilation process: {:?}",
                    read_guard
                )
            })
        }
        Preprocessable::Preprocessed(s) => s.clone()
    };
    drop(read_guard);

    let read_guard = generator.preamble.read()
         .map_err(|err| Error {
            kind: ErrorKind::PoisonedLock,
            message: err.to_string()
        })?;

    let preamble = match &*read_guard {
        Preprocessable::NotPreprocessed(_) => {
            return Err(Error { 
                kind: ErrorKind::NotPreprocessed, 
                message: 
                format!(
                    "Recived a string that was not preprocessed during the compilation process: {:?}",
                    read_guard
                )
            })
        }
        Preprocessable::Preprocessed(s) => s.clone()
    };
    drop(read_guard);

    let read_guard = generator.postamble.read()
         .map_err(|err| Error {
            kind: ErrorKind::PoisonedLock,
            message: err.to_string()
        })?;

    let postamble = match &*read_guard {
        Preprocessable::NotPreprocessed(_) => {
            return Err(Error { 
                kind: ErrorKind::NotPreprocessed, 
                message: 
                format!(
                    "Recived a string that was not preprocessed during the compilation process: {:?}",
                    read_guard
                )
            })
        }
        Preprocessable::Preprocessed(s) => s.clone()
    };
    drop(read_guard);

    let mut named_args: Vec<String> = vec![];
    let mut some_va_args: Option<usize> = None;

    for args in core.args.iter() {
        match args {
            Argument::Named(named) => {
                let read_guard = named.name.read()
                    .map_err(|err| Error {
                        kind: ErrorKind::PoisonedLock,
                        message: err.to_string()
                    })?;

                match &*read_guard {
                    Preprocessable::NotPreprocessed(_) => {
                        return Err(Error { 
                            kind: ErrorKind::NotPreprocessed, 
                            message: 
                            format!(
                                "Recived a string that was not preprocessed during the compilation process: {:?}",
                                read_guard
                            )
                        })
                    }
                    Preprocessable::Preprocessed(s) => named_args.push(s.clone())
                };
            }
            Argument::Varadict { varadict } => {
                if some_va_args.is_some() {
                    return Err(Error { 
                        kind: ErrorKind::DuplicateArgument, 
                        message: "2 or more conflicting varadict argument count arguments".to_string()
                    })
                }
                some_va_args = Some(*varadict)
            }
        }
    }

    let Some(va_args) = some_va_args else {
        return Err(Error { 
            kind: ErrorKind::NonExistantArgument, 
            message: "Missing varadict argument count argument".to_string()
        })
    };

    let read_guard = generator.repeat.read()
         .map_err(|err| Error {
            kind: ErrorKind::PoisonedLock,
            message: err.to_string()
        })?;

    let le_stranger = match &*read_guard {
        Preprocessable::NotPreprocessed(_) => {
            return Err(Error { 
                kind: ErrorKind::NotPreprocessed, 
                message: 
                format!(
                    "Recived a string that was not preprocessed during the compilation process: {:?}",
                    read_guard
                )
            })
        }
        Preprocessable::Preprocessed(le_stranger) => le_stranger
    };

    let le_tokens = CompilerToken::tokenize(le_stranger)?;
    let mut generated_repeats = String::new();

    generated_repeats.push_str("#define ");
    generated_repeats.push_str(generate_repeat_name(common, 0, suffix).as_str());
    generated_repeats.push('(');
    generated_repeats.push_str(named_args.join(", ").as_str());
    generated_repeats.push(')');
    generated_repeats.push(' ');
    generated_repeats.push_str(fallback_empty.as_str());
    generated_repeats.push('\n');

    for current_repetiton in 1..common.repeats {
        
        generated_repeats.push_str("#define ");
        generated_repeats.push_str(generate_repeat_name(common, current_repetiton, suffix).as_str());
        generated_repeats.push('(');
        generated_repeats.push_str(named_args.join(", ").as_str());
        generated_repeats.push_str(", ");
        generated_repeats.push_str(
            (0..current_repetiton)
                .map(|i| format!("__{i}__"))
                .collect::<Vec<String>>()
                .join(", ")
                .as_str()
        );
        generated_repeats.push(')');
        generated_repeats.push(' ');

        
        if current_repetiton % va_args == 0 {

            generated_repeats.push_str(preamble.as_str());

            let j = current_repetiton/va_args;

            for i in 0..j {
                
                for token in le_tokens.iter() {

                    match token {
                        CompilerToken::NamedArgumentRef(_) => unreachable!(),
                        CompilerToken::Raw(s) => {
                            generated_repeats.push_str(s)
                        }
                        CompilerToken::Position => {
                            generated_repeats.push_str((i+1).to_string().as_str());
                        }
                        CompilerToken::UnamedArgumentRef(n) => {
                            generated_repeats.push_str(format!("__{}__", n + i*va_args).as_str())
                        }
                        CompilerToken::SkipLast(s) => {
                            if j-1 != i {
                                generated_repeats.push_str(s);
                            }
                        }
                    }
                }
            }

            generated_repeats.push_str(postamble.as_str())

        } else {
            generated_repeats.push_str(fallback_unparity.as_str());
        }

        generated_repeats.push('\n');

    }

    generated_repeats.push_str("#define ");
    generated_repeats.push_str(generate_repeat_picker_macro_name(common, suffix).as_str());
    generated_repeats.push('(');
    generated_repeats.push_str(
    (0..common.repeats)
        .map(|i| format!("__{i}__"))
        .collect::<Vec<String>>()
        .join(", ")
        .as_str()
    );
    generated_repeats.push_str(", __NAME__, ...) __NAME__");

    Ok(generated_repeats)

}


fn generate_generator_macro_name(
    common: &Common,
    suffix: usize
) -> String {

    let mut name = String::new();
    name.push_str(&common.keyable.prefix);
    name.push_str(GENERATOR_SUFFIX);
    name.push_str(suffix.to_string().as_str());
    name

}


fn assemble_generator_macro_string(
    common: &Common,
    core: &Core,
    suffix: usize
) -> Result<String, Error> {

    let mut named_args: Vec<String> = vec![];

    for args in core.args.iter() {
        match args {
            Argument::Named(named) => {
                let read_guard = named.name.read()
                    .map_err(|err| Error {
                        kind: ErrorKind::PoisonedLock,
                        message: err.to_string()
                    })?;

                match &*read_guard {
                    Preprocessable::NotPreprocessed(_) => {
                        return Err(Error { 
                            kind: ErrorKind::NotPreprocessed, 
                            message: 
                            format!(
                                "Recived a string that was not preprocessed during the compilation process: {:?}",
                                read_guard
                            )
                        })
                    }
                    Preprocessable::Preprocessed(s) => named_args.push(s.clone())
                };
            }
            Argument::Varadict { varadict: _ } => ()
        }
    }


    let mut generator_macro = String::new();

    generator_macro.push_str("#define ");
    generator_macro.push_str(generate_generator_macro_name(common, suffix).as_str());
    generator_macro.push('(');
    generator_macro.push_str(named_args.join(", ").as_str());
    generator_macro.push_str(", __GEN__, ...");
    generator_macro.push(')');
    generator_macro.push(' ');
    generator_macro.push_str("__GEN__(");
    generator_macro.push_str(named_args.join(", ").as_str());
    generator_macro.push_str(", __VA_ARGS__)");
    generator_macro.push('\n');

    Ok(generator_macro)
    
}

fn assemble_main_macro_string(
    core: &Core,
    common: &Common,
    generator_count: usize
) -> Result<String, Error> {

    let read_guard = core.xmva.read()
         .map_err(|err| Error {
            kind: ErrorKind::PoisonedLock,
            message: err.to_string()
        })?;

    let xmva = match &*read_guard {
        Preprocessable::NotPreprocessed(_) => {
            return Err(Error { 
                kind: ErrorKind::NotPreprocessed, 
                message: 
                format!(
                    "Recived a string that was not preprocessed during the compilation process: {:?}",
                    read_guard
                )
            })
        }
        Preprocessable::Preprocessed(s) => s.clone()
    };
    drop(read_guard);

    let mut named_args: Vec<String> = vec![];
    let mut some_va_args: Option<usize> = None;

    for args in core.args.iter() {
        match args {
            Argument::Named(named) => {
                let read_guard = named.name.read()
                    .map_err(|err| Error {
                        kind: ErrorKind::PoisonedLock,
                        message: err.to_string()
                    })?;

                match &*read_guard {
                    Preprocessable::NotPreprocessed(_) => {
                        return Err(Error { 
                            kind: ErrorKind::NotPreprocessed, 
                            message: 
                            format!(
                                "Recived a string that was not preprocessed during the compilation process: {:?}",
                                read_guard
                            )
                        })
                    }
                    Preprocessable::Preprocessed(s) => named_args.push(s.clone())
                };
            }
            Argument::Varadict { varadict } => {
                if some_va_args.is_some() {
                    return Err(Error { 
                        kind: ErrorKind::DuplicateArgument, 
                        message: "2 or more conflicting varadict argument count arguments".to_string()
                    })
                }
                some_va_args = Some(*varadict)
            }
        }
    }

    let mut main_macro = String::new();

    main_macro.push_str("#define ");
    main_macro.push_str(xmva.as_str());
    main_macro.push('(');
    main_macro.push_str(named_args.join(", ").as_str());
    main_macro.push_str(", ...) ");
    
    for i in 0..generator_count {
        main_macro.push_str(generate_generator_macro_name(common, i).as_str());
        main_macro.push('(');
        main_macro.push_str(named_args.join(", ").as_str());
        main_macro.push_str(", ");
        main_macro.push_str(&generate_repeat_picker_macro_name(common, i).as_str());
        main_macro.push_str("(\"empty\", ##__VA_ARGS__, ");
        main_macro.push_str(
        (0..common.repeats)
            .map(|j| generate_repeat_name(common, j, i))
            .rev()
            .collect::<Vec<String>>()
            .join(", ")
            .as_str()
        );
        main_macro.push_str("), __VA_ARGS__) ");
    }

    Ok(main_macro)

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

    fn load_surface_compilable_strings(
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



    fn assemble_preamble(
        &self
    ) -> Result<String, Error> {

        let mut assembled_preamble: String = String::new();

        match &self.preamble {
            Some(preamble) => { 
                match &preamble.raw {
                    Some(raw) => {
                        let read_guard = raw.read()
                            .map_err(|err| Error {
                                kind: ErrorKind::PoisonedLock,
                                message: err.to_string()
                            })?;
                        match &*read_guard {
                            Preprocessable::NotPreprocessed(_) => {
                                return Err(Error { 
                                    kind: ErrorKind::NotPreprocessed, 
                                    message: 
                                    format!(
                                        "Recived a string that was not preprocessed during the compilation process: {:?}",
                                        read_guard
                                    )
                                })
                            }
                            Preprocessable::Preprocessed(string) => {
                                assembled_preamble.push_str(string);
                                assembled_preamble.push('\n');
                            }
                        }
                    }
                    None => ()
                }
            }
            None => ()
        }

        if self.definition.is_some() {
            for definition in self.definition.clone().unwrap().iter() {

                assembled_preamble.push_str("#define ");

                let read_guard = definition.name.read()
                    .map_err(|err| Error {
                        kind: ErrorKind::PoisonedLock,
                        message: err.to_string()
                    })?;
                match &*read_guard {
                    Preprocessable::NotPreprocessed(_) => {
                        return Err(Error { 
                            kind: ErrorKind::NotPreprocessed, 
                            message: 
                            format!(
                                "Recived a string that was not preprocessed during the compilation process: {:?}",
                                read_guard
                            )
                        })
                    }
                    Preprocessable::Preprocessed(name) => {
                        assembled_preamble.push_str(name);
                    }
                }
                drop(read_guard);

                if definition.parameters.is_some() {
                    let parameters = definition.parameters.clone().unwrap();
                    assembled_preamble.push_str(
                        format!(
                            "({})",
                            parameters.join(", ")
                        ).as_str()
                    );
                }

                assembled_preamble.push(' ');

                let read_guard = definition.expansion.read()
                    .map_err(|err| Error {
                        kind: ErrorKind::PoisonedLock,
                        message: err.to_string()
                    })?;
                match &*read_guard {
                    Preprocessable::NotPreprocessed(_) => {
                        return Err(Error { 
                            kind: ErrorKind::NotPreprocessed, 
                            message: 
                            format!(
                                "Recived a string that was not preprocessed during the compilation process: {:?}",
                                read_guard
                            )
                        })
                    }
                    Preprocessable::Preprocessed(expansion) => {
                        assembled_preamble.push_str(expansion);
                        assembled_preamble.push('\n');
                    }
                }
                drop(read_guard);
            }
        }

        log::trace!("{}", format!("Created preamble: \n{}", assembled_preamble).dimmed());
        Ok(assembled_preamble)
        
    }



    pub fn compile_and_assemble(
        &self
    ) -> Result<String, Error> {

        log::debug!("Starting to compile the config.");

        log::debug!("Loading named arguments...");
        let named = self.load_named_arguments()?;
        log::debug!("Loading all surface compilable strings...");
        let compilable_strings = self.load_surface_compilable_strings();
        log::debug!("Surface compiling...");
        compile_surface_strings(
            compilable_strings,
            &named
        )?;

        // surface compile and then start assembling the file
        log::debug!("Assembling preamble...");
        let preamble = self.assemble_preamble()?;

        let mut repeats: Vec<String> =  vec![];
        let mut generators: Vec<String> = vec![];
        log::debug!("Compiling and assembling the repeat section, and assembling the generator macro...");
        for (i, generator) in self.generator.iter().enumerate() {

            repeats.push(
                compile_and_assemble_repeat_string(
                    &generator, 
                    &self.common, 
                    &self.core,
                    i
                )?
            );

            generators.push(
                assemble_generator_macro_string(
                    &self.common, 
                    &self.core,
                    i
                )?
            );

        }

        log::debug!("Assembling the main xmva macro...");
        let xmva = assemble_main_macro_string(
            &self.core, 
            &self.common, 
            self.generator.len()
        )?;


        log::debug!("Assembling file contents...");
        let file = format!("{preamble}\n{}\n{}\n{xmva}",
            repeats.join("\n"),
            generators.join("\n")
        );

        Ok(file)

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