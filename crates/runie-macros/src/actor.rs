//! `define_actor!` macro implementation.
//!
//! Generates boilerplate for ractor-based actors with declarative message handling.
//!
//! ## Usage
//!
//! ```ignore
//! define_actor! {
//!     name: MyActor,
//!     msg: MyMsg,
//!     state: MyState,
//!     events: MyEvent,
//!
//!     impl handle(msg, state, bus) {
//!         MyMsg::Variant1 { field } => {
//!             state.value = field.clone();
//!             bus.publish(MyEvent::Updated);
//!         }
//!         MyMsg::Variant2 => {
//!             // Handle Variant2
//!         }
//!     }
//! }
//! ```
//!
//! This generates:
//! - The actor struct with state mutex and bus bridge
//! - Ractor trait implementation with `pre_start` and `handle`
//! - `spawn` function
//! - `apply_to` method on the message enum for synchronous testing

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, Ident, Token};

pub fn expand(input: TokenStream) -> TokenStream {
    let def = syn::parse_macro_input!(input as ActorDef);
    generate_actor(&def)
}

struct ActorDef {
    name: Ident,
    msg_type: Ident,
    state_type: Ident,
    events_type: Ident,
    handlers: Vec<MatchArm>,
}

impl Parse for ActorDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![,]>()?;

        // Parse `msg: Ident`
        let msg_keyword = input.parse::<Ident>()?;
        if msg_keyword != "msg" {
            return Err(syn::Error::new_spanned(msg_keyword, "expected `msg`"));
        }
        input.parse::<Token![:]>()?;
        let msg_type = input.parse()?;
        input.parse::<Token![,]>()?;

        // Parse `state: Ident`
        let state_keyword = input.parse::<Ident>()?;
        if state_keyword != "state" {
            return Err(syn::Error::new_spanned(state_keyword, "expected `state`"));
        }
        input.parse::<Token![:]>()?;
        let state_type = input.parse()?;
        input.parse::<Token![,]>()?;

        // Parse `events: Ident`
        let events_keyword = input.parse::<Ident>()?;
        if events_keyword != "events" {
            return Err(syn::Error::new_spanned(events_keyword, "expected `events`"));
        }
        input.parse::<Token![:]>()?;
        let events_type = input.parse()?;
        input.parse::<Token![,]>()?;

        // Parse `impl handle(...) { ... }`
        input.parse::<Token![impl]>()?;
        let _ = input.parse::<Ident>()?; // "handle"
        let handle_content;
        braced!(handle_content in input);
        let handlers_str = handle_content.to_string();

        // Parse match arms from the handler body
        let handlers = parse_match_arms(&handlers_str);

        Ok(Self {
            name,
            msg_type,
            state_type,
            events_type,
            handlers,
        })
    }
}

struct MatchArm {
    pattern: String,
    body: String,
}

fn parse_match_arms(input: &str) -> Vec<MatchArm> {
    let mut arms = Vec::new();
    let mut current_arm = String::new();
    let mut in_pattern = true;
    let mut brace_depth = 0;

    for ch in input.chars() {
        match ch {
            '{' => {
                if in_pattern {
                    in_pattern = false;
                }
                brace_depth += 1;
                current_arm.push(ch);
            }
            '}' => {
                brace_depth -= 1;
                current_arm.push(ch);
                if brace_depth == 0 && !current_arm.trim().is_empty() {
                    // Parse the arm: extract pattern and body
                    if let Some(eq_pos) = current_arm.find("=>") {
                        let pattern = current_arm[..eq_pos].trim().to_string();
                        let body = current_arm[eq_pos + 2..].trim().to_string();
                        arms.push(MatchArm { pattern, body });
                    }
                    current_arm.clear();
                    in_pattern = true;
                }
            }
            '\n' | ' ' | '\t' if current_arm.is_empty() => {}
            _ => {
                if !in_pattern || ch != '\n' {
                    current_arm.push(ch);
                }
            }
        }
    }

    arms
}

