use super::{PromptActor, UiActor, UiMsg, UiState};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use runie_core::events::EventBus;
use runie_core::types::AgentEvent;

#[test]
fn ui_reducer_owns_welcome_and_shortcut_transitions() {
    let initial = UiState::with_welcome();
    assert!(initial.show_welcome);
    assert!(!initial.shortcuts_open);
    let open = initial.clone().update(UiMsg::ToggleShortcuts);
    assert!(open.shortcuts_open);
    let palette = initial.update(UiMsg::ToggleCommandPalette);
    assert!(palette.command_palette_open);
    let activated = palette
        .update(UiMsg::CommandPaletteChar('n'))
        .update(UiMsg::ActivateCommandPalette);
    assert_eq!(
        activated.last_palette_command.as_deref(),
        Some("New Session")
    );
    assert!(!activated.command_palette_open);
    let hidden = open.update(UiMsg::HideWelcome);
    assert!(!hidden.show_welcome);
    assert!(hidden.shortcuts_open);
    assert_eq!(hidden.update(UiMsg::Reset), UiState::new());
}

include!("app_shared_tests.rs");

#[test]
fn view_document_preserves_declarative_composition_and_ownership() {
    let model = super::TuiSnapshot {
        ui: UiState::new(),
        feed: runie_tui_model::FeedState::default().snapshot(),
        prompt: super::PromptWidget::new().model_snapshot(),
        status: super::StatusBar::new().model_snapshot(),
    };
    let document = super::App::view_document_from_model(&model);
    assert_eq!(document.root.slots().count(), 5);
    assert_eq!(
        document.components.len(),
        crate::view::CHAT_COMPONENTS.len()
    );
    assert_eq!(
        crate::view::component(crate::view::Slot::Scrollback).owner,
        crate::view::StateOwner::ScrollbackActor
    );
}

#[tokio::test]
async fn ui_actor_keeps_welcome_disabled_after_reset() {
    let bus = EventBus::new();
    let actor = UiActor::new(&bus);
    assert!(!actor.snapshot().show_welcome);
    bus.publish(AgentEvent::Reset);
    for _ in 0..4 {
        tokio::task::yield_now().await;
        if !actor.snapshot().show_welcome {
            return;
        }
    }
    panic!("UiActor enabled the removed welcome surface");
}

#[tokio::test]
async fn ui_actor_publishes_palette_activation_command() {
    let bus = EventBus::new();
    let actor = UiActor::new(&bus);
    let mut commands = actor.subscribe_commands();
    actor.send(UiMsg::ToggleCommandPalette).await;
    actor.send(UiMsg::CommandPaletteChar('n')).await;
    actor.send(UiMsg::ActivateCommandPalette).await;
    assert_eq!(
        commands.recv().await.unwrap(),
        super::UiCommand::ActivatePaletteEntry(super::PaletteAction::NewSession)
    );
}

#[tokio::test]
async fn ui_actor_publishes_copy_payload_command() {
    let bus = EventBus::new();
    let actor = UiActor::new(&bus);
    let mut commands = actor.subscribe_commands();
    actor.send(UiMsg::CopyText("latest answer".into())).await;
    assert_eq!(
        commands.recv().await.unwrap(),
        super::UiCommand::CopyText("latest answer".into())
    );
}

#[test]
fn every_palette_command_projects_to_an_executable_flow() {
    for action in super::PaletteAction::labels() {
        let mut state = UiState::new().update(UiMsg::ToggleCommandPalette);
        for ch in action.chars() {
            state = state.update(UiMsg::CommandPaletteChar(ch));
        }
        let command =
            super::app_projection::palette_command_for(&state, &UiMsg::ActivateCommandPalette)
                .unwrap_or_else(|| panic!("no projected flow for palette command {action}"));
        let typed = super::PaletteAction::from_label(action).unwrap();
        if typed.requires_parameters() {
            assert_eq!(
                command,
                super::UiCommand::OpenPaletteParameters(typed),
                "{action} must open its parameter form"
            );
        } else {
            assert_eq!(
                command,
                super::UiCommand::ActivatePaletteEntry(typed),
                "{action} must emit an immediate activation"
            );
        }
    }
}

