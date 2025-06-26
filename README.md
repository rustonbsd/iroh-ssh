# iroh-ssh

[![Crates.io](https://img.shields.io/crates/v/iroh-ssh.svg)](https://crates.io/crates/iroh-ssh)
[![Documentation](https://docs.rs/iroh-ssh/badge.svg)](https://docs.rs/iroh-ssh)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

**SSH to any machine behind NAT/firewall without port forwarding or VPN setup.**

```bash
iroh-ssh user@38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac
```

**That's all it takes.** (requires ssh/(an ssh server) to be installed)

---
## Client Connection  

*GIF placeholder: Connecting to remote server*

```bash
# Install for your distro (see below)
# Connect from anywhere
> iroh-ssh root@38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac
```

Works through any firewall, NAT, or private network. No configuration needed.

---

## Server Setup

*GIF placeholder: Installing and starting iroh-ssh service*

```bash
# Install for your distro (see below)
# (use with tmux or install as service on linux)

> iroh-ssh server

    Connect to this this machine:

    iroh-ssh root@38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac

    where root is the username you want to connect to.

    Server listening for iroh connections...
    client -> iroh-ssh -> direct connect -> iroh-ssh -> local ssh :22
    Waiting for incoming connections...
    Press Ctrl+C to exit

```

```bash```

Display its Node ID and share it to allow connection

```bash
> iroh-ssh info

    Your iroh-ssh nodeid: 38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac
    iroh-ssh version 0.2.0
    https://github.com/rustonbsd/iroh-ssh

    run 'iroh-ssh server' to start the server
    run 'iroh-ssh service' to start the server as a service

    Your iroh-ssh nodeid:
    iroh-ssh root@38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac



```

---



## How It Works

```
┌─────────────┐    ┌──────────────┐    ┌─────────────────┐    ┌─────────────┐
│ iroh-ssh    │───▶│ system SSH   │───▶│ QUIC Tunnel     │───▶│ iroh-ssh    │
│ (your machine)   │ TCP Listener │    │ (P2P Network)   │    │ server      │
└─────────────┘    | (your machine)    └─────────────────┘    └─────────────┘
                   └──────────────┘
                           │                                           │
                           ▼                                           ▼
                   ┌──────────────┐                            ┌─────────────┐
                   │ localhost:   │                            │ SSH Server  │
                   │ random_port  │                            │ (port 22)   │
                   └──────────────┘                            └─────────────┘
```

1. **Client**: Creates local TCP listener, connects system SSH client to it
2. **Tunnel**: QUIC connection through Iroh's P2P network (automatic NAT traversal)  
3. **Server**: Proxies connections to local SSH daemon running on (e.g. port localhost:22) (requires ssh server)
4. **Authentication**: Standard SSH security applies end-to-end. The tunnel is ontop of that an encrypted QUIC connection.

## Use Cases

- **Remote servers**: Access cloud instances without exposing SSH ports
- **Home networks**: Connect to devices behind router/firewall
- **Corporate networks**: Bypass restrictive network policies
- **IoT devices**: SSH to embedded systems on private networks
- **Development**: Access staging servers and build machines

## Installation

Download the binary for your operating system from [GitHub Releases](https://github.com/rustonbsd/iroh-ssh/releases):

Linux
```bash 
# Linux
wget https://github.com/rustonbsd/iroh-ssh/releases/download/0.2.0/iroh-ssh.linux
chmod +x iroh-ssh.linux
sudo mv iroh-ssh.linux /usr/local/bin/iroh-ssh

# restart your terminal after!
```

macOS
```bash
# macOS arm
curl -LJO https://github.com/rustonbsd/iroh-ssh/releases/download/0.2.0/iroh-ssh.macos
chmod +x iroh-ssh.macos
sudo mv iroh-ssh.macos /usr/local/bin/iroh-ssh

# restart your terminal after!
```

Windows
```bash
# Windows x86 64bit
curl -L -o iroh-ssh.exe https://github.com/rustonbsd/iroh-ssh/releases/download/0.2.0/iroh-ssh.exe
mkdir %LOCALAPPDATA%\iroh-ssh
move iroh-ssh.exe %LOCALAPPDATA%\iroh-ssh\
setx PATH "%PATH%;%LOCALAPPDATA%\iroh-ssh"

# restart your terminal after!
```

Verify that it works
```bash
# restart your terminal first
> iroh-ssh --help
```

## Commands

```bash
# Get your Node ID and info
> iroh-ssh info

# Server modes
> iroh-ssh server                    # Interactive mode, e.g. use tmux (default SSH port 22)
> iroh-ssh server --ssh-port 2222    # Custom SSH port
> iroh-ssh service                   # Background daemon (Linux only, default port 22)
> iroh-ssh service --ssh-port 2222   # Background daemon with custom SSH port

# Client connection
> iroh-ssh user@<NODE_ID>            # Connect to remote server
> iroh-ssh connect user@<NODE_ID>    # Explicit connect command
```

## Security Model

- **Node ID access**: Anyone with the Node ID can reach your SSH port
- **SSH authentication**: ATM only password auth is supported
- **Persistent keys**: Uses dedicated `.ssh/iroh_ssh_ed25519` keypair
- **QUIC encryption**: Transport layer encryption between endpoints

## Status

- [x] Password authentication
- [x] Persistent SSH keys  
- [x] Linux service mode
- [ ] Add howto gifs
- [ ] Certificate support (`-i` flag)
- [ ] Additional SSH features

## License

Licensed under either of Apache License 2.0 or MIT license at your option.