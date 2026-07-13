mod collector;
mod matching;
mod pattern_parsing;
mod token_matching;

pub use self::matching::{check_forbid_pattern_ast, check_require_pattern_ast};
