
pub mod token;

use backtrace::Backtrace;
use miette::LabeledSpan;
use toml::Spanned;

use crate::backtrace;
use crate::error::Error;
use crate::metadata::Metadata;
use crate::preprocessor::{IntoPreprocessorTokens, Preprocessable};
use crate::compiler::token::CompilerToken;

#[derive(Debug, Clone)]
pub enum Compilable<T>
where T: IntoSurfaceCompilerTokens + IntoPreprocessorTokens
{
    NotCompiled(Preprocessable<T>),
    Compiler(Spanned<String>)
}

trait IntoSurfaceCompilerTokens {

    fn into_surface_compiler_tokens(
        &self,
        metadata: &Metadata
    ) -> miette::Result<Vec<CompilerToken>>;

}

impl IntoSurfaceCompilerTokens for Spanned<String> {

    fn into_surface_compiler_tokens(
        &self,
        metadata: &Metadata
    ) -> miette::Result<Vec<CompilerToken>> {
        
        token::CompilerToken::tokenize_surface(self, metadata)
        
    }

}

impl IntoSurfaceCompilerTokens for Preprocessable<Spanned<String>> {

    fn into_surface_compiler_tokens(
        &self,
        metadata: &Metadata
    ) -> miette::Result<Vec<CompilerToken>> {
        
        match self {
            Preprocessable::NotPreprocessed(spanned_s) => {
                return Err(Error::HigherRecivedUnfinished { 
                    src: metadata.named_source.clone(), 
                    span: vec![LabeledSpan::new_primary_with_span(
                        Some(format!(
                            "This string was not preprocessed."
                        )), 
                        spanned_s.span()
                    )], 
                    backtrace: backtrace!(Backtrace::new()), 
                    extra: None, 
                    activity: "compiling".to_owned()
                }.into())
            }
            Preprocessable::Preprocessed(spanned_s)

        }


    }

}

struct GeneratablePattern {
    pattern: Vec<CompilerToken>,
    macro_prefix: String
}

impl GeneratablePattern {

    pub fn generate(&self) -> String {
        todo!()
    }

}

pub enum Pattern<T>
where T: IntoPatternCompilerTokens +
         IntoSurfaceCompilerTokens +
         IntoPreprocessorTokens
{
    Ungeneratable(Compilable<T>),
    Generatable(GeneratablePattern)
}

trait IntoPatternCompilerTokens {

    fn into_pattern_compiler_tokens(
        &self
    ) -> Result<CompilerToken, Error>;

}



