-- ============================================================================
-- Outreach Tables: Contacts, Events, Webhook Log
-- ============================================================================

-- Core contact tracking
CREATE TABLE contacts (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  linkedin_url TEXT UNIQUE NOT NULL,
  name TEXT,
  headline TEXT,
  location TEXT,
  source TEXT NOT NULL DEFAULT 'kondo_sync',
  wave TEXT DEFAULT 'wave_2',
  status TEXT NOT NULL DEFAULT 'identified',
  kondo_labels TEXT,
  kondo_notes TEXT,
  kondo_url TEXT,
  last_message_at TEXT,
  last_message_preview TEXT,
  first_contact_at TEXT,
  install_confirmed_at TEXT,
  feedback_count INTEGER DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_contacts_status ON contacts(status);
CREATE INDEX idx_contacts_wave ON contacts(wave);

-- Event audit trail (append-only)
CREATE TABLE outreach_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  contact_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  event_data TEXT,
  source TEXT NOT NULL DEFAULT 'kondo_webhook',
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_events_contact ON outreach_events(contact_id);
CREATE INDEX idx_events_type ON outreach_events(event_type);

-- Raw webhook payloads for debugging/replay
CREATE TABLE webhook_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  payload TEXT NOT NULL,
  processed INTEGER DEFAULT 0,
  error_message TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);
