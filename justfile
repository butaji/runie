# Run the schema generator example to regenerate config.schema.json.
write-config-schema:
    cargo run -p runie-core --example write_config_schema -- config.schema.json