#[tokio::test]
async fn prompt_actor_reacts_to_reset_events() {
    let bus = EventBus::new();
    let actor = PromptActor::new(&bus);
    actor
        .handle_key(KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
        .await;
    assert!(!actor.snapshot().is_empty());
    bus.publish(AgentEvent::Reset);
    for _ in 0..4 {
        tokio::task::yield_now().await;
        if actor.snapshot().is_empty() {
            return;
        }
    }
    panic!("PromptActor did not apply the bus reset event");
}

#[tokio::test]
async fn prompt_actor_projects_theme_events_into_prompt_view() {
    let bus = EventBus::new();
    let actor = PromptActor::new(&bus);
    bus.publish(AgentEvent::ThemeChanged {
        theme: runie_core::types::ThemeKind::GrokDay,
    });
    for _ in 0..4 {
        tokio::task::yield_now().await;
        let mut buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 3,
        });
        ratatui::widgets::Widget::render(
            actor.snapshot(),
            ratatui::layout::Rect {
                x: 0,
                y: 0,
                width: 30,
                height: 3,
            },
            &mut buffer,
        );
        if buffer
            .cell((2, 1))
            .is_some_and(|cell| cell.fg == ratatui::style::Color::Rgb(38, 38, 38))
        {
            return;
        }
    }
    panic!("PromptActor did not project the theme event");
}

#[tokio::test]
async fn prompt_actor_projects_terminal_native_theme_into_reset_colors() {
    let bus = EventBus::new();
    let actor = PromptActor::new(&bus);
    bus.publish(AgentEvent::ThemeChanged {
        theme: runie_core::types::ThemeKind::TerminalNative,
    });
    for _ in 0..4 {
        tokio::task::yield_now().await;
        let mut buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 3,
        });
        ratatui::widgets::Widget::render(
            actor.snapshot(),
            ratatui::layout::Rect {
                x: 0,
                y: 0,
                width: 30,
                height: 3,
            },
            &mut buffer,
        );
        if buffer
            .cell((2, 1))
            .is_some_and(|cell| cell.fg == ratatui::style::Color::Reset)
        {
            return;
        }
    }
    panic!("PromptActor did not project terminal-native theme");
}

#[tokio::test]
async fn prompt_reset_preserves_actor_owned_theme() {
    let bus = EventBus::new();
    let actor = PromptActor::new(&bus);
    bus.publish(AgentEvent::ThemeChanged {
        theme: runie_core::types::ThemeKind::RosePineMoon,
    });
    for _ in 0..4 {
        tokio::task::yield_now().await;
        if actor.model_snapshot().theme == runie_core::types::ThemeKind::RosePineMoon {
            break;
        }
    }
    bus.publish(AgentEvent::Reset);
    for _ in 0..4 {
        tokio::task::yield_now().await;
        if actor.model_snapshot().theme == runie_core::types::ThemeKind::RosePineMoon {
            return;
        }
    }
    panic!("PromptActor reset discarded the actor-owned theme");
}

#[tokio::test]
async fn prompt_reset_preserves_actor_owned_model_caption() {
    let bus = EventBus::new();
    let actor = PromptActor::new(&bus);
    actor.set_model_caption("custom-model (high)".into()).await;
    bus.publish(AgentEvent::Reset);
    for _ in 0..4 {
        tokio::task::yield_now().await;
        if actor.model_snapshot().model_caption == "custom-model (high)" {
            return;
        }
    }
    panic!("PromptActor reset discarded the actor-owned model caption");
}

