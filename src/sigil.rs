use std::{
    collections::HashMap, 
    process::exit
};

use lazy_static::lazy_static;
use strum::{
    EnumIter, EnumProperty, EnumString, IntoEnumIterator
};

#[derive(EnumProperty, EnumIter, EnumString, Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub enum PreprocessorSigil {

    Non(char),
    #[strum(props(ch = "@"))]
    TokenStart,
    #[strum(props(ch = "\\"))]
    TokenEmbed,

    #[strum(props(ch = "{"))]
    KeyRefOpen,
    #[strum(props(ch = "}"))]
    KeyRefClose,

}

#[derive(EnumProperty, EnumIter, EnumString, Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub enum CompilerSigil {
    
    Non(char),
    #[strum(props(ch = "$"))]
    TokenStart,
    #[strum(props(ch = "\\"))]
    TokenEmbed,

    #[strum(props(ch = "."))]
    PositionDot,

    #[strum(props(ch = "{"))]
    NamedArgumentRefOpen,
    #[strum(props(ch = "}"))]
    NamedArgumentRefClose,

    #[strum(props(ch = "("))]
    UnamedArgumentRefOpen,
    #[strum(props(ch = ")"))]
    UnamedArgumentRefClose,

    #[strum(props(ch = "["))]
    SkipLastOpen,
    #[strum(props(ch = "]"))]
    SkipLastClose,

}

lazy_static! {
    static ref PREPROCESSOR_SIGIL_CONVERSION_TABLE: HashMap<char, PreprocessorSigil> = {
        let mut table: HashMap<char, PreprocessorSigil> = HashMap::new();
        for sigil in PreprocessorSigil::iter() {
            let Some(s) = sigil.get_str("ch") else {
                continue;
            };
            if s.len() != 1 {
                eprintln!("PREPROCESSOR_SIGIL_CONVERSION_TABLE: property 'ch' had a string with .len() != 1");
                exit(1);
            }
            let ch = s.chars().nth(0).unwrap();
            if let Some(existing) = table.get(&ch) {
                eprintln!(
                    "PREPROCESSOR_SIGIL_CONVERSION_TABLE: duplicate entry for '{}': {:?} and {:?}",
                    ch, existing, sigil
                );
                exit(1);
            }
            table.insert(ch, sigil);
        }
        table
    };

    static ref COMPILER_SIGIL_CONVERSION_TABLE: HashMap<char, CompilerSigil> = {
        let mut table: HashMap<char, CompilerSigil> = HashMap::new();
        for sigil in CompilerSigil::iter() {
            let Some(s) = sigil.get_str("ch") else {
                continue;
            };
            if s.len() != 1 {
                eprintln!("COMPILER_SIGIL_CONVERSION_TABLE: property 'ch' had a string with .len() != 1");
                exit(1);
            }
            let ch = s.chars().nth(0).unwrap();
            if let Some(existing) = table.get(&ch) {
                eprintln!(
                    "COMPILER_SIGIL_CONVERSION_TABLE: duplicate entry for '{}': {:?} and {:?}",
                    ch, existing, sigil
                );
                exit(1);
            }
            table.insert(ch, sigil);
        }
        table
    };
}

impl From<char> for PreprocessorSigil {
    fn from(value: char) -> Self {
        if let Some(sigil) = PREPROCESSOR_SIGIL_CONVERSION_TABLE.get(&value) {
            return sigil.to_owned();
        }
        PreprocessorSigil::Non(value)
    }
}

impl From<char> for CompilerSigil {
    fn from(value: char) -> Self {
        if let Some(sigil) = COMPILER_SIGIL_CONVERSION_TABLE.get(&value) {
            return sigil.to_owned();
        }
        CompilerSigil::Non(value)
    }
}