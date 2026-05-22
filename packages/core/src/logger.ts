import pino from 'pino';

// Default to 'warn' — MCP clients (Gemini, Copilot) read stdout for JSON-RPC
// and choke on any non-JSON output. Use MCP_LOG_LEVEL=info to restore verbose logging.
export const createLogger = (name: string, level: string = 'warn') => {
  const effectiveLevel = process.env.MCP_LOG_LEVEL ?? level;

  if (process.env.NODE_ENV === 'development') {
    return pino(
      {
        name,
        level: effectiveLevel,
        transport: {
          target: 'pino-pretty',
          options: {
            colorize: true,
            translateTime: 'HH:MM:ss Z',
            destination: 2, // fd 2 = stderr
          },
        },
      },
    );
  }

  return pino({ name, level: effectiveLevel }, process.stderr);
};
