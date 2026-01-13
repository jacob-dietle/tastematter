# Context OS Events Daemon Installation

## Step 1: Install Servy (run in admin terminal)

**Option A: Scoop (recommended)**
```powershell
scoop bucket add extras
scoop install servy
```

**Option B: Chocolatey**
```powershell
choco install -y servy
```

**Option C: Manual download**
Download from: https://github.com/aelassas/servy/releases

## Step 2: Verify Servy installed
```powershell
servy-cli --version
```

## Step 3: Install the daemon service (run in admin terminal)
```powershell
cd "C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system\apps\context_os_events"
.venv\Scripts\python.exe -m context_os_events.cli daemon install
```

## Step 4: Start the service
```powershell
context-os daemon start
```

## Step 5: Verify it's running
```powershell
context-os daemon status
```

## Useful commands
```powershell
# Show configuration
context-os daemon config

# View logs
context-os daemon logs

# Stop service
context-os daemon stop

# Uninstall service
context-os daemon uninstall

# Run in foreground (for debugging)
context-os daemon run
```