fn generate_actor(def: &ActorDef) -> TokenStream {
    let name = &def.name;
    let msg_type = &def.msg_type;
    let state_type = &def.state_type;
    let events_type = &def.events_type;
    let event_var = syn::Ident::new("events", proc_macro2::Span::call_site());

    // Generate match arms for handle method
    // (handle_arms used in handle method - kept for documentation)
    let _handle_arms: Vec<_> = def.handlers.iter().map(|arm| {
        let pattern = &arm.pattern;
        let body = &arm.body;
        quote! {
            #pattern => {
                #body
            }
        }
    }).collect();

    // Generate apply_to arms for synchronous testing
    let apply_arms: Vec<_> = def.handlers.iter().map(|arm| {
        let pattern = &arm.pattern;
        let body = strip_bus_from_body(&arm.body);
        quote! {
            #pattern => {
                #body
            }
        }
    }).collect();

    quote! {
        // ── Actor struct ────────────────────────────────────────────────────────

        /// Actor struct for #name.
        pub struct #name {
            state: std::sync::Mutex<#state_type>,
            bus_bridge: crate::actors::ractor_adapter::EventBusBridge<#events_type>,
        }

        impl #name {
            /// Spawn the actor on the given event bus.
            pub async fn spawn(
                bus: crate::bus::EventBus<#events_type>,
            ) -> Result<(crate::actors::RactorHandle<#msg_type>, ractor::ActorCell), ractor::SpawnErr> {
                let actor = Self {
                    state: std::sync::Mutex::new(#state_type::default()),
                    bus_bridge: crate::actors::ractor_adapter::EventBusBridge::new(bus.clone()),
                };
                let (handle, _, cell) = crate::actors::ractor_adapter::spawn_ractor(
                    Some(ractor::ActorName::new(stringify!(#name))),
                    actor,
                    bus,
                ).await?;
                Ok((handle, cell))
            }

            /// Access the current state (for testing).
            #[cfg(test)]
            pub fn state(&self) -> #state_type {
                self.state.lock().unwrap().clone()
            }
        }

        #[ractor::async_trait]
        impl ractor::Actor for #name {
            type Msg = #msg_type;
            type State = ();
            type Arguments = crate::bus::EventBus<#events_type>;

            async fn pre_start(
                &self,
                _myself: ractor::ActorRef<Self::Msg>,
                _args: Self::Arguments,
            ) -> Result<Self::State, ractor::ActorProcessingErr> {
                Ok(())
            }

            async fn handle(
                &self,
                _myself: ractor::ActorRef<Self::Msg>,
                msg: Self::Msg,
                _state: &mut Self::State,
            ) -> Result<(), ractor::ActorProcessingErr> {
                let (#event_var, emit) = {
                    let mut state = self.state.lock().unwrap();
                    let result = #msg_type::apply_to(&msg, &mut state, &self.bus_bridge);
                    (result.0, result.1)
                };
                if emit {
                    self.bus_bridge.publish(#event_var);
                }
                Ok(())
            }
        }

        // ── Message apply_to for synchronous testing ─────────────────────────────

        impl #msg_type {
            /// Apply a message to state synchronously (for tests without spawned actor).
            ///
            /// Returns `(event, should_emit)` where:
            /// - `event` is the event to publish if should_emit is true
            /// - `should_emit` indicates whether to broadcast the event
            pub fn apply_to(
                &self,
                state: &mut #state_type,
                bus: &crate::actors::ractor_adapter::EventBusBridge<#events_type>,
            ) -> (#events_type, bool) {
                match self {
                    #(#apply_arms)*
                }
            }
        }
    }.into()
}

/// Strip bus.publish() calls from body since apply_to is sync.
fn strip_bus_from_body(body: &str) -> String {
    let mut result = String::new();

    for line in body.lines() {
        let trimmed = line.trim();

        // Skip bus.publish lines in apply_to
        if trimmed.starts_with("bus.publish") || trimmed.starts_with("self.bus_bridge.publish") {
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result.trim().to_string()
}
