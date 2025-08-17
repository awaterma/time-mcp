# Time MCP Server Specification

## Overview

This specification defines a Model Context Protocol (MCP) server that provides time-related functionality. The server can operate as both a networked HTTP server and a local stdio server, with its primary purpose being to provide accurate time information and time-related operations.

## Architecture

### Transport Mechanisms

The Time MCP Server supports two transport modes:

1. **STDIO Transport**: For local integrations where client and server run on the same machine
2. **HTTP Transport**: For networked deployments using the MCP Streamable HTTP transport

### Server Capabilities

The server implements the following MCP capabilities:

- **Tools**: Time-related functions that can be executed by AI models
- **Resources**: Time zone information and calendar data
- **Prompts**: Templates for time-based queries and operations

## API Specification

### Tools

#### 1. `get_current_time`

Returns the current time in various formats.

**Parameters:**
- `timezone` (optional): Target timezone (default: UTC)
- `format` (optional): Output format (`iso`, `unix`, `human`, `custom`)
- `custom_format` (optional): Custom strftime format string

**Response:**
```json
{
  "timestamp": "2025-08-17T10:30:00Z",
  "unix": 1723892200,
  "timezone": "UTC",
  "formatted": "Saturday, August 17, 2025 at 10:30 AM UTC"
}
```

#### 2. `convert_timezone`

Converts time between timezones.

**Parameters:**
- `timestamp`: Input timestamp (ISO 8601 or Unix)
- `from_timezone`: Source timezone
- `to_timezone`: Target timezone
- `format` (optional): Output format

**Response:**
```json
{
  "original": {
    "timestamp": "2025-08-17T10:30:00Z",
    "timezone": "UTC"
  },
  "converted": {
    "timestamp": "2025-08-17T06:30:00-04:00",
    "timezone": "America/New_York"
  }
}
```

#### 3. `calculate_duration`

Calculates time differences between two timestamps.

**Parameters:**
- `start_time`: Start timestamp
- `end_time`: End timestamp
- `units`: Output units (`seconds`, `minutes`, `hours`, `days`)

**Response:**
```json
{
  "duration": {
    "total_seconds": 3600,
    "hours": 1,
    "minutes": 60,
    "human_readable": "1 hour"
  }
}
```

#### 4. `format_time`

Formats timestamps according to various standards.

**Parameters:**
- `timestamp`: Input timestamp
- `format`: Format type (`iso8601`, `rfc3339`, `unix`, `custom`)
- `custom_format` (optional): Custom format string
- `timezone` (optional): Target timezone

#### 5. `get_timezone_info`

Retrieves timezone information and current offset.

**Parameters:**
- `timezone`: Timezone identifier

**Response:**
```json
{
  "timezone": "America/New_York",
  "offset": "-04:00",
  "dst_active": true,
  "abbreviation": "EDT"
}
```

#### 6. `list_timezones`

Returns available timezone identifiers.

**Parameters:**
- `region` (optional): Filter by region (e.g., "America", "Europe")

### Resources

#### 1. `timezone_database`

Provides comprehensive timezone information including:
- All IANA timezone identifiers
- DST rules and transitions
- Historical timezone changes

#### 2. `time_formats`

Documentation of supported time formats:
- ISO 8601 examples
- RFC 3339 examples
- Custom format strings
- Locale-specific formats

### Prompts

#### 1. `time_query_assistant`

Template for helping users construct time-related queries.

**Variables:**
- `user_query`: The user's time-related question
- `current_context`: Current time and timezone context

## Transport Implementation

### STDIO Transport

For local operation, the server communicates via standard input/output using JSON-RPC 2.0 messages:

```bash
# Start server
./time-mcp-server --transport=stdio

# Server listens on stdin and writes to stdout
```

### HTTP Transport

For networked operation, the server provides REST endpoints:

```
POST /mcp/tools/call
POST /mcp/resources/read
POST /mcp/prompts/get
GET /mcp/capabilities
```

**Base URL Structure:**
```
http://localhost:8080/mcp/
https://time-server.example.com/mcp/
```

### Authentication (HTTP Transport)

When running as an HTTP server, implements OAuth 2.1 with:
- Bearer token authentication
- Scope-based access control
- Resource indicators (RFC 8707)

## Configuration

### Server Configuration

```json
{
  "server": {
    "name": "time-mcp-server",
    "version": "1.0.0",
    "transport": "stdio|http",
    "host": "localhost",
    "port": 8080
  },
  "capabilities": {
    "tools": true,
    "resources": true,
    "prompts": true
  },
  "time": {
    "default_timezone": "UTC",
    "supported_formats": ["iso8601", "rfc3339", "unix", "custom"],
    "precision": "milliseconds"
  }
}
```

### Client Integration

```json
{
  "mcpServers": {
    "time-server": {
      "command": "./time-mcp-server",
      "args": ["--transport=stdio"],
      "env": {
        "DEFAULT_TIMEZONE": "UTC"
      }
    }
  }
}
```

## Security Considerations

1. **User Consent**: All time operations require explicit user authorization
2. **Rate Limiting**: HTTP transport implements rate limiting to prevent abuse
3. **Input Validation**: All timestamp inputs are validated and sanitized
4. **Error Handling**: Graceful error responses without exposing system details

## Error Handling

Standard JSON-RPC error codes:
- `-32700`: Parse error
- `-32600`: Invalid request
- `-32601`: Method not found
- `-32602`: Invalid parameters
- `-32603`: Internal error

Custom error codes:
- `-32000`: Invalid timezone
- `-32001`: Invalid timestamp format
- `-32002`: Timezone conversion error

## Implementation Requirements

### Mandatory Features

1. **Current Time**: Must provide accurate current time in UTC
2. **Timezone Support**: Must support IANA timezone database
3. **Format Conversion**: Must support ISO 8601 and Unix timestamps
4. **Error Handling**: Must implement comprehensive error responses

### Optional Features

1. **Calendar Operations**: Advanced calendar calculations
2. **Recurring Events**: Support for recurring time patterns
3. **Historical Data**: Access to historical timezone changes
4. **Performance Metrics**: Server performance and usage statistics

## Compliance

This specification complies with:
- MCP Specification 2025-03-26
- JSON-RPC 2.0 (RFC 4627)
- OAuth 2.1 (RFC 6749, RFC 8252)
- ISO 8601 time format standard
- IANA Time Zone Database

## Example Usage

### STDIO Mode
```bash
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_current_time","arguments":{}},"id":1}' | ./time-mcp-server
```

### HTTP Mode
```bash
curl -X POST http://localhost:8080/mcp/tools/call \
  -H "Content-Type: application/json" \
  -d '{"name":"get_current_time","arguments":{}}'
```

## Deployment

### Local Deployment (STDIO)
- Single executable
- No network dependencies
- Immediate startup

### Network Deployment (HTTP)
- Docker container support
- Load balancer compatibility
- Health check endpoints

## Testing

### Test Coverage Requirements

1. **Unit Tests**: All time calculation functions
2. **Integration Tests**: Transport mechanism validation
3. **Security Tests**: Authentication and authorization flows
4. **Performance Tests**: Response time and throughput metrics

### Test Data

- Standard timezone test cases
- Edge cases (leap years, DST transitions)
- Historical timezone data validation
- Format conversion accuracy tests