/// Partition model catalog items into recent models and provider groups.
#[allow(clippy::type_complexity)]
pub fn partition_model_items(
    items: Vec<(String, String, String, bool, bool)>,
) -> (Vec<String>, Vec<(String, Vec<(String, crate::Event)>)>) {
    let mut recent: Vec<String> = Vec::new();
    let mut groups: Vec<(String, Vec<(String, crate::Event)>)> = Vec::new();
    let mut last_header = String::new();
    let mut current_group: Vec<(String, crate::Event)> = Vec::new();
    for (header, name, _cost, _is_selected, _is_current) in items {
        if header == "Recent" {
            recent.push(name);
            continue;
        }
        if !header.is_empty() && header != last_header {
            if !current_group.is_empty() {
                groups.push((last_header.clone(), std::mem::take(&mut current_group)));
            }
            last_header = header.clone();
        }
        if let Some((provider, model)) = name.split_once('/') {
            let evt = crate::Event::SwitchModel {
                provider: provider.to_string(),
                model: model.to_string(),
                explicit: true,
            };
            current_group.push((name, evt));
        }
    }
    if !current_group.is_empty() {
        groups.push((last_header, current_group));
    }
    (recent, groups)
}
