#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageChartMetric {
    All,
    Input,
    Output,
    Cost,
}

pub fn parse_usage_chart(args: &str) -> Option<UsageChartMetric> {
    let parts = args
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    match parts
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["chart"] => Some(UsageChartMetric::All),
        ["chart", "input"] => Some(UsageChartMetric::Input),
        ["chart", "output"] => Some(UsageChartMetric::Output),
        ["chart", "cost"] => Some(UsageChartMetric::Cost),
        _ => None,
    }
}
