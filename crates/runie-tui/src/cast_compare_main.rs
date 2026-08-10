use super::*;

#[path = "bin/cast_compare_support/modes.rs"]
mod modes;

pub fn run() -> Result<()> {
    let (left, right, frames, dump, phase_marker) = parse_args()?;
    validate_capture_metadata(Path::new(&left), Path::new(&right))?;
    if frames {
        return modes::run_frames(Path::new(&left), Path::new(&right), phase_marker.as_deref());
    }
    modes::run_cells(&left, &right, dump)
}

fn parse_args() -> Result<(String, String, bool, bool, Option<String>)> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let phase_marker = args
        .first()
        .and_then(|arg| arg.strip_prefix("--frames-after="))
        .map(str::to_owned);
    let frames = phase_marker.is_some() || args.first().is_some_and(|arg| arg == "--frames");
    let dump = args.first().is_some_and(|arg| arg == "--dump");
    if frames || dump {
        args.remove(0);
    }
    let mut args = args.into_iter();
    let left = args
        .next()
        .context("usage: cast_compare LEFT.cast RIGHT.cast")?;
    let right = args
        .next()
        .context("usage: cast_compare LEFT.cast RIGHT.cast")?;
    if args.next().is_some() {
        bail!("usage: cast_compare LEFT.cast RIGHT.cast");
    }
    Ok((left, right, frames, dump, phase_marker))
}
