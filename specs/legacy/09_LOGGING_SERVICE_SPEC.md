# Spec 09: Global Logging Service

**Status:** Implementation
**Created:** 2025-01-05
**Depends On:** Spec 08 (Unified Data Architecture)

## Overview

A global logging service for Tastematter that provides structured logging with correlation IDs across frontend (TypeScript) and backend (Rust) components.

### Goals

1. **Visibility** - Trace any request across frontend → IPC → backend → CLI
2. **Structured Events** - JSON logs, not printf strings
3. **Minimal Overhead** - Don't degrade code quality or performance
4. **Developer Experience** - Greppable, jq-parseable JSONL output

### Non-Goals

- User analytics or telemetry
- Production monitoring dashboards
- Real-time alerting
- Log aggregation services

## Architecture

```
Frontend (Svelte)           Backend (Rust)              File
┌─────────────┐            ┌─────────────┐            ┌──────────────────┐
│ LogService  │───(IPC)───▶│ LogService  │───write───▶│ dev-YYYY-MM-DD   │
│ (TS)        │            │ (Rust)      │            │ .jsonl           │
└─────────────┘            └─────────────┘            └──────────────────┘
      │                          │
      └────── correlation_id ────┘
```

**Design Decisions:**

1. **Backend writes to file** - Frontend sends events to backend via IPC
2. **JSONL format** - One JSON per line, greppable
3. **Daily rotation** - Automatic via filename `dev-YYYY-MM-DD.jsonl`
4. **Correlation ID generation** - Frontend generates, propagates everywhere

## Type Contracts

### Rust Types

```rust
// src-tauri/src/logging/mod.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Component {
    Frontend,
    Backend,
    Cli,
    Ipc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub timestamp: String,
    pub level: LogLevel,
    pub correlation_id: String,
    pub component: Component,
    pub operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

impl Default for LogEvent {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level: LogLevel::Info,
            correlation_id: String::new(),
            component: Component::Backend,
            operation: String::new(),
            duration_ms: None,
            success: true,
            context: None,
            error: None,
        }
    }
}
```

### TypeScript Types

```typescript
// src/lib/logging/types.ts

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
```

## Implementation

### 1. Rust LogService

```rust
// src-tauri/src/logging/service.rs

use super::{LogEvent, LogLevel};
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct LogService {
    log_dir: PathBuf,
    current_file: Mutex<Option<(String, std::fs::File)>>,
}

impl LogService {
    pub fn new() -> Self {
        let log_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".tastematter")
            .join("logs");

        // Ensure log directory exists
        fs::create_dir_all(&log_dir).ok();

        Self {
            log_dir,
            current_file: Mutex::new(None),
        }
    }

    fn get_log_file(&self) -> std::io::Result<std::fs::File> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let filename = format!("dev-{}.jsonl", today);

        let mut guard = self.current_file.lock().unwrap();

        // Check if we need a new file (day changed)
        if let Some((date, _)) = guard.as_ref() {
            if date != &today {
                *guard = None;
            }
        }

        // Open or create file
        if guard.is_none() {
            let path = self.log_dir.join(&filename);
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            *guard = Some((today, file));
        }

        // Clone the file handle
        let (_, ref file) = guard.as_ref().unwrap();
        file.try_clone()
    }

    pub fn log(&self, event: LogEvent) {
        if let Ok(mut file) = self.get_log_file() {
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = writeln!(file, "{}", json);
            }
        }
    }

    pub fn log_quick(
        &self,
        correlation_id: &str,
        component: super::Component,
        operation: &str,
        success: bool,
        duration_ms: Option<u64>,
        context: Option<serde_json::Value>,
        error: Option<super::ErrorInfo>,
    ) {
        self.log(LogEvent {
            correlation_id: correlation_id.to_string(),
            component,
            operation: operation.to_string(),
            success,
            duration_ms,
            context,
            error,
            level: if success { LogLevel::Info } else { LogLevel::Error },
            ..Default::default()
        });
    }
}

impl Default for LogService {
    fn default() -> Self {
        Self::new()
    }
}
```

