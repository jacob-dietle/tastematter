export type LogLevel = 'debug' | 'info' | 'warn' | 'error';
export type Component = 'frontend' | 'backend' | 'cli' | 'ipc';

export interface ErrorInfo {
  type: string;
  message: string;
  details?: Record<string, unknown>;
}

export interface LogEvent {
  timestamp: string;
  level: LogLevel;
  correlation_id: string;
  component: Component;
  operation: string;
  duration_ms?: number;
  success: boolean;
  context?: Record<string, unknown>;
  error?: ErrorInfo;
}

export interface LogServiceConfig {
  enabled: boolean;
  minLevel: LogLevel;
}
