//! TSX to Rust transpiler for TUI applications.
//!
//! This is a simplified transpiler that handles the specific patterns
//! used in the app.tsx PoC. It extracts state, JSX structure, and
//! event handlers, then emits compilable Rust code.

use regex::Regex;
use std::fs;
use std::path::Path;

/// Compiles a TSX file into Rust source code.
pub fn compile_tsx(tsx_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(tsx_path)?;
    let component = parse_tsx(&source)?;
    let rust_code = emit_rust(&component);
    Ok(rust_code)
}

/// Parsed TSX component data.
#[derive(Debug)]
pub struct TsxComponent {
    pub name: String,
    pub state_fields: Vec<StateField>,
    pub handlers: Vec<EventHandler>,
    pub jsx_root: JsxElement,
}

#[derive(Debug, Clone)]
pub struct StateField {
    pub name: String,
    pub initial_value: i32,
}

#[derive(Debug, Clone)]
pub struct EventHandler {
    pub state_field: String,
    pub operation: String, // e.g., "+ 2"
}

#[derive(Debug, Clone)]
pub struct JsxElement {
    pub tag: String,
    pub props: JsxProps,
    pub children: Vec<JsxChild>,
}

#[derive(Debug, Clone)]
pub enum JsxChild {
    Element(JsxElement),
    Text(String),         // Plain text
    Interpolation(String), // {variable}
}

#[derive(Debug, Clone, Default)]
pub struct JsxProps {
    pub style: JsxStyle,
    pub has_on_press: bool,
}

#[derive(Debug, Clone, Default)]
pub struct JsxStyle {
    pub flex_direction: Option<String>,
    pub padding: Option<u16>,
    pub color: Option<String>,
}

fn parse_tsx(source: &str) -> Result<TsxComponent, Box<dyn std::error::Error>> {
    // Extract function name
    let name_re = Regex::new(r"export\s+function\s+(\w+)").unwrap();
    let name = name_re
        .captures(source)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or_else(|| "App".to_string());

    // Extract useState: const [count, setCount] = useState(0)
    let state_re = Regex::new(r"const\s+\[(\w+),\s*\w+\]\s*=\s*useState\((\d+)\)").unwrap();
    let mut state_fields = Vec::new();
    for caps in state_re.captures_iter(source) {
        let field_name = caps.get(1).unwrap().as_str().to_string();
        let initial: i32 = caps.get(2).unwrap().as_str().parse().unwrap();
        state_fields.push(StateField {
            name: field_name,
            initial_value: initial,
        });
    }

    // Extract handlers: onPress={() => setCount((c) => c + 2)}
    // Capture the operation: "c + 2", "c - 1", etc.
    let handler_re = Regex::new(
        r"onPress\s*=\s*\{\s*\(\)\s*=>\s*set(\w+)\s*\(\s*\((\w+)\)\s*=>\s*([^}]+)\s*\)\s*\}"
    ).unwrap();
    let mut handlers = Vec::new();
    for caps in handler_re.captures_iter(source) {
        let setter_name = caps.get(1).unwrap().as_str().to_string();
        let operation = caps.get(3).unwrap().as_str().trim().to_string();
        handlers.push(EventHandler {
            state_field: setter_name.to_lowercase(),
            operation,
        });
    }

    // Extract JSX tree
    let jsx_root = extract_jsx_root(source)?;

    Ok(TsxComponent {
        name,
        state_fields,
        handlers,
        jsx_root,
    })
}

fn extract_jsx_root(source: &str) -> Result<JsxElement, Box<dyn std::error::Error>> {
    // Find return statement and extract JSX
    let return_start = source.find("return (")
        .ok_or("No return statement found")?;
    let jsx_start = source[return_start..].find('<')
        .ok_or("No JSX found after return")?;
    let jsx_block = &source[return_start + jsx_start..];

    // Find matching closing tag for the root element
    let root = parse_jsx_element(jsx_block)?;
    Ok(root)
}

