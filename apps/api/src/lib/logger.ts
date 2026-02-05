type LogLevel = "debug" | "info" | "warn" | "error";

const colors = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  dim: "\x1b[2m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  magenta: "\x1b[35m",
  cyan: "\x1b[36m",
  white: "\x1b[37m",
  gray: "\x1b[90m",
};

const levelColors: Record<LogLevel, string> = {
  debug: colors.gray,
  info: colors.cyan,
  warn: colors.yellow,
  error: colors.red,
};

const levelLabels: Record<LogLevel, string> = {
  debug: "DEBUG",
  info: "INFO ",
  warn: "WARN ",
  error: "ERROR",
};

function formatTimestamp(): string {
  return new Date().toISOString().replace("T", " ").substring(0, 19);
}

function createLogger(context: string) {
  const log = (level: LogLevel, message: string, data?: unknown) => {
    const timestamp = formatTimestamp();
    const color = levelColors[level];
    const label = levelLabels[level];

    const prefix = `${colors.gray}${timestamp}${colors.reset} ${color}${label}${colors.reset} ${colors.magenta}[${context}]${colors.reset}`;

    if (data !== undefined) {
      console.log(`${prefix} ${message}`, data);
    } else {
      console.log(`${prefix} ${message}`);
    }
  };

  return {
    debug: (message: string, data?: unknown) => log("debug", message, data),
    info: (message: string, data?: unknown) => log("info", message, data),
    warn: (message: string, data?: unknown) => log("warn", message, data),
    error: (message: string, data?: unknown) => log("error", message, data),
    raw: (message: string) => console.log(message),
  };
}

export const Logger = createLogger;
export type LoggerInstance = ReturnType<typeof createLogger>;
