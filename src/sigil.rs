use std::{
    collections::HashMap, 
    process::exit
};

use lazy_static::lazy_static;
use strum::{
    EnumIter, EnumProperty, EnumString, IntoEnumIterator
};

#[derive(EnumProperty, EnumIter, EnumString, Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub enum Sigil {

    Non(char),
    #[strum(props(ch = "$"))]
    TokenStart,

    #[strum(props(ch = "{"))]
    PreprocessorKeyRefOpen,
    #[strum(props(ch = "}"))]
    PreprocessorKeyRefClose,

    #[strum(props(ch = "["))]
    CompilerSkipLastOpen,
    #[strum(props(ch = "]"))]
    CompilerSkipLastClose,

    #[strum(props(ch = "("))]
    CompilerArgumentRefOpen,
    #[strum(props(ch = ")"))]
    CompilerArgumentRefClose,

}

lazy_static! {
    static ref SIGIL_CONVERSION_TABLE: HashMap<char, Sigil> = {
        let mut table: HashMap<char, Sigil> = HashMap::new();
        for sigil in Sigil::iter() {
            let Some(s) = sigil.get_str("ch") else {
                continue;
            };
            if s.len() != 1 {
                eprintln!("SIGIL_CONVERSION_TABLE: property 'ch' had a string with .len() != 1");
                exit(1);
            }
            let ch = s.chars().nth(0).unwrap();
            if let Some(existing) = table.get(&ch) {
                eprintln!(
                    "SIGIL_CONVERSION_TABLE: duplicate entry for '{}': {:?} and {:?}",
                    ch, existing, sigil
                );
                exit(1);
            }
            table.insert(ch, sigil);
        }
        table
    };
}

impl From<char> for Sigil {
    fn from(value: char) -> Self {
        if let Some(sigil) = SIGIL_CONVERSION_TABLE.get(&value) {
            return sigil.to_owned();
        }
        Sigil::Non(value)
    }
}