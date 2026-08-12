#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputWindowDirection {
    Head,
    Tail,
}

pub fn parse_output_window_query(args: &str) -> Option<(&str, OutputWindowDirection, usize)> {
    let mut parts = args.split_whitespace();
    if parts.next() != Some("output") || parts.next().is_none() {
        return None;
    }
    let id = args.split_whitespace().nth(1)?;
    let direction = match parts.next()? {
        "head" => OutputWindowDirection::Head,
        "tail" => OutputWindowDirection::Tail,
        _ => return None,
    };
    let count = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((id, direction, count))
}
pub fn parse_output_tail_query(args: &str) -> Option<(&str, usize)> {
    parse_output_window_query(args)
        .filter(|(_, direction, _)| *direction == OutputWindowDirection::Tail)
        .map(|(id, _, count)| (id, count))
}
