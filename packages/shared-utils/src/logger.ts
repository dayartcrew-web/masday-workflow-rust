export const logger = {
  info(message: string, data?: unknown) {
    console.error(JSON.stringify({ level: "info", message, data }));
  },
  error(message: string, data?: unknown) {
    console.error(JSON.stringify({ level: "error", message, data }));
  },
  warn(message: string, data?: unknown) {
    console.error(JSON.stringify({ level: "warn", message, data }));
  },
};
