[Español](README_es.md) [Portuguese](README_pt.md)
# iroh-ssh

[![Crates.io](https://img.shields.io/crates/v/iroh-ssh.svg)](https://crates.io/crates/iroh-ssh)
[![Documentation](https://docs.rs/iroh-ssh/badge.svg)](https://docs.rs/iroh-ssh)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

**SSH to any machine without ip, behind a NAT/firewall without port forwarding or VPN setup.**

```bash
# on server
> iroh-ssh server --persist

    Connect to this this machine:

    iroh-ssh my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330


# on client
> iroh-ssh user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330
# or with certificate
> iroh-ssh -i ~/.ssh/id_rsa_my_cert my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330
```

**That's all it takes.** (requires ssh/(an ssh server) to be installed)

---

## Installation

```bash
cargo install iroh-ssh
```

Download and setup the binary automatically for your operating system from [GitHub Releases](https://github.com/rustonbsd/iroh-ssh/releases):

Linux
```bash
# Linux
wget https://github.com/rustonbsd/iroh-ssh/releases/download/0.2.6/iroh-ssh.linux
chmod +x iroh-ssh.linux
sudo mv iroh-ssh.linux /usr/local/bin/iroh-ssh
```

macOS
```bash
# macOS arm
curl -LJO https://github.com/rustonbsd/iroh-ssh/releases/download/0.2.6/iroh-ssh.macos
chmod +x iroh-ssh.macos
sudo mv iroh-ssh.macos /usr/local/bin/iroh-ssh
```

Windows
```bash
# Windows x86 64bit
curl -L -o iroh-ssh.exe https://github.com/rustonbsd/iroh-ssh/releases/download/0.2.6/iroh-ssh.exe
mkdir %LOCALAPPDATA%\iroh-ssh
move iroh-ssh.exe %LOCALAPPDATA%\iroh-ssh\
setx PATH "%PATH%;%LOCALAPPDATA%\iroh-ssh"
```

Verify that the installation was successful
```bash
# restart your terminal first
> iroh-ssh --help
```

---

## Client Connection

```bash
# Install for your distro (see above)
# Connect from anywhere
> iroh-ssh my-user@38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac
```

Works through any firewall, NAT, or private network. No configuration needed.

![Connecting to remote server](/media/t-rec_connect.gif)
<br>

---

## Server Setup

```bash
# Install for your distro (see above)
# (use with tmux or install as service on linux)

> iroh-ssh server --persist

    Connect to this this machine:

    iroh-ssh my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330

    (using persistent keys in /home/my-user/.ssh/irohssh_ed25519)

    Server listening for iroh connections...
    client -> iroh-ssh -> direct connect -> iroh-ssh -> local ssh :22
    Waiting for incoming connections...
    Press Ctrl+C to exit

```

or use ephemeral keys

```bash
# Install for your distro (see above)
# (use with tmux or install as service on linux)

> iroh-ssh server

    Connect to this this machine:

    iroh-ssh my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330

    warning: (using ephemeral keys, run 'iroh-ssh server --persist' to create persistent keys)

    client -> iroh-ssh -> direct connect -> iroh-ssh -> local ssh :22
    Waiting for incoming connections...
    Press Ctrl+C to exit
    Server listening for iroh connections...

```

Display its Node ID and share it to allow connection

![Starting server/Installing as service](/media/t-rec_server_service.gif)
<br>

## Connection information
```bash
// note: works only with persistent keys
> iroh-ssh info

    Your iroh-ssh nodeid: 38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac
    iroh-ssh version 0.2.4
    https://github.com/rustonbsd/iroh-ssh

    Your server iroh-ssh nodeid:
      iroh-ssh my-user@38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac

    Your service iroh-ssh nodeid:
      iroh-ssh my-user@4fjeeiui4jdm96005255c3begj389xk3aeaeef6cfebd88344aa8c85e1dbfc1ad
```

---



## How It Works

```
┌─────────────┐    ┌──────────────┐     ┌─────────────────┐     ┌─────────────┐
│ iroh-ssh    │───▶│ internal TCP │────▶│ QUIC Tunnel     │────▶│ iroh-ssh    │
│ (your machine)   │    Listener  │     │ (P2P Network)   │     │ server      │
└─────────────┘    | (your machine)     └─────────────────┘     └─────────────┘
                   └──────────────┘
        │                  ▲                                           │
        ▼                  │                                           ▼
                   ┌──────────────┐                            ┌─────────────┐
        ⦜   -- ▶   │ run:     ssh │                            │ SSH Server  │
                   │ user@localhost                            │ (port 22)   │
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

## Commands

```bash
# Get your Node ID and info
> iroh-ssh info

# Server modes
> iroh-ssh server --persist          # Interactive mode, e.g. use tmux (default SSH port 22)
> iroh-ssh server --ssh-port 2222    # Custom SSH port (using ephemeral keys)

# Service mode
> iroh-ssh service install                   # Background daemon (linux and windows only, default port 22)
> iroh-ssh service install --ssh-port 2222   # Background daemon with custom SSH port
> iroh-ssh service uninstall                 # Uninstall service

# Client connection
> iroh-ssh user@<NODE_ID>                           # Connect to remote server
> iroh-ssh connect user@<NODE_ID>                   # Explicit connect command
> iroh-ssh -i ~/.ssh/id_rsa_my_cert user@<NODE_ID>  # Connect with certificate
> iroh-ssh -L [bind_address:]port:host:hostport user@<NODE_ID>  # Forward connections made to client (bind_addr:port) to server (host:hostport)
> iroh-ssh -R [bind_address:]port:host:hostport user@<NODE_ID>  # Forward connections made to server (bind_addr:port) to client (host:hostport)

```

## Security Model

- **Node ID access**: Anyone with the Node ID can reach your SSH port
- **SSH authentication**: SSH key file, certificate and password auth are supported
- **Persistent keys**: Uses dedicated `.ssh/iroh_ssh_ed25519` keypair
- **QUIC encryption**: Transport layer encryption between endpoints

## Status

- [x] Password authentication
- [x] Persistent SSH keys
- [x] Linux service mode
- [x] Add howto gifs
- [x] Add -p flag for persistence
- [x] Windows service mode
- [x] Certificate support (`-i` flag)
- [ ] MacOS service mode
- [ ] Additional SSH features

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
