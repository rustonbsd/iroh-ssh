# iroh-ssh

SSH over Iroh.

## Description

Use [Iroh](https://iroh.computer/) node_id even behind NAT firewall to connect to a remote SSH server.

## Quick Start

Download from releases or ``cargo install iroh-ssh``.

On your server you run:
    
    iroh-ssh server

On your client you run:
    
    iroh-ssh user@<NODE_ID>

## ToDo's

- [x] add ssh basic password login
- [ ] add -i support for certificates
- [ ] decide what other ssh native features to support

## More Details
`iroh-ssh` operates in two modes: a `server` mode for the machine you want to access, and a client mode for the machine you are connecting from.

### 1. On the remote machine (the one you want to connect *to*)

Start the server. It will listen for incoming Iroh connections and forward them to the local SSH server (`sshd`).

```sh
# Start the server, forwarding to the local SSH port 22
iroh-ssh server

# The server will print its Iroh nodeid. Copy this ID.
# Node ID: [your-node-id-will-be-here]
```

You can optionally specify a different SSH port: `iroh-ssh server --ssh-port 2222`.

### 2. On your local machine (the one you want to connect *from*)

Use the `nodeid` from the server to open a connection.

```sh
# Paste the ssh user and nodeid from the server
iroh-ssh user@<NODE_ID>
```

## How It Works

1.  **`iroh-ssh server`**: Starts an Iroh node, prints its unique `nodeid`, and listens for connections. For each incoming Iroh stream, it opens a corresponding TCP connection to the local `sshd` and proxies all data between them.
2.  **`iroh-ssh <NODE_ID>`**: Starts a local TCP listener. When your `ssh` client connects to it, `iroh-ssh` opens a stream to the target `nodeid` over the Iroh network and proxies data between your local `ssh` client and the remote `sshd`.

This creates a secure end-to-end tunnel, with Iroh handling peer discovery and NAT traversal.

## License

Licensed under either of
* Apache License, Version 2.0
* MIT license
at your option.
