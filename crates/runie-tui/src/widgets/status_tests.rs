#![allow(
    clippy::too_many_lines,
    reason = "snapshot-style footer assertions compare idle and active frames together"
)]
use super::*;
use ratatui::style::Color;

#[test]
fn default_is_ready() {
    assert_eq!(StatusBar::new().current(), &Status::Ready);
}

#[test]
fn theme_is_an_actor_owned_status_projection() {
    let mut bar = StatusBar::new();
    assert_eq!(bar.theme(), ThemeKind::GrokNight);
    bar.set_theme(ThemeKind::GrokDay);
    assert_eq!(bar.theme(), ThemeKind::GrokDay);
}

#[test]
fn renderer_adapter_preserves_status_projection_fields() {
    let mut source = StatusBar::new();
    source.set_theme(ThemeKind::GrokDay);
    source.set(Status::Thinking);
    source.advance_animation();
    let snapshot = source.model_snapshot();
    let adapted = StatusBar::from_model_snapshot(snapshot.clone());
    assert_eq!(adapted.model_snapshot(), snapshot);
}

#[test]
fn status_preserves_every_declared_theme_variant() {
    let variants = [
        ThemeKind::GrokNight,
        ThemeKind::GrokDay,
        ThemeKind::TokyoNight,
        ThemeKind::RosePineMoon,
        ThemeKind::OscuraMidnight,
        ThemeKind::Auto,
    ];
    for theme in variants {
        let mut bar = StatusBar::new();
        bar.set_theme(theme);
        assert_eq!(bar.theme(), theme);
    }
}

#[test]
fn status_footer_and_turn_status_use_selected_theme_tokens() {
    let mut bar = StatusBar::new();
    bar.set_theme(ThemeKind::GrokDay);
    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 1));
    bar.render(Rect::new(0, 0, 80, 1), &mut buffer);
    let enter = buffer.cell((0, 0)).expect("footer cell");
    assert_eq!(
        Some(enter.fg),
        appearance::base_style_for(ThemeKind::GrokDay).fg
    );

    let mut turn_buffer = Buffer::empty(Rect::new(0, 0, 40, 1));
    TurnStatus::new(0)
        .phase(TurnStatusPhase::Thinking)
        .with_theme(ThemeKind::GrokDay)
        .render(Rect::new(0, 0, 40, 1), &mut turn_buffer);
    assert_eq!(
        Some(turn_buffer.cell((2, 0)).expect("spinner cell").fg),
        appearance::accent_style_for(ThemeKind::GrokDay).fg
    );
}

#[test]
fn label_distinct_per_variant() {
    let variants = [
        Status::Ready,
        Status::Loading,
        Status::Thinking,
        Status::Streaming,
        Status::Aborted,
        Status::Error("x".into()),
    ];
    let labels: Vec<_> = variants.iter().map(Status::label).collect();
    let unique: std::collections::HashSet<_> = labels.iter().collect();
    assert_eq!(unique.len(), labels.len());
}

#[test]
fn full_mode_footer_matches_grok_idle_and_active_hints() {
    let mut bar = StatusBar::new();
    assert_idle_footer(&mut bar);
    assert_active_footer(&mut bar);
}

fn assert_idle_footer(bar: &mut StatusBar) {
    let mut buffer = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 1,
    });
    bar.render(
        Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 1,
        },
        &mut buffer,
    );
    let idle: String = (0..80)
        .filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()))
        .collect();
    assert!(idle.contains("Enter:send"));
    assert!(idle.contains("Shift+Tab:mode"));
    assert!(idle.contains("Ctrl+x:shortcuts"));
}

fn assert_active_footer(bar: &mut StatusBar) {
    bar.set(Status::Thinking);
    let mut buffer = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 1,
    });
    bar.render(
        Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 1,
        },
        &mut buffer,
    );
    let active: String = (0..80)
        .filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()))
        .collect();
    assert!(active.contains("Shift+Tab:mode"));
    assert!(active.contains("Esc:cancel"));
    assert!(active.contains("Ctrl+.:shortcuts"));
    assert!(!active.contains("Thinking…"));
}

#[test]
fn active_footer_is_stable_across_animation_frames() {
    let mut bar = StatusBar::new();
    bar.set(Status::Thinking);
    let mut frames = Vec::new();
    for frame in 0..3 {
        bar.set_animation_frame(frame);
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 1,
        });
        bar.render(
            Rect {
                x: 0,
                y: 0,
                width: 80,
                height: 1,
            },
            &mut buffer,
        );
        frames.push(
            (0..80)
                .filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()))
                .collect::<String>(),
        );
    }
    assert_eq!(frames[0], frames[1]);
    assert_eq!(frames[1], frames[2]);
    insta::assert_snapshot!(frames.join("\n"));
}

#[test]
fn turn_status_projects_usage_and_stop_reason_from_event_state() {
    let mut bar = StatusBar::new();
    bar.begin_turn();
    bar.finish_turn(
        Usage {
            total_tokens: 42,
            ..Usage::default()
        },
        StopReason::ToolUse,
    );
    bar.set(Status::Streaming);
    let text = bar.turn_status().expect("active turn status").text();
    assert!(text.contains("⇣42"));
    bar.finish_turn(
        Usage {
            total_tokens: 3_180,
            ..Usage::default()
        },
        StopReason::Stop,
    );
    assert!(bar
        .turn_status()
        .expect("active turn status")
        .text()
        .contains("⇣3.18k"));
    assert!(text.contains("toolUse"));
}

