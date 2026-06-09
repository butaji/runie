# Config Migrations

**Status**: todo
**Milestone**: R3
**Category**: Configuration

## Description

Automatic config migration when format changes. Versioned config with upgrade paths.

## Architecture

```rust
pub const CURRENT_CONFIG_VERSION: u32 = 2;

pub struct ConfigMigrator {
    version: u32,
}

impl ConfigMigrator {
    pub fn migrate(config: &mut toml::Value) -> Result<()> {
        let version = config.get("version")
            .and_then(|v| v.as_integer())
            .unwrap_or(0) as u32;
        
        if version < 1 {
            Self::v0_to_v1(config)?;
        }
        if version < 2 {
            Self::v1_to_v2(config)?;
        }
        
        config["version"] = toml::Value::Integer(CURRENT_CONFIG_VERSION as i64);
        Ok(())
    }
    
    fn v0_to_v1(config: &mut toml::Value) -> Result<()> {
        // Migrate old flat format to nested
        Ok(())
    }
    
    fn v1_to_v2(config: &mut toml::Value) -> Result<()> {
        // Migrate provider format
        Ok(())
    }
}
```

## Acceptance Criteria

- [ ] Config version tracked in config.toml
- [ ] Auto-migration on load if version mismatch
- [ ] Backup old config before migration
- [ ] Migration paths tested

## Tests

### Layer 1
- [ ] `migrate_v0_to_v1` — flat to nested format
- [ ] `migrate_v1_to_v2` — provider format change
- [ ] `backup_created` — old config backed up
