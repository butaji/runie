impl FeedState {
    pub fn reduce(&mut self, message: ScrollbackMsg) {
        macro_rules! reduce_stage {
            ($state:expr, $message:expr, $($stage:ident),+ $(,)?) => {{
                let mut message = $message;
                $(
                    message = match $state.$stage(message) {
                        Ok(()) => return,
                        Err(message) => message,
                    };
                )+
                $state.reduce_navigation(message);
            }};
        }

        reduce_stage!(self, message, reduce_lifecycle, reduce_content);
    }
}