#[test]
fn worked_for_label_uses_owned_deterministic_elapsed_ticks() {
    let mut bar = StatusBar::new();
    bar.begin_turn();
    for _ in 0..22 {
        bar.set(Status::Thinking);
        bar.advance_animation();
    }
    assert_eq!(bar.worked_for_label(), "Worked for 1.1s");
}

#[test]
fn header_meter_projects_event_owned_usage() {
    let mut bar = StatusBar::new();
    assert_eq!(bar.header_meter(), "0 turn / 500K");
    bar.finish_turn(
        Usage {
            total_tokens: 18_000,
            ..Usage::default()
        },
        StopReason::Stop,
    );
    assert_eq!(bar.header_meter(), "18K turn / 500K");
}

#[test]
fn status_messages_are_pure_event_projection_inputs() {
    let mut bar = StatusBar::new();
    bar.apply(StatusMsg::BeginTurn);
    bar.apply(StatusMsg::Set(Status::Thinking));
    bar.apply(StatusMsg::AdvanceAnimation);
    assert_eq!(bar.current(), &Status::Thinking);
    assert_eq!(bar.worked_for_label(), "Worked for 0.0s");
    bar.apply(StatusMsg::FinishTurn(
        Usage {
            total_tokens: 1_200,
            ..Usage::default()
        },
        StopReason::Stop,
    ));
    assert_eq!(bar.header_meter(), "1.2K turn / 500K");
}

#[test]
fn animation_frame_is_deterministic_and_owned_by_status_bar() {
    let mut bar = StatusBar::new();
    assert_eq!(bar.animation_frame(), 0);
    bar.set(Status::Thinking);
    bar.advance_animation();
    assert_eq!(bar.animation_frame(), 1);
    bar.set(Status::Ready);
    bar.advance_animation();
    assert_eq!(bar.animation_frame(), 1);
}

#[test]
fn animation_demand_is_false_for_idle_and_terminal_states() {
    let mut bar = StatusBar::new();
    assert!(!bar.animation_demand());
    bar.set(Status::Thinking);
    assert!(bar.animation_demand());
    bar.set(Status::Ready);
    assert!(!bar.animation_demand());
    bar.set(Status::Error("done".into()));
    assert!(!bar.animation_demand());
}

#[test]
fn active_footer_matches_grok_full_mode_vocabulary_and_spacing() {
    let mut bar = StatusBar::new();
    bar.set(Status::Thinking);
    let mut buffer = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 1,
    });
    bar.render(
        Rect {
            x: 2,
            y: 0,
            width: 76,
            height: 1,
        },
        &mut buffer,
    );
    let row: String = (0..80)
        .map(|x| buffer.cell((x, 0)).expect("footer cell").symbol())
        .collect();
    assert!(row.starts_with("  Shift+Tab:mode  │  Esc:cancel  │  Ctrl+.:shortcuts"));
    assert_eq!(row.chars().count(), 80);
}

#[test]
fn spinner_frame_helpers_match_grok_glyphs() {
    assert_eq!(
        braille_spinner_frames(),
        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"]
    );
    assert_eq!(braille_spinner_fallback(), &["|", "/", "-", "\\"]);
    assert_eq!(dot_spinner_frames(), &["⋅", ":", "⸬", "⁙"]);
    assert_eq!(dot_spinner_fallback(), &[".", ":", "·"]);
}

#[test]
fn loading_status_renders_grok_loading_row() {
    let mut bar = StatusBar::new();
    bar.set(Status::Loading);
    bar.set_animation_frame(0);
    let mut buffer = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 40,
        height: 1,
    });
    bar.render(
        Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 1,
        },
        &mut buffer,
    );
    let row: String = (0..40)
        .filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()))
        .collect();
    assert!(
        row.contains("Loading..."),
        "loading row should show the grok Loading label, got: {row:?}"
    );
    assert!(
        row.starts_with(dot_spinner_frames()[0]),
        "loading row should start with the dot spinner"
    );
}

#[test]
fn turn_status_uses_groks_deterministic_braille_frames() {
    assert_eq!(TurnStatus::new(0).text(), "  ⠋ Starting session… 0.0s");
    assert_eq!(TurnStatus::new(8).text(), "  ⠹ Starting session… 0.0s");
    assert_eq!(TurnStatus::new(3).text(), "  ⠙ Starting session… 0.0s");
    assert!(TurnStatus::new(0)
        .phase(TurnStatusPhase::Waiting)
        .text()
        .contains("Waiting for response…"));
    assert!(TurnStatus::new(0)
        .phase(TurnStatusPhase::Responding)
        .text()
        .contains("Responding…"));
}

#[test]
fn thinking_turn_status_matches_grok_working_marker() {
    assert_eq!(
        TurnStatus::new(0).phase(TurnStatusPhase::Thinking).text(),
        "┃  ◆ Thinking…"
    );
}

#[test]
fn turn_status_holds_frames_at_grok_equivalent_cadence_and_colors_roles() {
    assert_eq!(TurnStatus::new(2).text(), TurnStatus::new(0).text());
    assert_ne!(TurnStatus::new(3).text(), TurnStatus::new(0).text());
    let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 1));
    TurnStatus::new(0).render(Rect::new(0, 0, 40, 1), &mut buffer);
    assert_eq!(
        buffer.cell((2, 0)).expect("spinner").fg,
        Color::Rgb(187, 154, 247)
    );
    assert_eq!(
        buffer.cell((4, 0)).expect("label").fg,
        Color::Rgb(108, 108, 108)
    );
    assert!(!buffer
        .cell((2, 0))
        .expect("spinner")
        .modifier
        .contains(Modifier::DIM));
}
