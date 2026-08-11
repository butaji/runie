#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    Http,
}

macro_rules! mcp_transport_wire_names {
    ($(($variant:ident, $wire:literal)),+ $(,)?) => {
        impl McpTransport {
            pub const fn wire_name(self) -> &'static str {
                match self { $(Self::$variant => $wire,)+ }
            }

            pub fn from_wire_name(name: &str) -> Option<Self> {
                match name { $($wire => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

mcp_transport_wire_names! {
    (Stdio, "stdio"),
    (Http, "http"),
}
