
use std::{mem::discriminant};

use backtrace::Backtrace;
use colored::Colorize;
use miette::LabeledSpan;
use strum::{EnumProperty, EnumIter};
use toml::Spanned;

use crate::{backtrace, error::Error, metadata::Metadata, sigil::CompilerSigil};

#[derive(Debug, PartialEq, Eq, EnumProperty, EnumIter)]
pub enum CompilerToken {
    Raw(String),
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

    pub fn tokenize(
        spanned_s: &Spanned<String>,
        metadata: &Metadata,
    ) -> miette::Result<Vec<CompilerToken>> {

        let mut parts: Vec<CompilerToken> = vec![];
        let mut state: CompilerTokenizerState 
            = CompilerTokenizerState::Copying(String::new());
        let mut prev_state = state.clone();
        let activity = "compiling".to_owned();

        let s = spanned_s.get_ref();
        let span = spanned_s.span();

        for (index, ch) in s.chars().enumerate() {

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
                            return Err(Error::IllegalSymbol { 
                                src: metadata.named_source.clone(), 
                                span: vec![LabeledSpan::new_primary_with_span(
                                    Some(format!(
                                        "Unexpected character '{}' after {:?} symbol '{}'.",
                                        ch, CompilerSigil::TokenEmbed,
                                        CompilerSigil::TokenEmbed.get_str("ch").unwrap()
                                    )),
                                    span.start + index..std::cmp::min(span.end, index+1)
                                )], 
                                backtrace: backtrace!(Backtrace::new()), 
                                extra: Some(format!(
                                    "After a {:?} symbol '{}' we expect either a {:?} - '{}' or a {:?} - '{}' symbol.",
                                    CompilerSigil::TokenEmbed,
                                    CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                                    CompilerSigil::TokenStart,
                                    CompilerSigil::TokenStart.get_str("ch").unwrap(),
                                    CompilerSigil::TokenEmbed,
                                    CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                                )),
                                activity
                            }.into())
                        }
                    }
                    state = CompilerTokenizerState::Copying(buffer.clone());
                }
                CompilerTokenizerState::SigilFound => {
                    match CompilerSigil::from(ch) {  
                        CompilerSigil::TokenStart => {
                            return Err(Error::IllegalSymbol { 
                                src: metadata.named_source.clone(), 
                                span: vec![LabeledSpan::new_primary_with_span(
                                    Some(format!(
                                        "Duplicate character '{}' after {:?} symbol '{}'.",
                                        ch, CompilerSigil::TokenEmbed,
                                        CompilerSigil::TokenEmbed.get_str("ch").unwrap()
                                    )),
                                    span.start + index..std::cmp::min(span.end, index+1)
                                )], 
                                backtrace: backtrace!(Backtrace::new()),
                                extra: None,
                                activity
                            }.into())
                            // return Err(Error{
                            //     kind: ErrorKind::IllegalSymbol,
                            //     message: format!(
                            //         "Duplicate symbol '{}' in '{}' twice or more in a row", ch, s
                            //     )
                            // })
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
                            return Err(Error::IllegalSymbol { 
                                src: metadata.named_source.clone(), 
                                span: vec![LabeledSpan::new_primary_with_span(
                                    Some(format!(
                                        "Illegal non sigil character '{}' after {:?} symbol '{}'.",
                                        ch, CompilerSigil::TokenEmbed,
                                        CompilerSigil::TokenEmbed.get_str("ch").unwrap()
                                    )),
                                    span.start + index..std::cmp::min(span.end, index+1)
                                )], 
                                backtrace: backtrace!(Backtrace::new()),
                                extra: None, 
                                activity
                            }.into())
                            // return Err(Error {
                            //     kind: ErrorKind::IllegalSymbol,
                            //     message: format!(
                            //         "Illegal character '{}' in '{}' after '{:?}' symbol '{:?}' ", 
                            //         ch, s, CompilerSigil::TokenStart, CompilerSigil::TokenStart.get_str("ch")
                            //     )
                            // })
                        }
                    }
                }
                CompilerTokenizerState::CopyingNamedArgumentRef(ref mut buffer_key) => {
                    match CompilerSigil::from(ch) {
                        CompilerSigil::NamedArgumentRefClose => {
                            if buffer_key.is_empty() {
                                return Err(Error::EmptyReference { 
                                    src: metadata.named_source.clone(), 
                                    span: vec![LabeledSpan::new_primary_with_span(
                                        Some(format!(
                                            "Expected a name between '{}'...'{}'.",
                                            CompilerSigil::NamedArgumentRefOpen.get_str("ch").unwrap(),
                                            CompilerSigil::NamedArgumentRefClose.get_str("ch").unwrap()
                                        )),
                                        span.start + index-2..index
                                    )], 
                                    backtrace: backtrace!(Backtrace::new()), 
                                    extra: None,
                                    activity
                                }.into())
                            }
                            parts.push(CompilerToken::NamedArgumentRef(buffer_key.clone()));
                            state = CompilerTokenizerState::Copying(String::new());
                        }

                        CompilerSigil::PositionDot |
                        CompilerSigil::Non(_) => buffer_key.push(ch),

                        CompilerSigil::UnamedArgumentRefOpen |
                        CompilerSigil::UnamedArgumentRefClose |
                        CompilerSigil::SkipLastOpen |
                        CompilerSigil::SkipLastClose | 
                        CompilerSigil::NamedArgumentRefOpen |
                        CompilerSigil::TokenStart | 
                        CompilerSigil::TokenEmbed => {
                            return Err(Error::IllegalSymbol { 
                                src: metadata.named_source.clone(), 
                                span: vec![LabeledSpan::new_primary_with_span(
                                    Some(format!(
                                        "Illegal character here."
                                    )),
                                    span.start + index..std::cmp::min(span.end, index+1)
                                )], 
                                backtrace: backtrace!(Backtrace::new()),
                                extra: Some(format!(
                                    "The compiler expected a {:?} - {} symbol since the compiler tokenizer state was {:?}",
                                    CompilerSigil::NamedArgumentRefClose,
                                    CompilerSigil::NamedArgumentRefClose.get_str("ch").unwrap(),
                                    state.clone()
                                )), 
                                activity
                            }.into())
                        }
                    }
                }
                CompilerTokenizerState::CopyingUnamedArgumentRef(ref mut buffer_key) => {
                    match CompilerSigil::from(ch) {
                        CompilerSigil::UnamedArgumentRefClose => {
                            if buffer_key.is_empty() {
                                return Err(Error::EmptyReference { 
                                    src: metadata.named_source.clone(), 
                                    span: vec![LabeledSpan::new_primary_with_span(
                                        Some(format!(
                                            "Expected a number between '{}'...'{}'.",
                                            CompilerSigil::UnamedArgumentRefOpen.get_str("ch").unwrap(),
                                            CompilerSigil::UnamedArgumentRefClose.get_str("ch").unwrap()
                                        )),
                                        span.start + index-2..index
                                    )], 
                                    backtrace: backtrace!(Backtrace::new()), 
                                    extra: None,
                                    activity
                                }.into())
                            }
                            let Ok(value) = buffer_key.clone().parse::<usize>() else {
                                return Err(Error::InvalidReference { 
                                    src: metadata.named_source.clone(), 
                                    span: vec![LabeledSpan::new_primary_with_span(
                                        Some(format!(
                                            "Expected a number between '{}'...'{}'.",
                                            CompilerSigil::UnamedArgumentRefOpen.get_str("ch").unwrap(),
                                            CompilerSigil::UnamedArgumentRefClose.get_str("ch").unwrap()
                                        )),
                                        span.start+index-1-buffer_key.len()..index-1
                                    )], 
                                    backtrace: backtrace!(Backtrace::new()), 
                                    extra: Some(format!(
                                        "'{}' failed to be converted into a numerical type, perhaps the value is wrong?", buffer_key
                                    )),
                                    activity
                                }.into())
                            };
                            parts.push(CompilerToken::UnamedArgumentRef(value));
                            state = CompilerTokenizerState::Copying(String::new());
                        }
                        CompilerSigil::PositionDot |
                        CompilerSigil::Non(_) => buffer_key.push(ch),

                        CompilerSigil::NamedArgumentRefOpen |
                        CompilerSigil::NamedArgumentRefClose |
                        CompilerSigil::SkipLastOpen |
                        CompilerSigil::SkipLastClose | 
                        CompilerSigil::UnamedArgumentRefOpen |
                        CompilerSigil::TokenStart | 
                        CompilerSigil::TokenEmbed => {
                            return Err(Error::IllegalSymbol { 
                                src: metadata.named_source.clone(), 
                                span: vec![LabeledSpan::new_primary_with_span(
                                    Some(format!(
                                        "Illegal character here."
                                    )),
                                    span.start + index..std::cmp::min(span.end, index+1)
                                )], 
                                backtrace: backtrace!(Backtrace::new()),
                                extra: Some(format!(
                                    "The compiler expected a {:?} - {} symbol since the compiler tokenizer state was {:?}",
                                    CompilerSigil::UnamedArgumentRefClose,
                                    CompilerSigil::UnamedArgumentRefClose.get_str("ch").unwrap(),
                                    state.clone()
                                )), 
                                activity
                            }.into())
                        }
                    }
                }
                CompilerTokenizerState::CopyingSkipLast(ref mut buffer_key) => {
                    // log::trace!("sl: {ch}");
                    match CompilerSigil::from(ch) {
                        CompilerSigil::SkipLastClose => {
                            if buffer_key.is_empty() {
                                return Err(Error::EmptyReference { 
                                    src: metadata.named_source.clone(), 
                                    span: vec![LabeledSpan::new_primary_with_span(
                                        Some(format!(
                                            "Empty skip last token.",
                                        )),
                                        span.start + index-2..index
                                    )], 
                                    backtrace: backtrace!(Backtrace::new()), 
                                    extra: None,
                                    activity
                                }.into())
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
                    match CompilerSigil::from(ch) {
                        CompilerSigil::SkipLastClose |
                        CompilerSigil::TokenEmbed => {
                            buffer_key.push(ch);
                        }
                        _ => {
                            return Err(Error::IllegalSymbol { 
                                src: metadata.named_source.clone(), 
                                span: vec![LabeledSpan::new_primary_with_span(
                                    Some(format!(
                                        "Unexpected character '{}' after {:?} symbol '{}'.",
                                        ch, CompilerSigil::TokenEmbed,
                                        CompilerSigil::TokenEmbed.get_str("ch").unwrap()
                                    )),
                                    span.start + index..std::cmp::min(span.end, index+1)
                                )], 
                                backtrace: backtrace!(Backtrace::new()), 
                                extra: Some(format!(
                                    "After a {:?} symbol '{}' inside of a skip last token we expect either a {:?} - '{}' or a {:?} - '{}' symbol.",
                                    CompilerSigil::TokenEmbed,
                                    CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                                    CompilerSigil::SkipLastClose,
                                    CompilerSigil::SkipLastClose.get_str("ch").unwrap(),
                                    CompilerSigil::TokenEmbed,
                                    CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                                )),
                                activity
                            }.into())
                            // return Err(Error{
                            //     kind: ErrorKind::IllegalSymbol,
                            //     message: format!(
                            //         "Expected a {:?} symbol {:?} or {:?} symbol {:?} after {ch}",
                            //             CompilerSigil::SkipLastClose,
                            //             CompilerSigil::SkipLastClose.get_str("ch"),
                            //             CompilerSigil::TokenEmbed,
                            //             CompilerSigil::TokenEmbed.get_str("ch")
                            //     )
                            // })
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
                return Err(Error::IllegalSymbol { 
                    src: metadata.named_source.clone(), 
                    span: vec![LabeledSpan::new_primary_with_span(
                        Some(format!(
                            "Unexpected lone {:?} symbol '{}'.",
                            CompilerSigil::TokenEmbed,
                            CompilerSigil::TokenEmbed.get_str("ch").unwrap()
                        )),
                        span.start + s.len()..std::cmp::min(span.end, s.len()+1)
                    )], 
                    backtrace: backtrace!(Backtrace::new()), 
                    extra: Some(format!(
                        "After a {:?} symbol '{}' inside we expect either a {:?} - '{}' or a {:?} - '{}' symbol.",
                        CompilerSigil::TokenEmbed,
                        CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                        CompilerSigil::TokenStart,
                        CompilerSigil::TokenStart.get_str("ch").unwrap(),
                        CompilerSigil::TokenEmbed,
                        CompilerSigil::TokenEmbed.get_str("ch").unwrap(),
                    )),
                    activity
                }.into())
                // return Err(Error{
                //     kind: ErrorKind::IllegalSymbol,
                //     message: format!(
                //         "Expected a {:?} symbol {:?} or {:?} symbol {:?} after {:?}",
                //             CompilerSigil::TokenStart,
                //             CompilerSigil::TokenStart.get_str("ch"),
                //             CompilerSigil::TokenEmbed,
                //             CompilerSigil::TokenEmbed.get_str("ch"),
                //             CompilerSigil::TokenEmbed.get_str("ch")
                //     )
                // })
            }
            CompilerTokenizerState::SigilFound => {
                return Err(Error::InvalidToken { 
                    src: metadata.named_source.clone(), 
                    span: vec![LabeledSpan::new_primary_with_span(
                        Some(format!(
                             "'{:?}' symbol '{:?}' found with no body to go along side it.", 
                            CompilerSigil::TokenStart, CompilerSigil::TokenStart.get_str("ch")
                        )),
                        span.start + s.len()..std::cmp::min(span.end, s.len()+1)
                    )], 
                    backtrace: backtrace!(Backtrace::new()), 
                    extra: None,
                    activity
                }.into())
            }
            CompilerTokenizerState::CopyingNamedArgumentRef(_) |
            CompilerTokenizerState::CopyingUnamedArgumentRef(_) |
            CompilerTokenizerState::CopyingSkipLastEmbed(_) |
            CompilerTokenizerState::CopyingSkipLast(_) => {
                return Err(Error::InvalidToken { 
                    src: metadata.named_source.clone(), 
                    span: vec![LabeledSpan::new_primary_with_span(
                        Some(format!(
                            "Unfinished token.", 
                        )),
                        span.start + s.len()..std::cmp::min(span.end, s.len()+1)
                    )], 
                    backtrace: backtrace!(Backtrace::new()), 
                    extra: None,
                    activity
                }.into())
            }
        }

        Ok(parts)

    }

    pub fn tokenize_surface(
        s: &Spanned<String>,
        metadata: &Metadata
    ) -> miette::Result<Vec<CompilerToken>> {

        let mut tokens = Self::tokenize(s, metadata)?;

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