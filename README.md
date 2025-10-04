# Time MCP Server

A Model Context Protocol server implementation in Rust that provides comprehensive time-related functionality.

## Features

- **6 Time Tools**: Current time, timezone conversion, duration calculation, time formatting, timezone info, and timezone listing
- **Dual Transport**: Supports both STDIO and HTTP transports
- **MCP 2025 Compliant**: Implements the latest MCP specification (2025-03-26)
- **Comprehensive Timezone Support**: Uses the IANA timezone database via chrono-tz
- **Authentication Support**: Optional OAuth2/JWT-based authentication for HTTP mode
- **RESTful API**: Full HTTP REST endpoints for all MCP operations
- **Comprehensive Testing**: Unit tests, integration tests, and HTTP API tests

## Installation

```bash
cargo build --release
```

## Usage

### STDIO Mode (Local)
```bash
./target/release/time-mcp-server --transport=stdio
```

### HTTP Mode (Networked)
```bash
./target/release/time-mcp-server --transport=http --host=localhost --port=8080
```

#### With Authentication
Set the `OAUTH_ENABLED=true` environment variable to enable authentication:
```bash
OAUTH_ENABLED=true ./target/release/time-mcp-server --transport=http --host=localhost --port=8080
```

## Available Tools

1. `get_current_time` - Get current time in various formats
2. `convert_timezone` - Convert time between timezones  
3. `calculate_duration` - Calculate time differences
4. `format_time` - Format timestamps
5. `get_timezone_info` - Get timezone details
6. `list_timezones` - List available timezones

## HTTP API Endpoints

When running in HTTP mode, the following REST endpoints are available:

- `GET /health` - Health check endpoint
- `GET /mcp/capabilities` - Get server capabilities
- `POST /mcp/tools/call` - Call a time tool
- `POST /mcp/resources/read` - Read resources (if applicable)
- `POST /mcp/prompts/get` - Get prompts (if applicable)

## Development

```bash
# Run in development mode
cargo run -- --transport=stdio

# Run all tests
cargo test

# Run specific test suites
cargo test --test unit_tests
cargo test --test http_integration_tests
cargo test --test main_integration_tests

# Build release
cargo build --release
```

## MCP Integration

Add to your MCP client configuration:

```json
{
  "mcpServers": {
    "time-server": {
      "command": "./target/release/time-mcp-server",
      "args": ["--transport=stdio"]
    }
  }
}
```

## Project Structure

```
src/
├── main.rs              # Application entry point
├── lib.rs               # Library exports
├── auth.rs              # Authentication management
├── config.rs            # Configuration handling
├── models.rs            # Data models and types
├── tools.rs             # Time tool implementations
└── handlers/
    ├── mod.rs           # Handler module exports
    ├── http.rs          # HTTP transport handler
    ├── stdio.rs         # STDIO transport handler
    └── mcp.rs           # Core MCP protocol logic
tests/
├── unit_tests.rs        # Unit tests
├── http_integration_tests.rs  # HTTP API integration tests
└── main_integration_tests.rs  # Main integration tests
```

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).