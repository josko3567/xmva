use std::collections::HashMap;

use colored::Colorize;

use crate::{
    config::{Argument, Config}, 
    preprocessor::{PreprocessableName, PreprocessableString}
};

#[derive(Debug)]
pub enum ErrorKind {
    DuplicateArgument
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    message: String
}

impl std::fmt::Display for Error {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Encountered a error, [{:?}]: {}", self.kind, self.message)
    }

}

impl std::error::Error for Error {}


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

    pub fn compile(
        &self
    ) -> Result<String, Error> {

        log::debug!("Starting to compile the config.");

        log::debug!("Loading named arguments...");
        let named = self.load_named_arguments()?;


        


        unimplemented!()

    }

}