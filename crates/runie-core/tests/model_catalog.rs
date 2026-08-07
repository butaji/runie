use runie_core::model_catalog::{CycleDirection, ModelCatalog, ScopedModel};
use runie_core::types::Model;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixture {
    available: Vec<Model>,
    scoped: Vec<ScopedFixture>,
    search: SearchFixture,
    cycle: CycleFixture,
}

#[derive(Debug, Deserialize)]
struct ScopedFixture {
    provider: String,
    id: String,
    name: String,
    thinking_level: Option<runie_core::types::ThinkingLevel>,
}

#[derive(Debug, Deserialize)]
struct SearchFixture {
    query: String,
    scoped_only: bool,
    expected_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CycleFixture {
    current: String,
    forward: String,
    backward: String,
}

#[test]
#[allow(clippy::too_many_lines)]
fn yaml_catalog_cases_reduce_without_recompiling_behavior() {
    let fixture: Fixture = serde_yaml::from_str(include_str!("fixtures/model-catalog.yaml"))
        .expect("model catalog fixture");
    let scoped = fixture
        .scoped
        .into_iter()
        .map(|item| ScopedModel {
            model: Model {
                provider: item.provider,
                id: item.id,
                name: item.name,
                ..Model::default()
            },
            thinking_level: item.thinking_level,
        })
        .collect();
    let catalog = ModelCatalog::new(fixture.available, scoped);
    assert_eq!(
        catalog
            .search(&fixture.search.query, fixture.search.scoped_only)
            .into_iter()
            .map(|model| model.id)
            .collect::<Vec<_>>(),
        fixture.search.expected_ids
    );
    let current = catalog
        .available
        .iter()
        .find(|model| model.id == fixture.cycle.current)
        .expect("fixture current model");
    assert_eq!(
        catalog
            .cycle(Some(current), CycleDirection::Forward)
            .expect("forward cycle")
            .model
            .id,
        fixture.cycle.forward
    );
    assert_eq!(
        catalog
            .cycle(Some(current), CycleDirection::Backward)
            .expect("backward cycle")
            .model
            .id,
        fixture.cycle.backward
    );
}