### 2. Rust Integration

```rust
// src-tauri/src/lib.rs

mod logging;

use logging::LogService;
use std::sync::Arc;

pub struct AppState {
    pub log_service: Arc<LogService>,
}

// In run():
let log_service = Arc::new(LogService::new());

tauri::Builder::default()
    .manage(AppState {
        log_service: log_service.clone(),
    })
    .invoke_handler(tauri::generate_handler![
        log_event,
        // ... other commands
    ])
```

### 3. Rust IPC Command

```rust
// src-tauri/src/commands.rs

use crate::logging::{LogEvent, Component, LogLevel, ErrorInfo};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn log_event(
    event: LogEvent,
    state: State<AppState>,
) -> Result<(), String> {
    state.log_service.log(event);
    Ok(())
}
```

### 4. TypeScript LogService

```typescript
// src/lib/logging/service.ts

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
```

### 5. invokeLogged Wrapper

```typescript
// src/lib/api/tauri.ts

import { invoke } from '@tauri-apps/api/core';
import { logService } from '../logging/service';

export async function invokeLogged<T>(
  command: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  const correlationId = logService.getCorrelationId();
  const start = performance.now();

  try {
    const result = await invoke<T>(command, {
      ...args,
      correlation_id: correlationId
    });

    const duration = Math.round(performance.now() - start);

    await logService.log({
      component: 'ipc',
      operation: command,
      duration_ms: duration,
      success: true,
      context: {
        args: sanitizeArgs(args),
        result_summary: summarizeResult(result)
      }
    });

    return result;
  } catch (error) {
    const duration = Math.round(performance.now() - start);

    await logService.log({
      level: 'error',
      component: 'ipc',
      operation: command,
      duration_ms: duration,
      success: false,
      context: { args: sanitizeArgs(args) },
      error: {
        type: error instanceof Error ? error.constructor.name : 'Error',
        message: String(error)
      }
    });

    throw error;
  }
}

function sanitizeArgs(args: Record<string, unknown>): Record<string, unknown> {
  const sanitized: Record<string, unknown> = {};
  const sensitiveKeys = ['password', 'token', 'api_key', 'secret', 'credential'];

  for (const [key, value] of Object.entries(args)) {
    if (key === 'correlation_id') continue;

    if (sensitiveKeys.some(k => key.toLowerCase().includes(k))) {
      sanitized[key] = '[REDACTED]';
      continue;
    }

    if (typeof value === 'string' && value.length > 100) {
      sanitized[key] = value.slice(0, 100) + '...';
      continue;
    }

    sanitized[key] = value;
  }

  return sanitized;
}

function summarizeResult(result: unknown): string {
  if (result === null || result === undefined) return 'null';
  if (Array.isArray(result)) return `${result.length} items`;
  if (typeof result === 'object') {
    const keys = Object.keys(result as object);
    if ('count' in (result as object)) return `count: ${(result as Record<string, unknown>).count}`;
    if ('length' in (result as object)) return `length: ${(result as Record<string, unknown>).length}`;
    return `object with ${keys.length} keys`;
  }
  return typeof result;
}
```

## Integration Points

### Store Integration

Update stores to use `invokeLogged` instead of direct `invoke`:

```typescript
// src/lib/stores/files.svelte.ts

import { invokeLogged } from '../api/tauri';

export function createFilesStore(ctx: ContextStore): FilesStore {
  // ... existing code ...

  async function fetch() {
    loading = true;
    error = null;

    try {
      const result = await invokeLogged<FlexQueryResult>('query_flex', {
        time_range: ctx.timeRange,
        chain: ctx.selectedChain
      });
      data = result;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  // ... rest unchanged ...
}
```

### App Initialization

