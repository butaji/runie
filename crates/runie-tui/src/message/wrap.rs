//! Word-wrapping helpers for message rendering.

pub(super) fn word_wrap(text: &str, first_width: u16, rest_width: u16) -> Vec<String> {
    let mut state = WrapState::new(first_width, rest_width);
    for word in text.split_whitespace() {
        state.consume_word(word);
    }
    state.finish(text)
}

fn push_flush(result: &mut Vec<String>, current: &mut String, width: &mut u16, max: u16) {
    if !current.is_empty() {
        result.push(std::mem::take(current));
        *width = 0;
    }
}

fn force_split_word(
    word: &str,
    max: u16,
    result: &mut Vec<String>,
    current: &mut String,
    width: &mut u16,
    _rest_width: u16,
) {
    let mut chars = word.chars().peekable();
    while chars.peek().is_some() {
        if *width >= max {
            push_flush(result, current, width, max);
        }
        current.push(chars.next().unwrap());
        *width += 1;
    }
}

struct WrapState {
    result: Vec<String>,
    current: String,
    width: u16,
    max: u16,
}

impl WrapState {
    fn new(first_width: u16, _rest_width: u16) -> Self {
        Self {
            result: Vec::new(),
            current: String::new(),
            width: 0,
            max: first_width.max(1),
        }
    }

    fn consume_word(&mut self, word: &str) {
        let w = word.chars().count() as u16;
        let need_space = !self.current.is_empty();
        if need_space && self.width + 1 + w > self.max {
            push_flush(&mut self.result, &mut self.current, &mut self.width, self.max);
        }
        if !need_space && w > self.max {
            force_split_word(
                word,
                self.max,
                &mut self.result,
                &mut self.current,
                &mut self.width,
                self.max,
            );
            return;
        }
        if need_space {
            self.current.push(' ');
            self.width += 1;
        }
        self.current.push_str(word);
        self.width += w;
    }

    fn finish(mut self, text: &str) -> Vec<String> {
        if !self.current.is_empty() {
            self.result.push(self.current);
        }
        if self.result.is_empty() && text.is_empty() {
            self.result.push(String::new());
        }
        self.result
    }
}
