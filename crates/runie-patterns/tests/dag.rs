//! Black-box tests for the Dag primitive (Phase 3): topological waves + cycle detection.

use runie_patterns::primitives::dag::{CycleError, Dag};

#[test]
fn dag_linear_chain_waves() {
    let mut dag = Dag::new();
    let a = dag.add_node("a".into());
    let b = dag.add_node("b".into());
    let c = dag.add_node("c".into());
    dag.add_edge(b, a); // b waits for a
    dag.add_edge(c, b); // c waits for b

    assert_eq!(dag.node_count(), 3);
    let waves = dag.topological_waves().expect("linear chain has no cycle");
    assert_eq!(waves, vec![vec![a], vec![b], vec![c]]);
}

#[test]
fn dag_diamond_two_roots_join() {
    let mut dag = Dag::new();
    let root_a = dag.add_node("root-a".into());
    let root_b = dag.add_node("root-b".into());
    let mid = dag.add_node("mid".into());
    let join = dag.add_node("join".into());
    dag.add_edge(mid, root_a);
    dag.add_edge(mid, root_b);
    dag.add_edge(join, mid);

    let waves = dag.topological_waves().expect("diamond has no cycle");
    assert_eq!(waves.len(), 3, "roots together, then mid, then join");
    assert_eq!(
        waves[0],
        vec![root_a, root_b],
        "both roots run in the first wave"
    );
    assert_eq!(waves[1], vec![mid]);
    assert_eq!(waves[2], vec![join]);
}

#[test]
fn dag_cycle_is_detected() {
    let mut dag = Dag::new();
    let a = dag.add_node("a".into());
    let b = dag.add_node("b".into());
    dag.add_edge(a, b);
    dag.add_edge(b, a);

    let error = dag.topological_waves().expect_err("cycle must error");
    assert!(
        matches!(error, CycleError(node) if node == a || node == b),
        "error carries a node stuck in the cycle, got {error:?}"
    );
    assert_eq!(
        error.to_string(),
        format!("dependency cycle involving node {}", error.0)
    );
}

#[test]
fn dag_self_loop_is_a_cycle() {
    let mut dag = Dag::new();
    let a = dag.add_node("a".into());
    dag.add_edge(a, a);

    assert_eq!(dag.topological_waves().unwrap_err(), CycleError(a));
}

#[test]
fn dag_empty_yields_empty_waves() {
    let dag = Dag::new();
    assert_eq!(dag.node_count(), 0);
    assert_eq!(
        dag.topological_waves().expect("empty dag has no cycle"),
        Vec::<Vec<usize>>::new()
    );
}

#[test]
fn dag_out_of_range_edges_are_ignored() {
    let mut dag = Dag::new();
    let a = dag.add_node("a".into());
    dag.add_edge(a, 99); // dependency does not exist
    dag.add_edge(99, a); // task does not exist

    assert_eq!(dag.node_count(), 1);
    assert_eq!(dag.topological_waves().expect("no cycle"), vec![vec![a]]);
}