Initialize correlation ID at app start:

```svelte
<!-- src/App.svelte -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { logService } from '$lib/logging/service';

  onMount(() => {
    // Start a new correlation context for this session
    logService.startRequest();
  });
</script>
```

## File Locations

| File | Purpose |
|------|---------|
| `src-tauri/src/logging/mod.rs` | Rust types (LogEvent, Component, etc.) |
| `src-tauri/src/logging/service.rs` | Rust LogService implementation |
| `src/lib/logging/types.ts` | TypeScript types |
| `src/lib/logging/service.ts` | TypeScript LogService |
| `src/lib/api/tauri.ts` | invokeLogged wrapper |
| `~/.tastematter/logs/dev-*.jsonl` | Log output files |

## Testing Strategy

### Unit Tests

```typescript
// tests/unit/logging/service.test.ts

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

import { logService } from '$lib/logging/service';
import { invoke } from '@tauri-apps/api/core';

describe('LogService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('generates correlation ID on first call', () => {
    const id1 = logService.getCorrelationId();
    const id2 = logService.getCorrelationId();
    expect(id1).toBe(id2);
    expect(id1).toMatch(/^[0-9a-f-]{36}$/);
  });

  it('sends log event via IPC', async () => {
    await logService.log({
      operation: 'test_op',
      success: true
    });

    expect(invoke).toHaveBeenCalledWith('log_event', {
      event: expect.objectContaining({
        operation: 'test_op',
        success: true,
        level: 'info'
      })
    });
  });

  it('logs errors with full context', async () => {
    const error = new Error('Test error');
    await logService.error('failed_op', error, { extra: 'context' });

    expect(invoke).toHaveBeenCalledWith('log_event', {
      event: expect.objectContaining({
        level: 'error',
        success: false,
        error: {
          type: 'Error',
          message: 'Test error'
        },
        context: { extra: 'context' }
      })
    });
  });
});
```

### Integration Test

```typescript
// tests/integration/logging.test.ts

import { describe, it, expect, vi } from 'vitest';
import { invokeLogged } from '$lib/api/tauri';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({ items: [1, 2, 3] })
}));

describe('invokeLogged', () => {
  it('logs successful IPC call with duration', async () => {
    const result = await invokeLogged('test_command', { arg: 'value' });

    expect(result).toEqual({ items: [1, 2, 3] });
    // Verify log was sent (check invoke calls)
  });
});
```

## Migration Plan

### Phase 1: Add Infrastructure (No Breaking Changes)

1. Create `src-tauri/src/logging/` module
2. Create `src/lib/logging/` module
3. Add `log_event` command
4. Export `invokeLogged` from `src/lib/api/tauri.ts`

### Phase 2: Integrate with Stores

1. Update `files.svelte.ts` to use `invokeLogged`
2. Update `timeline.svelte.ts` to use `invokeLogged`
3. Initialize correlation ID in `App.svelte`

### Phase 3: Cleanup

1. Remove `log_to_file()` from `commands.rs`
2. Remove temporary console.logs from stores
3. Delete `debug.log`

## Success Criteria

- [ ] All IPC calls logged with correlation ID
- [ ] Log files written to `~/.tastematter/logs/`
- [ ] Can grep correlation ID to trace full request
- [ ] No performance degradation visible
- [ ] Tests pass
- [ ] Old debug.log hack removed

## Log Analysis Commands

```bash
# Trace a request
grep "CORRELATION_ID" ~/.tastematter/logs/dev-*.jsonl | jq '.'

# Find all errors
cat ~/.tastematter/logs/dev-*.jsonl | jq 'select(.success == false)'

# Find slow operations
cat ~/.tastematter/logs/dev-*.jsonl | jq 'select(.duration_ms > 100)'

# Count by operation
cat ~/.tastematter/logs/dev-*.jsonl | jq -s 'group_by(.operation) | map({op: .[0].operation, count: length})'
```
