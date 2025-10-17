# JetKVM MCP Server

A Model Context Protocol (MCP) server that exposes the complete [jetkvm_client](https://github.com/nilp0inter/jetkvm_client) API, allowing AI assistants like Claude to control JetKVM devices remotely.

## Features

- **Keyboard Control**: Send text and key combinations to remote systems
- **Screenshot Capture**: Capture screenshots from the WebRTC video stream
- **Mouse Control**: Move cursor, click, and drag (coming soon)
- **System Management**: Reboot, shutdown, and power management (coming soon)
- **Hardware Configuration**: EDID and display settings (coming soon)

## Prerequisites

- Rust 1.70 or later
- A JetKVM device accessible on your network
- GStreamer libraries (for video functionality)

On NixOS/with Nix flakes:
```bash
nix develop
```

On other systems, install:
- GStreamer and development headers
- OpenSSL and pkg-config

## Installation

### From Source

```bash
git clone https://github.com/nilp0inter/jetkvm_mcp_server.git
cd jetkvm_mcp_server
cargo build --release
```

The binary will be at `target/release/jetkvm_mcp_server`.

## Configuration

The server requires two environment variables:

- `JETKVM_HOST`: The hostname or IP address with port (e.g., `192.168.1.100:80`)
- `JETKVM_PASSWORD`: The password for your JetKVM device

## Usage

### Running the Server

The server communicates via stdio transport, which is the standard for MCP servers:

```bash
export JETKVM_HOST="192.168.1.100:80"
export JETKVM_PASSWORD="your_password"
./target/release/jetkvm_mcp_server
```

### Using with Claude Desktop

Add the server to your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
**Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "jetkvm": {
      "command": "/path/to/jetkvm_mcp_server",
      "env": {
        "JETKVM_HOST": "192.168.1.100:80",
        "JETKVM_PASSWORD": "your_password"
      }
    }
  }
}
```

### Using with Cursor IDE

Similar configuration in Cursor's MCP settings.

## Available Tools

### send_text

Send text to the remote system through the JetKVM device.

**Parameters:**
- `text` (string): The text to send

**Example usage in Claude:**
```
Please use the jetkvm server to send "Hello World" to my remote computer
```

### screenshot

Capture a screenshot from the remote system's display via the WebRTC video stream.

**Parameters (both required):**
- `max_width` (integer, **required**): Maximum width in pixels. Use conservative values (e.g., 800-1024) to avoid overwhelming the context window.
- `max_height` (integer, **required**): Maximum height in pixels. Use conservative values (e.g., 600-768) to avoid overwhelming the context window.

**Example usage in Claude:**
```
Please capture a screenshot from my remote computer using jetkvm with max width 800 and max height 600
```

**Returns:** A base64-encoded PNG image that can be displayed directly in the MCP client.

**Scaling behavior:**
- The image is always scaled to fit within the specified max_width and max_height
- Aspect ratio is preserved (no deformation)
- If the original image is smaller than the specified dimensions, it remains at original size
- Uses Lanczos3 filter for high-quality downscaling

**Important:** Always use conservative resolution values (recommended: 800x600 or 1024x768) to prevent large images from consuming excessive context window space.

**Note:** The screenshot is captured from the active WebRTC video stream. Make sure the video connection is established before requesting a screenshot (usually takes ~500ms after connection).

## Development

### Project Structure

```
jetkvm_mcp_server/
├── Cargo.toml          # Dependencies and project metadata
├── src/
│   ├── main.rs         # Server entry point
│   ├── lib.rs          # Library exports
│   └── server.rs       # MCP server implementation with tools
└── README.md
```

### Adding New Tools

To add a new tool, add a method to the `JetKvmServer` implementation in `src/server.rs`:

```rust
#[tool(description = "Description of what the tool does")]
async fn tool_name(&self, params: Parameters<ToolParams>) -> Result<CallToolResult, McpError> {
    // Tool implementation using jetkvm_client
}
```

### Testing

To test the server manually:

```bash
# Set environment variables
export JETKVM_HOST="192.168.1.100:80"
export JETKVM_PASSWORD="your_password"

# Run the server
cargo run
```

The server will connect to your JetKVM device and wait for MCP requests on stdin/stdout.

## Architecture

This server uses:
- **rmcp v0.8.1**: Official Rust implementation of the Model Context Protocol
- **jetkvm_client v1.0.0**: Rust client library for JetKVM WebRTC communication
- **tokio**: Async runtime for handling concurrent operations

The server maintains a persistent WebRTC connection to the JetKVM device and exposes its functionality as MCP tools that can be called by AI assistants.

## Troubleshooting

### Connection Issues

If the server fails to connect:
1. Verify your JetKVM device is powered on and accessible
2. Check the host/port combination is correct
3. Verify the password is correct
4. Ensure your firewall allows WebRTC connections

### Build Issues

If you encounter GStreamer-related build errors:
- Ensure GStreamer development packages are installed
- On NixOS, use `nix develop` to enter the development environment
- Check that `PKG_CONFIG_PATH` includes GStreamer

## Contributing

Contributions are welcome! Areas for expansion:
- Additional keyboard controls (special keys, macros)
- Mouse movement and click operations
- Screenshot capture functionality
- System management tools (reboot, shutdown)
- Hardware configuration tools
- Network management
- USB device management

## License

MIT License - see LICENSE file for details

## Acknowledgments

- Built on [jetkvm_client](https://github.com/nilp0inter/jetkvm_client) by nilp0inter
- Uses [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - official Rust MCP SDK
- Inspired by the Model Context Protocol specification

## Related Projects

- [jetkvm_client](https://github.com/nilp0inter/jetkvm_client) - The underlying Rust client library
- [Model Context Protocol](https://modelcontextprotocol.io/) - Protocol specification
