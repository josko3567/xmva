use std::path::PathBuf;
use miette::{Diagnostic, LabeledSpan, NamedSource};

#[macro_export]
macro_rules! backtrace {
    ($trace:expr) => {
        if cfg!(debug_assertions) {
            let printer = color_backtrace::BacktracePrinter::new()
                .lib_verbosity(color_backtrace::Verbosity::Full);
            let str = printer.format_trace_to_string(&$trace).unwrap();
            Some(format!("{}", str))
        } else {None}
    };
}

#[derive(thiserror::Error, Diagnostic, Debug)]
pub enum Error {
    #[error("An IO operation failed!")]
    #[diagnostic(code(xmva::error::io_operation))]
    IO {
        #[help] help: String, 
        #[help] backtrace: Option<String>,
    },
    #[error("Failed to deserialize `{file}`!")]
    #[diagnostic(code(xmva::error::toml_deserialization))]
    TOML {
        #[source_code] src: NamedSource<String>, 
        #[label(collection)] span: Vec<LabeledSpan>,
        #[help] backtrace: Option<String>,
        file: PathBuf
    },
    #[error("Tried to read/write to a poisoned lock!")]
    #[diagnostic(code(xmva::error::toml_deserialization))]
    PoisonedLock {
        #[help] error: String,
        #[help] backtrace: Option<String>,
    },
    #[error("Illegal symbol encountered while {activity}!")]
    #[diagnostic(code(xmva::error::illegal_symbol))]
    IllegalSymbol {
        #[source_code] src: NamedSource<String>,
        #[label(collection)] span: Vec<LabeledSpan>,
        #[help] backtrace: Option<String>,
        #[help] extra: Option<String>,
        activity: String
    },
    #[error("Encountered a empty reference while {activity}!")]
    #[diagnostic(code(xmva::error::empty_reference))]
    EmptyReference {
        #[source_code] src: NamedSource<String>,
        #[label(collection)] span: Vec<LabeledSpan>,
        #[help] backtrace: Option<String>,
        #[help] extra: Option<String>,
        activity: String
    },
    #[error("Encountered a invalid reference while {activity}!")]
    #[diagnostic(code(xmva::error::invalid_reference))]
    InvalidReference {
        #[source_code] src: NamedSource<String>,
        #[label(collection)] span: Vec<LabeledSpan>,
        #[help] backtrace: Option<String>,
        #[help] extra: Option<String>,
        activity: String
    },
    #[error("Encountered a invalid token while {activity}!")]
    #[diagnostic(code(xmva::error::invalid_token))]
    InvalidToken {
        #[source_code] src: NamedSource<String>,
        #[label(collection)] span: Vec<LabeledSpan>,
        #[help] backtrace: Option<String>,
        #[help] extra: Option<String>,
        activity: String
    },
    #[error("Recived a unprocessed lower level string {activity}!")]
    #[diagnostic(code(xmva::error::invalid_token))]
    HigherRecivedUnfinished {
        #[source_code] src: NamedSource<String>,
        #[label(collection)] span: Vec<LabeledSpan>,
        #[help] backtrace: Option<String>,
        #[help] extra: Option<String>,
        activity: String
    },
}