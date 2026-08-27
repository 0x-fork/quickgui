export class CliError extends Error {
  readonly exitCode: number;

  constructor(message: string, options: ErrorOptions & { exitCode?: number } = {}) {
    super(message, options);
    this.name = "CliError";
    this.exitCode = options.exitCode ?? 1;
  }
}

export function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    const cause = error.cause instanceof Error ? `\n${errorMessage(error.cause)}` : "";
    return `${error.message}${cause}`;
  }
  return String(error);
}