/// Regression test for the `biased;` select! in `run_prompt_actor`:
/// when keys are already queued in the mailbox and a flood of broadcast
/// events is also ready, the actor must acknowledge every queued key
/// before draining the event queue. We drive `run_prompt_actor`
/// directly so we can observe the broadcast receiver's buffered depth
/// from outside the actor task, and we use a `Notify` pause to read
/// the buffered count deterministically rather than racing the
/// wake-up against the actor's event drain loop.
///
/// Eight observation points rather than one is deliberate: a single
/// message turns the bias into a coin flip because the unbiased
/// `select!` picks the mailbox branch by chance about half the time,
/// and a measured 25-run sweep of the deleted-`biased;` build caught
/// the regression only 15/25 times; the eight-point form caught it
/// 25/25 while staying deterministic (10/10) with the fix in place.
#[tokio::test(flavor = "current_thread")]
#[allow(
    clippy::too_many_lines,
    reason = "deterministic test wires up bus + mailbox + actor + pause hooks in one place"
)]
async fn prompt_actor_services_key_mailbox_before_draining_queued_events() {
    use std::sync::Arc;
    use tokio::sync::{mpsc, watch, Notify};
    let event_rx = queued_agent_events();
    let (mailbox_tx, mailbox_rx) = mpsc::channel::<super::PromptMsg>(8);
    let (snapshot_tx, snapshot_rx) = watch::channel(super::PromptWidget::new().model_snapshot());
    let (shared_tx, _shared_rx) = watch::channel(runie_core::SharedSnapshot::new(
        super::PromptWidget::new().model_snapshot(),
    ));
    let event_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_for_worker = event_counter.clone();
    let key_done = Arc::new(Notify::new());
    let actor_release = Arc::new(Notify::new());
    let key_done_for_worker = key_done.clone();
    let actor_release_for_worker = actor_release.clone();
    // OWNER: test
    let actor_task = tokio::spawn(async move {
        // OWNER: test
        super::run_prompt_actor(
            mailbox_rx,
            event_rx,
            snapshot_tx,
            shared_tx,
            counter_for_worker,
            Some((key_done_for_worker, actor_release_for_worker)),
        )
        .await;
    });
    let replies = enqueue_prompt_keys(&mailbox_tx, 8);
    finish_prompt_mailbox_test(
        key_done,
        actor_release,
        event_counter,
        snapshot_rx,
        replies,
        mailbox_tx,
        actor_task,
    )
    .await;
}

async fn finish_prompt_mailbox_test(
    key_done: std::sync::Arc<tokio::sync::Notify>,
    actor_release: std::sync::Arc<tokio::sync::Notify>,
    event_counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    snapshot_rx: tokio::sync::watch::Receiver<super::PromptSnapshot>,
    replies: Vec<tokio::sync::oneshot::Receiver<super::PromptOutcome>>,
    mailbox_tx: tokio::sync::mpsc::Sender<super::PromptMsg>,
    actor_task: tokio::task::JoinHandle<()>,
) {
    for reduced in 1..=8 {
        key_done.notified().await;
        assert_eq!(event_counter.load(std::sync::atomic::Ordering::SeqCst), 0);
        if reduced == 1 {
            assert_eq!(snapshot_rx.borrow().text, "x");
        }
        actor_release.notify_one();
    }
    for reply_rx in replies {
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), reply_rx)
            .await
            .expect("key reply should fire promptly")
            .expect("key reply should resolve Ok");
        assert_eq!(outcome, super::PromptOutcome::Edited);
    }
    drop(mailbox_tx);
    actor_task.await.expect("actor task should join cleanly");
}

fn enqueue_prompt_keys(
    mailbox_tx: &tokio::sync::mpsc::Sender<super::PromptMsg>,
    count: usize,
) -> Vec<tokio::sync::oneshot::Receiver<super::PromptOutcome>> {
    let mut replies = Vec::with_capacity(count);
    for i in 0..count {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let key = crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char((b'x' + (i % 3) as u8) as char),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        mailbox_tx
            .try_send(super::PromptMsg::Key(key, reply_tx))
            .expect("key enqueue should not fail");
        replies.push(reply_rx);
    }
    replies
}

fn queued_agent_events() -> tokio::sync::broadcast::Receiver<AgentEvent> {
    let (event_tx, event_rx) = tokio::sync::broadcast::channel::<AgentEvent>(65_536);
    for _ in 0..16_384 {
        event_tx.send(AgentEvent::AgentStart).expect("send");
    }
    assert_eq!(event_rx.len(), 16_384);
    event_rx
}

