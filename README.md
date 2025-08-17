# Time MCP Server

A Model Context Protocol server implementation in Rust that provides comprehensive time-related functionality.

## Features

- **6 Time Tools**: Current time, timezone conversion, duration calculation, time formatting, timezone info, and timezone listing
- **Dual Transport**: Supports both STDIO and HTTP transports
- **MCP 2025 Compliant**: Implements the latest MCP specification (2025-03-26)
- **Comprehensive Timezone Support**: Uses the IANA timezone database via chrono-tz

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

## Available Tools

1. `get_current_time` - Get current time in various formats
2. `convert_timezone` - Convert time between timezones  
3. `calculate_duration` - Calculate time differences
4. `format_time` - Format timestamps
5. `get_timezone_info` - Get timezone details
6. `list_timezones` - List available timezones

## Development

```bash
# Run in development mode
cargo run -- --transport=stdio

# Run tests
cargo test

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