fn find_opening_tag_end(source: &str) -> Option<usize> {
    let mut in_string = false;
    let mut brace_depth = 0;
    for (i, c) in source.char_indices() {
        if c == '"' || c == '\'' {
            in_string = !in_string;
        } else if !in_string {
            if c == '{' {
                brace_depth += 1;
            } else if c == '}' {
                brace_depth -= 1;
            } else if c == '>' && brace_depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn parse_jsx_element(source: &str) -> Result<JsxElement, Box<dyn std::error::Error>> {
    let trimmed = source.trim_start();

    // Extract opening tag: <TagName props>
    if !trimmed.starts_with('<') {
        return Err("Expected opening tag".into());
    }

    let tag_end = find_opening_tag_end(trimmed).ok_or("No closing > for opening tag")?;
    let opening_tag = &trimmed[..=tag_end];

    // Extract tag name (between < and first space or >)
    let tag_name = {
        let after_lt = &opening_tag[1..tag_end];
        let space_pos = after_lt.find(|c: char| c.is_whitespace() || c == '>');
        match space_pos {
            Some(pos) => &after_lt[..pos],
            None => after_lt,
        }
        .to_string()
    };

    // Extract props
    let props_start = 1 + tag_name.len();
    let props_str = if props_start < tag_end {
        &opening_tag[props_start..tag_end]
    } else {
        ""
    };
    let props = parse_props(props_str);

    // Find matching closing tag </TagName>
    let closing_tag = format!("</{}>", tag_name);
    let content_end = find_matching_end(trimmed, &closing_tag)
        .ok_or(format!("No closing tag found for {}", tag_name))?;

    // Extract children content (between > and </Tag>)
    let children_str = &trimmed[tag_end + 1..content_end];
    let children = parse_children(children_str)?;

    Ok(JsxElement {
        tag: tag_name,
        props,
        children,
    })
}

fn find_matching_end(source: &str, closing_tag: &str) -> Option<usize> {
    let mut depth = 1;
    let mut i = 1; // Skip the opening <

    while i < source.len() {
        let ch = source.as_bytes()[i];

        if ch == b'"' || ch == b'\'' {
            // Skip string literal
            let quote = ch;
            i += 1;
            while i < source.len() && source.as_bytes()[i] != quote {
                if source.as_bytes()[i] == b'\\' {
                    i += 1; // Skip escaped char
                }
                i += 1;
            }
            i += 1; // Skip closing quote
            continue;
        }

        if ch == b'<' {
            if i + 1 < source.len() && source.as_bytes()[i + 1] == b'/' {
                // Closing tag - check if it's ours
                if source[i..].starts_with(closing_tag) {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                    // Skip past this closing tag
                    let mut end = i + closing_tag.len();
                    while end < source.len() && source.as_bytes()[end] != b'>' {
                        end += 1;
                    }
                    i = end + 1;
                    continue;
                } else {
                    // Skip past other closing tags
                    let mut end = i + 2;
                    while end < source.len() && source.as_bytes()[end] != b'>' {
                        end += 1;
                    }
                    i = end + 1;
                    depth -= 1;
                    continue;
                }
            } else {
                // Opening tag - find its > to check if self-closing
                let mut j = i + 1;
                let mut in_string = false;
                while j < source.len() {
                    let c = source.as_bytes()[j];
                    if c == b'"' || c == b'\'' {
                        in_string = !in_string;
                    } else if !in_string && c == b'>' {
                        break;
                    }
                    j += 1;
                }
                if j > 0 && j < source.len() && source.as_bytes()[j - 1] == b'/' {
                    // Self-closing tag
                } else {
                    depth += 1;
                }
            }
        }

        i += 1;
    }

    None
}

fn parse_props(props_str: &str) -> JsxProps {
    let mut props = JsxProps::default();
    let trimmed = props_str.trim();

    // Parse style={{ ... }}
    if let Some(style_start) = trimmed.find("style=") {
        let after_style = &trimmed[style_start + 6..];
        if after_style.starts_with("{{") {
            let mut depth = 0;
            let mut end = 0;
            for (i, ch) in after_style.chars().enumerate() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        break;
                    }
                }
            }
            let style_content = &after_style[1..end]; // Remove outer braces
            props.style = parse_style(style_content);
        }
    }

    // Parse onPress
    if trimmed.contains("onPress=") {
        props.has_on_press = true;
    }

    props
}