/// Regression test for the `biased;` select! in `run_ui_actor`: when a
/// `UiMsg` is already queued in the mailbox and a flood of broadcast
/// events is also ready, the actor must reduce the UI message before
/// draining the event queue. We drive `run_ui_actor` directly so we can
/// observe the drained-event counter from outside the actor task, and we
/// use a `Notify` pause to read it deterministically rather than racing
/// the wake-up against the actor's event drain loop. A single message
/// would only catch the unbiased select! about 60% of the time (the
/// random branch order picks the mailbox first half the time), so the
/// test observes `MESSAGES` consecutive pause points instead.
#[tokio::test(flavor = "current_thread")]
#[allow(
    clippy::too_many_lines,
    reason = "deterministic test wires up bus + mailbox + actor + pause hooks in one place"
)]
async fn ui_actor_services_mailbox_before_draining_queued_events() {
    use std::sync::Arc;
    use tokio::sync::{broadcast, mpsc, watch, Notify};

    let event_rx = queued_ui_events();
    const MESSAGES: usize = 8;
    let (mailbox_tx, mailbox_rx) = mpsc::channel::<super::UiMailbox>(MESSAGES);
    let (snapshot_tx, snapshot_rx) = watch::channel(UiState::new());
    let (command_tx, _command_rx) = broadcast::channel::<super::UiCommand>(32);

    let event_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_for_worker = event_counter.clone();
    let message_done = Arc::new(Notify::new());
    let actor_release = Arc::new(Notify::new());
    let message_done_for_worker = message_done.clone();
    let actor_release_for_worker = actor_release.clone();

    let actor_task = spawn_ui_test_actor(
        mailbox_rx,
        event_rx,
        snapshot_tx,
        command_tx,
        counter_for_worker,
        message_done_for_worker,
        actor_release_for_worker,
    );

    let replies = enqueue_ui_messages(&mailbox_tx, MESSAGES);

    finish_ui_mailbox_test(
        message_done,
        actor_release,
        event_counter,
        snapshot_rx,
        replies,
        mailbox_tx,
        actor_task,
    )
    .await;
}

async fn finish_ui_mailbox_test(
    message_done: std::sync::Arc<tokio::sync::Notify>,
    actor_release: std::sync::Arc<tokio::sync::Notify>,
    event_counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    snapshot_rx: tokio::sync::watch::Receiver<UiState>,
    replies: Vec<tokio::sync::oneshot::Receiver<()>>,
    mailbox_tx: tokio::sync::mpsc::Sender<(UiMsg, tokio::sync::oneshot::Sender<()>)>,
    actor_task: tokio::task::JoinHandle<()>,
) {
    for reduced in 1..=8 {
        message_done.notified().await;
        assert_eq!(event_counter.load(std::sync::atomic::Ordering::SeqCst), 0);
        if reduced == 1 {
            assert!(snapshot_rx.borrow().command_palette_open);
        }
        actor_release.notify_one();
    }
    for reply_rx in replies {
        tokio::time::timeout(std::time::Duration::from_secs(2), reply_rx)
            .await
            .expect("ui message ack should fire promptly")
            .expect("ui message ack should resolve Ok");
    }
    drop(mailbox_tx);
    actor_task.await.expect("actor task should join cleanly");
}

fn queued_ui_events() -> tokio::sync::broadcast::Receiver<AgentEvent> {
    let (event_tx, event_rx) = tokio::sync::broadcast::channel(65_536);
    for _ in 0..16_384 {
        event_tx.send(AgentEvent::AgentStart).expect("send");
    }
    assert_eq!(event_rx.len(), 16_384);
    event_rx
}

fn enqueue_ui_messages(
    mailbox_tx: &tokio::sync::mpsc::Sender<super::UiMailbox>,
    count: usize,
) -> Vec<tokio::sync::oneshot::Receiver<()>> {
    let mut replies = Vec::with_capacity(count);
    for _ in 0..count {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        mailbox_tx
            .try_send((UiMsg::ToggleCommandPalette, reply_tx))
            .expect("ui message enqueue should not fail");
        replies.push(reply_rx);
    }
    replies
}

fn spawn_ui_test_actor(
    mailbox_rx: tokio::sync::mpsc::Receiver<super::UiMailbox>,
    event_rx: tokio::sync::broadcast::Receiver<AgentEvent>,
    snapshot_tx: tokio::sync::watch::Sender<UiState>,
    command_tx: tokio::sync::broadcast::Sender<super::UiCommand>,
    counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    done: std::sync::Arc<tokio::sync::Notify>,
    release: std::sync::Arc<tokio::sync::Notify>,
) -> tokio::task::JoinHandle<()> {
    // OWNER: test
    tokio::spawn(async move {
        super::run_ui_actor(
            mailbox_rx,
            event_rx,
            snapshot_tx,
            command_tx,
            UiState::new(),
            counter,
            Some((done, release)),
        )
        .await;
    })
}
