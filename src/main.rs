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

mod args;
mod config;
mod sigil;
mod preprocessor;

use clap::Parser;
use config::Config;
use args::Arguments;

fn main() {

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

    let config = match Config::load(args.input.as_path()) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}"); 
            panic!()
        }
    };

    log::info!("Loaded config.");


    match config.preprocess() {
        Ok(_) => (),
        Err(err) => {
            eprintln!("{err}"); 
            panic!()
        }
    }

    log::info!("Finished preprocessing.")

    
}
