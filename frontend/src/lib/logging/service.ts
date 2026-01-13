import { invoke } from '@tauri-apps/api/core';
import type { LogEvent, LogLevel, Component, ErrorInfo, LogServiceConfig } from './types';

class LogServiceImpl {
  private correlationId: string = '';
  private config: LogServiceConfig = {
    enabled: true,
    minLevel: 'info'
  };

  private levelPriority: Record<LogLevel, number> = {
    debug: 0,
    info: 1,
    warn: 2,
    error: 3
  };

  startRequest(): string {
    this.correlationId = crypto.randomUUID();
    return this.correlationId;
  }

  getCorrelationId(): string {
    if (!this.correlationId) {
      this.correlationId = crypto.randomUUID();
    }
    return this.correlationId;
  }

  setCorrelationId(id: string): void {
    this.correlationId = id;
  }

  async log(event: Partial<LogEvent> & { operation: string }): Promise<void> {
    if (!this.config.enabled) return;

    const level = event.level ?? 'info';
    if (this.levelPriority[level] < this.levelPriority[this.config.minLevel]) {
      return;
    }

    const fullEvent: LogEvent = {
      timestamp: new Date().toISOString(),
      level,
      correlation_id: event.correlation_id ?? this.getCorrelationId(),
      component: event.component ?? 'frontend',
      operation: event.operation,
      success: event.success ?? true,
      duration_ms: event.duration_ms,
      context: event.context,
      error: event.error,
    };

    try {
      await invoke('log_event', { event: fullEvent });
    } catch (e) {
      // Fallback to console if IPC fails
      console.error('[LogService] Failed to send log:', e, fullEvent);
    }
  }

  async error(
    operation: string,
    error: Error | string,
    context?: Record<string, unknown>
  ): Promise<void> {
    const errorInfo: ErrorInfo = {
      type: error instanceof Error ? error.constructor.name : 'Error',
      message: error instanceof Error ? error.message : String(error),
    };

    await this.log({
      level: 'error',
      operation,
      success: false,
      context,
      error: errorInfo,
    });
  }

  async storeMutation(
    store: string,
    mutation: string,
    metrics?: Record<string, number>
  ): Promise<void> {
    await this.log({
      component: 'frontend',
      operation: 'store_mutation',
      context: {
        store,
        mutation,
        trigger: 'ipc_response',
        metrics,
      },
    });
  }

  configure(config: Partial<LogServiceConfig>): void {
    this.config = { ...this.config, ...config };
  }
}

export const logService = new LogServiceImpl();