fn parse_style(style_str: &str) -> JsxStyle {
    let mut style = JsxStyle::default();

    let flex_re = Regex::new(r#"flexDirection\s*:\s*"(\w+)""#).unwrap();
    if let Some(caps) = flex_re.captures(style_str) {
        style.flex_direction = Some(caps.get(1).unwrap().as_str().to_string());
    }

    let padding_re = Regex::new(r"padding\s*:\s*(\d+)").unwrap();
    if let Some(caps) = padding_re.captures(style_str) {
        style.padding = Some(caps.get(1).unwrap().as_str().parse().unwrap());
    }

    let color_re = Regex::new(r#"color\s*:\s*"(#[A-Fa-f0-9]+)""#).unwrap();
    if let Some(caps) = color_re.captures(style_str) {
        style.color = Some(caps.get(1).unwrap().as_str().to_string());
    }

    style
}

fn parse_children(children_str: &str) -> Result<Vec<JsxChild>, Box<dyn std::error::Error>> {
    let mut children = Vec::new();
    let trimmed = children_str.trim();

    if trimmed.is_empty() {
        return Ok(children);
    }

    // Check if this is purely text content (no child elements)
    let has_child_elements = trimmed.contains('<');

    if !has_child_elements {
        // Pure text - check for interpolations like {count}
        children.extend(parse_text_with_interpolations(trimmed));
        return Ok(children);
    }

    // Mixed content: child elements and text
    let mut pos = 0;
    let chars: Vec<char> = trimmed.chars().collect();

    while pos < chars.len() {
        // Skip whitespace
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }
        if pos >= chars.len() {
            break;
        }

        if chars[pos] == '<' {
            // Parse child element
            let elem_str = &trimmed[pos..];
            if let Ok(elem) = parse_jsx_element(elem_str) {
                // Calculate how many chars this element consumed
                let consumed = find_element_length(elem_str, &elem.tag).unwrap_or(elem_str.len());
                children.push(JsxChild::Element(elem));
                pos += consumed;
            } else {
                pos += 1;
            }
        } else {
            // Text content until next <
            let text_start = pos;
            while pos < chars.len() && chars[pos] != '<' {
                pos += 1;
            }
            let text = trimmed[text_start..pos].trim();
            if !text.is_empty() {
                children.extend(parse_text_with_interpolations(text));
            }
        }
    }

    Ok(children)
}

fn find_element_length(source: &str, tag_name: &str) -> Option<usize> {
    let closing = format!("</{}>", tag_name);
    find_matching_end(source, &closing).map(|end| end + closing.len())
}

fn parse_text_with_interpolations(text: &str) -> Vec<JsxChild> {
    let mut result = Vec::new();
    let re = Regex::new(r"\{(\w+)\}").unwrap();

    if !re.is_match(text) {
        result.push(JsxChild::Text(text.to_string()));
        return result;
    }

    let mut last_end = 0;
    for caps in re.captures_iter(text) {
        let mat = caps.get(0).unwrap();
        let prefix = &text[last_end..mat.start()];
        if !prefix.is_empty() {
            result.push(JsxChild::Text(prefix.to_string()));
        }
        let var_name = caps.get(1).unwrap().as_str().to_string();
        result.push(JsxChild::Interpolation(var_name));
        last_end = mat.end();
    }

    let suffix = &text[last_end..];
    if !suffix.is_empty() {
        result.push(JsxChild::Text(suffix.to_string()));
    }

    result
}

fn emit_rust(component: &TsxComponent) -> String {
    let state_struct = emit_state_struct(component);
    let app_struct = "struct App {\n    state: AppState,\n    renderer: RatatuiRenderer,\n}";

    let initializers = component
        .state_fields
        .iter()
        .map(|f| format!("                {}: {},", f.name, f.initial_value))
        .collect::<Vec<_>>()
        .join("\n");

    let vnode = emit_element(&component.jsx_root, component);

    let handle_key = emit_handle_key(component);
    let dispatch = emit_dispatch(component);

    format!(
        r#"//! Generated from TSX - DO NOT EDIT
//!
//! Source: export function {name}()

use renderer::{{Renderer, RatatuiRenderer, VNode, VElement, VText, VProps, VStyle, FlexDirection}};
use ratatui::{{Terminal, backend::CrosstermBackend}};
use std::cell::RefCell;
use std::io::stdout;
use std::rc::Rc;
use crossterm::{{
    event::{{self, Event, KeyCode, KeyEvent}},
    execute,
    terminal::{{EnterAlternateScreen, LeaveAlternateScreen}},
}};
use crossterm::terminal::{{disable_raw_mode, enable_raw_mode}};

{state_struct}

{app_struct}

impl App {{
    fn new() -> Self {{
        Self {{
            state: AppState {{
{initializers}
            }},
            renderer: RatatuiRenderer::new(),
        }}
    }}

    fn render(&mut self, frame: &mut ratatui::Frame) {{
        let area = frame.size();
        let buf = frame.buffer_mut();

        let vnode = {vnode};

        self.renderer.mount(vnode, area);
        self.renderer.render(buf);
    }}

{handle_key}

{dispatch}
}}

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let app = Rc::new(RefCell::new(App::new()));

    loop {{
        let app_clone = Rc::clone(&app);
        terminal.draw(|frame| {{
            app_clone.borrow_mut().render(frame);
        }})?;

        if event::poll(std::time::Duration::from_millis(16))? {{
            if let Event::Key(key) = event::read()? {{
                let should_quit = app.borrow_mut().handle_key(key);
                if should_quit {{
                    execute!(stdout(), LeaveAlternateScreen)?;
                    disable_raw_mode()?;
                    return Ok(());
                }}
            }}
        }}
    }}
}}
"#,
        name = component.name,
        state_struct = state_struct,
        app_struct = app_struct,
        initializers = initializers,
        vnode = vnode,
        handle_key = handle_key,
        dispatch = dispatch,
    )
}

