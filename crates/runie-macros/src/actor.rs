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

        let msg_type = parse_named_field(input, "msg")?;
        input.parse::<Token![,]>()?;

        let state_type = parse_named_field(input, "state")?;
        input.parse::<Token![,]>()?;

        let events_type = parse_named_field(input, "events")?;
        input.parse::<Token![,]>()?;

        // Parse `impl handle(...) { ... }`
        input.parse::<Token![impl]>()?;
        let _ = input.parse::<Ident>()?;
        let handle_content;
        braced!(handle_content in input);
        let handlers_str = handle_content.to_string();

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

fn parse_named_field(input: ParseStream, keyword: &str) -> syn::Result<Ident> {
    let key = input.parse::<Ident>()?;
    if key != keyword {
        return Err(syn::Error::new_spanned(key, format!("expected `{keyword}`")));
    }
    input.parse::<Token![:]>()?;
    input.parse()
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

    let apply_arms = def.handlers.iter().map(|arm| {
        let pattern = &arm.pattern;
        let body = strip_bus_from_body(&arm.body);
        quote! {
            #pattern => {
                #body
            }
        }
    }).collect::<Vec<_>>();

    let struct_tokens = actor_struct(name, state_type, events_type);
    let impl_tokens = actor_impl(name, msg_type, state_type, events_type, event_var);
    let apply_tokens = apply_impl(msg_type, state_type, events_type, apply_arms);

    quote! {
        #struct_tokens
        #impl_tokens
        #apply_tokens
    }.into()
}

fn actor_struct(name: &Ident, state_type: &Ident, events_type: &Ident) -> proc_macro2::TokenStream {
    quote! {
        /// Actor struct for #name.
        pub struct #name {
            state: std::sync::Mutex<#state_type>,
            bus_bridge: crate::actors::ractor_adapter::EventBusBridge<#events_type>,
        }
    }
}

fn actor_impl(
    name: &Ident,
    msg_type: &Ident,
    state_type: &Ident,
    events_type: &Ident,
    event_var: syn::Ident,
) -> proc_macro2::TokenStream {
    let spawn_tokens = spawn_fn(name, msg_type, state_type, events_type);
    let ractor_tokens = ractor_trait_impl(name, msg_type, events_type, event_var);

    quote! {
        impl #name {
            #spawn_tokens
        }
        #ractor_tokens
    }
}

fn spawn_fn(
    name: &Ident,
    msg_type: &Ident,
    state_type: &Ident,
    events_type: &Ident,
) -> proc_macro2::TokenStream {
    let name_str = format!("{}", name);
    let bridge_type = quote! { crate::actors::ractor_adapter::EventBusBridge<#events_type> };
    let handle_type = quote! { crate::actors::RactorHandle<#msg_type> };

    quote! {
        /// Spawn the actor on the given event bus.
        pub async fn spawn(
            bus: crate::bus::EventBus<#events_type>,
        ) -> Result<(#handle_type, ractor::ActorCell), ractor::SpawnErr> {
            let actor = Self {
                state: std::sync::Mutex::new(#state_type::default()),
                bus_bridge: #bridge_type::new(bus.clone()),
            };
            let (handle, _, cell) = crate::actors::ractor_adapter::spawn_ractor(
                Some(ractor::ActorName::new(#name_str)),
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
}

fn ractor_trait_impl(
    name: &Ident,
    msg_type: &Ident,
    events_type: &Ident,
    event_var: syn::Ident,
) -> proc_macro2::TokenStream {
    quote! {
        #[ractor::async_trait]
        impl ractor::Actor for #name {
            type Msg = #msg_type;
            type State = ();
            type Arguments = crate::bus::EventBus<#events_type>;

            async fn pre_start(&self, _: ractor::ActorRef<Self::Msg>, _: Self::Arguments)
                -> Result<Self::State, ractor::ActorProcessingErr> { Ok(()) }

            async fn handle(&self, _: ractor::ActorRef<Self::Msg>, msg: Self::Msg, _: &mut Self::State)
                -> Result<(), ractor::ActorProcessingErr> {
                let (evt, emit) = { let mut s = self.state.lock().unwrap(); #msg_type::apply_to(&msg, &mut s, &self.bus_bridge) };
                if emit { self.bus_bridge.publish(#event_var(evt)); }
                Ok(())
            }
        }
    }
}

fn apply_impl(
    msg_type: &Ident,
    state_type: &Ident,
    events_type: &Ident,
    apply_arms: Vec<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    quote! {
        impl #msg_type {
            /// Apply a message to state synchronously (for tests without spawned actor).
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
    }
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
