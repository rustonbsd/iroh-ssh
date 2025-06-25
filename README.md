# irohssh

SSH over Iroh.

This tool provides a way to create SSH connections using an [Iroh](https://iroh.computer/) node ID instead of an IP address, allowing you to connect to devices behind NATs without needing public IPs or complex firewall configuration.

## Quick Start

Install and run `irohssh` with a single command. This will download the latest binary from GitHub Releases.

coming soon
**Linux / macOS:**
```sh
curl -L https://github.com/rustonbsd/iroh-ssh/releases/latest/download/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://github.com/rustonbsd/iroh-ssh/releases/latest/download/install.ps1 | iex
```

## Usage

`irohssh` operates in two modes: a `server` mode for the machine you want to access, and a client mode for the machine you are connecting from.

### 1. On the remote machine (the one you want to connect *to*)

Start the server. It will listen for incoming Iroh connections and forward them to the local SSH server (`sshd`).

```sh
# Start the server, forwarding to the local SSH port 22
irohssh server

# The server will print its Iroh nodeid. Copy this ID.
# Node ID: [your-node-id-will-be-here]
```

You can optionally specify a different SSH port: `irohssh server --ssh-port 2222`.

### 2. On your local machine (the one you want to connect *from*)

Use the `nodeid` from the server to open a connection.

```sh
# Paste the ssh user and nodeid from the server
irohssh user@<NODE_ID>
```

### 3. Connect with your SSH client

In a **new terminal**, use your standard `ssh` client to connect to the local address provided by `irohssh`.

```sh
# Use the address and port printed by irohssh
ssh user@127.0.0.1 -p 52695
```

## How It Works

1.  **`irohssh server`**: Starts an Iroh node, prints its unique `nodeid`, and listens for connections. For each incoming Iroh stream, it opens a corresponding TCP connection to the local `sshd` and proxies all data between them.
2.  **`irohssh <NODE_ID>`**: Starts a local TCP listener. When your `ssh` client connects to it, `irohssh` opens a stream to the target `nodeid` over the Iroh network and proxies data between your local `ssh` client and the remote `sshd`.

This creates a secure end-to-end tunnel, with Iroh handling peer discovery and NAT traversal.

## Build From Source

If you prefer to build from source, you can use `cargo`.

```sh
cargo install iroh-ssh
```

## License

Licensed under either of
* Apache License, Version 2.0
* MIT license
at your option.