fn emit_state_struct(component: &TsxComponent) -> String {
    let fields = component
        .state_fields
        .iter()
        .map(|f| format!("    {}: i32,", f.name))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "/// Component state derived from useState calls\nstruct AppState {{\n{}\n}}",
        fields
    )
}

fn emit_element(elem: &JsxElement, component: &TsxComponent) -> String {
    let tag = &elem.tag;

    // Build style
    let mut style_parts = Vec::new();
    if let Some(fd) = &elem.props.style.flex_direction {
        let rust_fd = match fd.as_str() {
            "column" => "FlexDirection::Column",
            "row" => "FlexDirection::Row",
            _ => "FlexDirection::Row",
        };
        style_parts.push(format!("flex_direction: {}", rust_fd));
    }
    if let Some(padding) = elem.props.style.padding {
        style_parts.push(format!("padding: {}", padding));
    }
    if let Some(color) = &elem.props.style.color {
        style_parts.push(format!("color: Some(\"{}\".to_string())", color));
    }

    let style_str = if style_parts.is_empty() {
        "VStyle::default()".to_string()
    } else {
        format!("VStyle {{ {}, ..Default::default() }}", style_parts.join(", "))
    };

    // Build children
    let children_code = emit_children(&elem.children, component, &elem.props.style);

    // on_press placeholder
    let on_press_field = if elem.props.has_on_press {
        "on_press: Some(Box::new(|| {})), // Handled by dispatch_on_press"
    } else {
        ""
    };

    format!(
        "VNode::Element(VElement {{\n            tag: \"{}\".to_string(),\n            props: VProps {{\n                style: {},\n                {}\n                ..Default::default()\n            }},\n            children: {},\n        }})",
        tag,
        style_str,
        on_press_field,
        children_code
    )
}

fn emit_children(children: &[JsxChild], component: &TsxComponent, parent_style: &JsxStyle) -> String {
    if children.is_empty() {
        return "vec![]".to_string();
    }

    // If this is a Text element with mixed text+interpolation, combine into single format! string
    if children.len() > 1 && children.iter().all(|c| matches!(c, JsxChild::Text(_) | JsxChild::Interpolation(_))) {
        // Combine into format!("...", args)
        let mut format_str = String::new();
        let mut args = Vec::new();

        for child in children {
            match child {
                JsxChild::Text(text) => {
                    format_str.push_str(&text.replace("{", "{{").replace("}", "}}"));
                }
                JsxChild::Interpolation(var) => {
                    format_str.push_str("{}");
                    let is_state = component.state_fields.iter().any(|f| &f.name == var);
                    if is_state {
                        args.push(format!("self.state.{}", var));
                    } else {
                        args.push(var.clone());
                    }
                }
                _ => {}
            }
        }

        let args_str = if args.is_empty() {
            String::new()
        } else {
            format!(", {}", args.join(", "))
        };

        // Inherit color from parent if present
        let color_field = if let Some(color) = &parent_style.color {
            format!("color: Some(\"{}\".to_string()), ", color)
        } else {
            String::new()
        };

        return format!(
            "vec![VNode::Text(VText {{ content: format!(\"{}\"{}), style: VStyle {{ {}..Default::default() }} }})]",
            format_str,
            args_str,
            color_field
        );
    }

    let child_code: Vec<String> = children
        .iter()
        .map(|child| emit_child(child, component))
        .collect();

    format!("vec![{}]", child_code.join(", "))
}

