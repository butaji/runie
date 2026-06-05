use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub scroll: usize,
    pub streaming: bool,
    pub stream_buffer: String,
    pub build_time: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: vec![ChatMessage {
                role: "system".into(),
                content: "Welcome! Type a message and press Enter.".into(),
            }],
            input: String::new(),
            scroll: 0,
            streaming: false,
            stream_buffer: String::new(),
            build_time: String::new(),
        }
    }
}

impl AppState {
    pub fn push_user_message(&mut self) {
        if !self.input.is_empty() {
            self.messages.push(ChatMessage {
                role: "user".into(),
                content: self.input.clone(),
            });
            self.input.clear();
            self.scroll = self.messages.len().saturating_sub(1);
        }
    }

    pub fn handle_agent_event(&mut self, event: &str) {
        // Parse and handle agent events (simplified for now)
        if event.starts_with("START") {
            self.streaming = true;
            self.messages.push(ChatMessage {
                role: "assistant".into(),
                content: String::new(),
            });
        } else if event.starts_with("TEXT:") {
            if let Some(last) = self.messages.last_mut() {
                last.content.push_str(&event[5..]);
            }
        } else if event == "END" {
            self.streaming = false;
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn deserialize(data: &[u8]) -> Self {
        serde_json::from_slice(data).unwrap_or_default()
    }
}

// Functions exported for the host

#[no_mangle]
pub extern "C" fn app_init() -> *mut AppState {
    Box::into_raw(Box::new(AppState::default()))
}

#[no_mangle]
pub extern "C" fn app_serialize(state: *mut AppState) -> *mut Vec<u8> {
    if state.is_null() {
        return Box::into_raw(Box::new(Vec::new()));
    }
    let state = unsafe { &*state };
    Box::into_raw(Box::new(state.serialize()))
}

#[no_mangle]
pub extern "C" fn app_deserialize(data: *mut Vec<u8>) -> *mut AppState {
    if data.is_null() {
        return Box::into_raw(Box::new(AppState::default()));
    }
    let data = unsafe { Box::from_raw(data) };
    Box::into_raw(Box::new(AppState::deserialize(&data)))
}

#[no_mangle]
pub unsafe fn app_free_state(state: *mut AppState) {
    if !state.is_null() {
        let _ = Box::from_raw(state);
    }
}

#[no_mangle]
pub unsafe fn app_free_bytes(bytes: *mut Vec<u8>) {
    if !bytes.is_null() {
        let _ = Box::from_raw(bytes);
    }
}

#[no_mangle]
pub extern "C" fn app_push_message(state: *mut AppState) {
    if !state.is_null() {
        unsafe { &mut *state }.push_user_message();
    }
}

#[no_mangle]
pub extern "C" fn app_handle_event(state: *mut AppState, event: *const libc::c_char) {
    if state.is_null() || event.is_null() {
        return;
    }
    let event_str = unsafe { std::ffi::CStr::from_ptr(event) }
        .to_string_lossy()
        .into_owned();
    unsafe { &mut *state }.handle_agent_event(&event_str);
}

#[no_mangle]
pub extern "C" fn app_input_push(state: *mut AppState, c: libc::c_char) {
    if !state.is_null() {
        unsafe { &mut *state }.input.push(c as u8 as char);
    }
}

#[no_mangle]
pub extern "C" fn app_input_backspace(state: *mut AppState) {
    if !state.is_null() {
        unsafe { &mut *state }.input.pop();
    }
}

#[no_mangle]
pub extern "C" fn app_set_streaming(state: *mut AppState, streaming: bool) {
    if !state.is_null() {
        unsafe { &mut *state }.streaming = streaming;
    }
}
