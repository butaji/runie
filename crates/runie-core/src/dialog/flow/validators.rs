//! Common Validators

use crate::dialog::dsl::Panel;
use crate::dialog::PanelItem;
use super::FlowContext;

/// Common validation functions
pub mod validators {
    use super::*;

    /// Validate that required form fields are not empty
    pub fn required_fields(ctx: &mut FlowContext, panel: &Panel) -> Result<(), String> {
        for item in &panel.items {
            if let PanelItem::FormField { label, key, value, .. } = item {
                if value.is_empty() {
                    return Err(format!("{} is required", label));
                }
                ctx.data.insert(key.clone(), value.clone());
            }
        }
        Ok(())
    }

    /// Validate email format
    pub fn email(ctx: &mut FlowContext, panel: &Panel) -> Result<(), String> {
        for item in &panel.items {
            if let PanelItem::FormField { key, value, .. } = item {
                if !value.is_empty() && !value.contains('@') {
                    return Err(format!("Invalid email: {}", value));
                }
                if !value.is_empty() {
                    ctx.data.insert(key.clone(), value.clone());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::dsl::panel;

    #[test]
    fn test_required_fields_empty() {
        let mut ctx = FlowContext::new();
        let p = panel("test", "Test")
            .field("Name", "enter name", "name")
            .field("Email", "enter email", "email");

        let result = validators::required_fields(&mut ctx, &p);
        assert!(result.is_err());
    }

    #[test]
    fn test_required_fields_filled() {
        let mut ctx = FlowContext::new();
        let p = panel("test", "Test")
            .field_value("Name", "enter name", "name", "Alice")
            .field_value("Email", "enter email", "email", "alice@example.com");

        let result = validators::required_fields(&mut ctx, &p);
        assert!(result.is_ok());
    }
}
