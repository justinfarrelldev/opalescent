use crate::build_system::targets::{TargetTriple, parse_target_triple};

pub(crate) fn resolve_target_from_args(args: &[String]) -> Result<Option<TargetTriple>, i32> {
    let target_str = args
        .iter()
        .position(|a| a == "--target")
        .and_then(|i| args.get(i.saturating_add(1)).map(String::as_str));
    match target_str {
        Some(triple_str) => parse_target_triple(triple_str).map(Some).map_err(|_| {
            eprintln!("error: unknown target triple: {triple_str}. Supported: x86_64-linux, x86_64-pc-windows-msvc, x86_64-pc-windows-gnu, aarch64-darwin, x86_64-apple-darwin");
            1
        }),
        None => Ok(None),
    }
}
