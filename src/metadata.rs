use std::path::PathBuf;

use miette::NamedSource;

pub(crate) const MAX_REPEATS: usize = 1000;

pub struct Metadata {

    pub named_source: NamedSource<String>

}