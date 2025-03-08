# MCP Protocol Implementation

## Protocol Compliance
- Follow the Model Context Protocol specification.
- Implement required methods like `resources/list` and `prompts/list`.
- Ensure proper JSON-RPC 2.0 formatting for all messages.
- Handle protocol errors gracefully.

## Server Implementation
- Use stdin/stdout for communication in MCP server mode.
- Implement proper error handling and response formatting.
- Provide meaningful error messages for debugging.
- Log MCP interactions appropriately.

## Claude Desktop Integration
- Ensure seamless integration with Claude Desktop.
- Set up proper environment variables for Claude Desktop.
- Support changing terraform directories at runtime.
- Document integration steps clearly.

## Security Considerations
- Implement appropriate safeguards for sensitive operations.
- Consider permission boundaries for terraform operations.
- Validate inputs to prevent command injection.
- Add safety mechanisms like confirmation prompts for destructive operations. 