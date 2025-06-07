//! # xmvagen
//! 
//! This is a app that runs on files with "name.xmva.toml" files.
//! As the "xmva" shorthand says, the config configurates a generator
//! for custom X-Macros with Varadict Arguments (xmva).
//! 
//! Googling the term "X-Macros with Varadict Arguments" doesn't give much,
//! because this technique is quite uhh, uncommon.
//! 
//! There are certaint techniques we use such as X-Macros (lol), Varadict macro
//! argument counting/macro argument overloading (idk if thats the proper name) 
//! and varadict macro dispatch.
//! 
//! Googling those terms will give you enough info i suppose to create your own XMVA.
//! But since none has really coined a term for this ill call them XMVAs.
//! 
//! XMVAs are very good for generating code that will both create a structure/enum
//! and functionality for said structure/enum (but mostly enums).
//! 
//! Ill have an example.xmva.toml to show you a use case for this.
//! 
//! And also i need this for my thesis O_O.
//! 
//! This program could have definitely been my thesis since i've seen a
//! thesis thats litteraly a CLI where you set values for a quadratic
//! formula that then calculates the quadratic formula (this program 
//! itself would have been overkill ;_;).
//! 
//! But i've dug my own grave so ill just do my best 👍
//! 
//! TODO:
//! if i got the time, preprocessor.rs and compiler.rs are both pretty full to
//! the brim with functions from all sides.
//! Best to do with this:
//! 
//! preprocessor.rs
//! 
//! into..
//! 
//! preprocessor
//! +--mod.rs // preprocessing 
//! +--token.rs // tokens and tokenizer
//! 
//! and implement a similiar structure to compiler where his tokens have
//! impl of tokenize and detokenize [also implement detokenize mostly for tests].
//! 
//! compiler.rs
//! 
//! into
//! 
//! compiler
//! +--mod.rs // compiling
//! +--token.rs // tokens and tokenizers
//! +--surface.rs // surface compiler
//! +--repeat.rs // special repeat pattern compiler
//! assembler.rs // assembling the compiled code into a file.
//! 
//! also good would be to have a Compilable<> object for surface compilables
//! and Repeat<> object for the repeat pattern.
//! 
//! kinda like
//! 
//! Compilable {
//!     Uncompiled(PreprocessableString)
//!     Compiled(String)
//! }
//! 
//! Repeat {
//!     Ungenerated(Compilable)
//!     Generated(String)
//! }
//! 
//! and instead of doin arc mutex we make a type that implements arc mutex.
//! kinda like...
//! Reflective<> // meaning that we can change it even if we only have a refrence.
//! 
//! this way we can also make types like:
//! 
//! Reflective<Repeat<Compilable<Preprocessable<String>>>>
//! 
//! but instead of writing this monstrosity of verbosity
//! we can instead just expect every type Repeat has to be a Compilable
//! and every Compilable to be a Preprocessable.
//! 
//! Reflective<Repeat<String>>
//! Reflective<Compilable<String>>
//! Reflective<Preprocessable<String>>
//! 
//! then the underlying types are
//! 
//! Repeat<T> {
//!     Ungenerated(Compilable<T>)
//!     Generated(String)
//! }
//! 
//! Compilable<T> {
//!     Uncompiled(Preprocessable<T>)
//!     Compiled(String)
//! }
//! 
//! and instead of writing along with them a monstrostiy of a match statment
//! to get the underlying type especially with Repeat we can just make
//! a function .get_<type>()
//! 
//! so we can do let raw = repeat.get_ungenerated()?.get_uncompiled()?.get_preprocessed()?;
//! 
//! but now the main question is how would you edit these like i do now?
//! 
//! reflective is only reflective on the Repeat so if we take the preprocessed
//! part we cant edit it we have to create a new Repeat::Ungenerated(Compiled::Uncompiled(Preprocessed::Type))
//! 
//! maybe we make all of these types reflective?
//! orr we can simply edit the AnyPreprocessable to handle Repeat and Compiled
//! 
//! repeat and compilable have to implement compiler tokenizer
//! while preprocessables have to implement the preprocessor tokenizer.
//! 
//! errors are very similar across the board so a error.rs would be great
//! aswell and the error messages are pretty ass we can add some [miette]
//! to the recipe and this is all i can think of.
//! 
//! also if we use miette we are gonna need to store where the data is being read
//! from or to be more precise the range of the data so that we can pinpoint accuretly
//! where an error occurs. toml_edit seems to be the best for this.
//! 
//! i have a whole ass thesis that this is just a small part of so this is only
//! if like idk people ask me cuz im not editing this after i finish my thesis
//! (probably???)

mod args;
mod sigil;
mod error;
mod config;
mod preprocessor;
mod compiler;
mod metadata;

mod _config;
mod _compiler;

use std::{env, fs, path::PathBuf};

use clap::Parser;
use args::Arguments;
use _config::Config;

pub fn xmva(
    config: PathBuf, 
    contents: String
) -> miette::Result<()>
{

    todo!()
}

fn main() -> miette::Result<()> {

    let args = Arguments::parse();
    if args.logging {
        env_logger::builder()
            .filter_level(log::LevelFilter::Trace)
            .init();
        log::info!("Logs are enabled.");
    }

    log::info!("Loaded arguments, input file is {:?}", args.input);
    if args.output.is_some() {
        log::info!("Specified a external output file {:?}", args.output.unwrap())
    }   

    


    
    todo!()

}

// fn main() {

//     let args = Arguments::parse();
//     if args.logging {
//         env_logger::builder()
//             .filter_level(log::LevelFilter::Trace)
//             .init();
//         log::info!("Logs are enabled.");
//     }

//     log::info!("Loaded arguments, input file is {:?}", args.input);
//     if args.output.is_some() {
//         log::info!("Specified a external output file {:?}", args.output.unwrap())
//     }    

//     let config = match Config::load(args.input.as_path()) {
//         Ok(config) => {
//             log::info!("Loaded config.");
//             config
//         },
//         Err(err) => {
//             eprintln!("{err}"); 
//             panic!()
//         }
//     };

//     // changing the current pwd so that all relative paths are a okay
//     let output = args.input.as_path();
//     let canon_output = output.canonicalize()
//         .expect("Failed to get absolute path from output file.");
//     let current_dir = canon_output.parent();
//     env::set_current_dir(current_dir.clone().unwrap())
//         .expect(format!(
//             "Failed to change the current PWD to {:?}",
//             current_dir
//         ).as_str());

//     match config.preprocess() {
//         Ok(_) => log::info!("Finished preprocessing."),
//         Err(err) => {
//             eprintln!("{err}"); 
//             panic!()
//         }
//     }

//     let output = match config.compile_and_assemble() {
//         Ok(output) => {
//             log::info!("Finished compiling and assembling.");
//             output
//         },
//         Err(err) => {
//             eprintln!("{err}");
//             panic!()
//         }
//     };

//     let output_path = &config.common.output.unwrap();
//     if let Err(e) = fs::write(output_path, &output) {
//         eprintln!("Failed to write output to {}: {e}", output_path.display());
//         panic!();
//     } else {
//         log::info!("Output written to {}", output_path.display());
//     }

// }