# PulsePad Receiver

A production-quality desktop companion application that transforms your iPhone into a professional wireless controller for Windows and macOS.

## Architecture

The project is organized as a Cargo workspace with the following crates:

### Core Crates

- **pulsepad-protocol** - Binary protocol for device communication
- **pulsepad-transport** - Transport abstraction layer (UDP, Bluetooth, USB)
- **pulsepad-input** - Input processing engine (controller, keyboard, mouse, media)
- **pulsepad-platform** - Platform abstraction for OS-specific input injection
- **pulsepad-profiles** - Profile management for controller mappings
- **pulsepad-storage** - Persistent storage and configuration management
- **pulsepad-security** - Device pairing, authentication, and session management
- **pulsepad-telemetry** - Performance metrics and system monitoring
- **pulsepad-discovery** - Device discovery via UDP broadcast, mDNS, and Bluetooth

### Application

- **src-tauri** - Tauri 2 desktop application shell with React frontend

## Features

- Real-time controller input processing
- UDP, Bluetooth LE, and USB transport support
- Binary protocol with versioning and checksums
- Input mapping profiles (gaming, productivity, media)
- Platform-native input injection (Windows, macOS)
- Device discovery and automatic reconnection
- Performance monitoring (latency, packet rate, memory)
- Secure device pairing and session management

## Development

### Prerequisites

- Rust 1.75+
- Node.js 18+
- Tauri CLI 2

### Setup

```bash
# Install dependencies
cd frontend && npm install

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## Project Structure

```
pulsepad-receiver/
├── Cargo.toml              # Workspace configuration
├── crates/
│   ├── pulsepad-protocol/  # Binary protocol
│   ├── pulsepad-transport/ # Transport abstraction
│   ├── pulsepad-input/     # Input processing
│   ├── pulsepad-platform/  # Platform backends
│   ├── pulsepad-profiles/  # Profile management
│   ├── pulsepad-storage/   # Configuration storage
│   ├── pulsepad-security/  # Security & pairing
│   ├── pulsepad-telemetry/ # Metrics collection
│   └── pulsepad-discovery/ # Device discovery
├── src-tauri/              # Tauri application
│   ├── src/
│   │   ├── main.rs
│   │   ├── app/            # Application manager
│   │   ├── commands/       # Tauri command handlers
│   │   ├── network/        # Network connection management
│   │   ├── profiles/       # Profile management
│   │   └── ui/             # UI state management
│   └── tauri.conf.json
└── frontend/               # React frontend
    ├── src/
    │   ├── components/
    │   └── styles.css
    └── package.json
```

## License

MIT
