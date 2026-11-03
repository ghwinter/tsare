use super::registry;
use super::*;

pub(crate) fn usage() {
    println!(
        "tsare {} — thought structure-assisted reasoning engine (document format {})",
        env!("CARGO_PKG_VERSION"),
        crate::syntax::FORMAT_VERSION
    );
    println!();
    println!("usage: tsare [COMMAND] PATH [options]");
    println!("       tsare kinds | mem | --explain CODE | --version | --help");
    println!();
    println!("PATH is a .tsare document, a directory of them, or - for standard input.");
    for group in registry::GROUPS {
        println!();
        println!("{group}");
        for command in registry::COMMANDS
            .iter()
            .filter(|command| command.group == *group)
        {
            println!("  {}", command.usage);
        }
    }
    println!();
    println!("exit codes");
    println!("  0  no errors (warnings are allowed)");
    println!("  1  errors found, or a proposal refused");
    println!("  2  usage problem");
}

pub(crate) fn is_command(name: &str) -> bool {
    registry::find(name).is_some() || registry::HIDDEN.contains(&name)
}

/// A bare path is shorthand for `check PATH`.  A misspelled command is not a path: guessing turns
/// `tsare shap doc.tsare` into "shap: the system cannot find the file", which reads as the tool
/// being broken instead of the invocation being wrong, and would silently check a file that
/// happens to carry the typo's name.
pub(crate) fn looks_like_a_document(word: &str) -> bool {
    word == "-" || word.ends_with(".tsare") || Path::new(word).exists()
}
