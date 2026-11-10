# Per-repo fleet start config for aicraft
# Assigned ports in mcp-central-docs/operations/WEBAPP_PORTS.md
@{
    Name         = 'aicraft'
    FrontendPort = 11260
    BackendPort  = 11261
    HealthPath   = '/health'
    Backend = @{
        Kind       = 'custom'
        Command    = 'cargo run -p aicraft-mcp -- --mcp'
        WorkingDir = '.'
    }
    Frontend = @{
        Kind       = 'custom'
        Command    = 'cargo run -p aicraft-app'
        WorkingDir = '.'
    }
}
