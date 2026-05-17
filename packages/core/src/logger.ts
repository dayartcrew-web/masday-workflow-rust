import pino from 'pino';

export const createLogger = (name: string, level: string = 'info') => {
  return pino(
    {
      name,
      level,
      transport:
        process.env.NODE_ENV === 'development'
          ? {
              target: 'pino-pretty',
              options: {
                colorize: true,
                translateTime: 'HH:MM:ss Z',
              },
            }
          : undefined,
    },
    process.stderr,
  );
};