fn emit_child(child: &JsxChild, component: &TsxComponent) -> String {
    match child {
        JsxChild::Element(elem) => emit_element(elem, component),
        JsxChild::Text(text) => {
            format!(
                "VNode::Text(VText {{ content: \"{}\".to_string(), style: VStyle::default() }})",
                text.replace('"', "\\\"")
            )
        }
        JsxChild::Interpolation(var) => {
            let is_state = component.state_fields.iter().any(|f| &f.name == var);
            let source = if is_state {
                format!("self.state.{}", var)
            } else {
                var.clone()
            };
            format!(
                "VNode::Text(VText {{ content: format!(\"{{}}\", {}), style: VStyle::default() }})",
                source
            )
        }
    }
}

fn emit_handle_key(component: &TsxComponent) -> String {
    let has_onpress = !component.handlers.is_empty();

    let enter_arm = if has_onpress {
        "KeyCode::Enter => {\n                self.dispatch_on_press();\n                false\n            }"
    } else {
        "KeyCode::Enter => false,"
    };

    format!(
        r#"    fn handle_key(&mut self, key: KeyEvent) -> bool {{
        match key.code {{
            {}
            KeyCode::Char('q') => true,
            _ => false,
        }}
    }}"#,
        enter_arm
    )
}

fn emit_dispatch(component: &TsxComponent) -> String {
    if component.handlers.is_empty() {
        return r#"    /// Dispatches onPress handlers
    fn dispatch_on_press(&mut self) {
        // No handlers
    }"#
        .to_string();
    }

    let ops: Vec<String> = component
        .handlers
        .iter()
        .map(|h| {
            // Parse "c + 2" into self.state.count += 2
            let re = Regex::new(r"(\w+)\s*([+\-*/])\s*(\d+)").unwrap();
            if let Some(caps) = re.captures(&h.operation) {
                let _var = caps.get(1).unwrap().as_str();
                let op = caps.get(2).unwrap().as_str();
                let val = caps.get(3).unwrap().as_str();
                format!("self.state.{} {}= {};", h.state_field, op, val)
            } else {
                format!("// Unparsed operation: {}", h.operation)
            }
        })
        .collect();

    format!(
        r#"    /// Dispatches onPress handlers
    fn dispatch_on_press(&mut self) {{
        {}
    }}"#,
        ops.join("\n        ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TSX: &str = r##"export function App() {
  const [count, setCount] = useState(0);
  return (
    <View style={{ flexDirection: "column", padding: 2 }}>
      <Text style={{ color: "#FF5733" }}>Count: {count}</Text>
      <Button onPress={() => setCount((c) => c + 2)}>Increment</Button>
    </View>
  );
}"##;

    #[test]
    fn test_parse_component() {
        let comp = parse_tsx(TEST_TSX).unwrap();
        assert_eq!(comp.name, "App");
        assert_eq!(comp.state_fields.len(), 1);
        assert_eq!(comp.state_fields[0].name, "count");
        assert_eq!(comp.state_fields[0].initial_value, 0);
        assert_eq!(comp.handlers.len(), 1);
        assert_eq!(comp.handlers[0].state_field, "count");
    }

    #[test]
    fn test_generated_code_compiles() {
        let comp = parse_tsx(TEST_TSX).unwrap();
        let code = emit_rust(&comp);

        // Verify key patterns in generated code
        assert!(code.contains("self.state.count += 2;"), "Should increment by 2");
        assert!(code.contains("format!(\"Count: {}\", self.state.count)"), "Should interpolate count");
        assert!(code.contains("color: Some(\"#FF5733\""), "Should have text color");
        assert!(code.contains("\"Increment\""), "Should have button text");
    }
}
