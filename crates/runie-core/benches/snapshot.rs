use criterion::{black_box, criterion_group, criterion_main, Criterion};
use runie_core::{AppState, ChatMessage, Role};

fn bench_snapshot(c: &mut Criterion) {
    c.bench_function("snapshot_100_messages", |b| {
        let mut state = AppState::default();
        for i in 0..100 {
            state.messages.push(ChatMessage {
                role: Role::User,
                content: format!("Message {} with some content to make it realistic", i),
                timestamp: i as f64,
                id: format!("msg{}", i),
                ..Default::default()
            });
        }
        state.messages_changed();
        state.ensure_fresh();
        b.iter(|| {
            let snap = state.snapshot();
            black_box(snap);
        });
    });

    c.bench_function("snapshot_500_messages", |b| {
        let mut state = AppState::default();
        for i in 0..500 {
            state.messages.push(ChatMessage {
                role: Role::User,
                content: format!("Message {} with some content to make it realistic", i),
                timestamp: i as f64,
                id: format!("msg{}", i),
                ..Default::default()
            });
        }
        state.messages_changed();
        state.ensure_fresh();
        b.iter(|| {
            let snap = state.snapshot();
            black_box(snap);
        });
    });
}

criterion_group!(benches, bench_snapshot);
criterion_main!(benches);
