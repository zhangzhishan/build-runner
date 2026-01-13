# Build Runner

A client-server build tool that maintains an initialized shell environment for faster builds.

## Problem

Some build environments require running a slow `init.ps1` script before building. This tool lets you:
1. Run init once in a persistent server
2. Send build requests from Claude Code (or any client)
3. Get streaming build output back

## Usage

### 1. Start the server (in your terminal, after init)

```powershell
# Option A: Let the server run init for you
build-runner server --init Q:\src\IndexServe\init.ps1

# Option B: Run init yourself first, then start server
. .\init.ps1
build-runner server
```

### 2. Send build requests (from Claude Code)

```bash
# Build in a specific directory
build-runner run -d Q:\src\IndexServe\private\indexserve\Saas

# With custom command (default is "quickbuild debug")
build-runner run -d Q:\src\IndexServe\private\indexserve\Saas -c "quickbuild release"
```

### 3. Other commands

```bash
# Check if server is running
build-runner status

# Stop the server
build-runner stop
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-p, --port` | TCP port for communication | 19527 |
| `-i, --init` | Path to init script (server only) | None |
| `-d, --dir` | Working directory for build | Required |
| `-c, --command` | Build command to execute | `quickbuild debug` |

## Architecture

```
┌─────────────────┐         TCP/19527         ┌─────────────────┐
│  Claude Code    │ ◄───────────────────────► │  Build Server   │
│  (client)       │    JSON over newlines     │  (initialized   │
│                 │                           │   environment)  │
└─────────────────┘                           └─────────────────┘
```

## Building

```bash
cd Q:\src\build-runner
cargo build --release
```

The binary will be at `target/release/build-runner.exe`.
