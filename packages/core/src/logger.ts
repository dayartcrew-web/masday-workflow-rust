import pino from 'pino';

// Default to 'warn' — MCP clients (Gemini, Copilot) read stdout for JSON-RPC
// and choke on any non-JSON output. Use MCP_LOG_LEVEL=info to restore verbose logging.

// Singleton root logger — ensures ONE pino-pretty transport (one set of exit listeners).
// Without this, each createLogger() call spawns a new transport worker thread,
// registering 11+ process exit listeners per call and triggering MaxListenersExceededWarning.
let _root: pino.Logger | null = null;

function getRootLogger(): pino.Logger {
  if (!_root) {
    const defaultLevel = process.env.MCP_LOG_LEVEL ?? 'warn';
    if (process.env.NODE_ENV === 'development') {
      _root = pino({
        level: defaultLevel,
        transport: {
          target: 'pino-pretty',
          options: {
            colorize: true,
            translateTime: 'HH:MM:ss Z',
            destination: 2, // fd 2 = stderr
          },
        },
      });
    } else {
      _root = pino({ level: defaultLevel }, process.stderr);
    }
  }
  return _root;
}

export const createLogger = (name: string, level: string = 'warn') => {
  const effectiveLevel = process.env.MCP_LOG_LEVEL ?? level;
  return getRootLogger().child({ name }, { level: effectiveLevel });
};
