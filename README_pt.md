[Español](README_es.md) [Portuguese](README_pt.md)
# iroh-ssh

[![Crates.io](https://img.shields.io/crates/v/iroh-ssh.svg)](https://crates.io/crates/iroh-ssh)
[![Documentation](https://docs.rs/iroh-ssh/badge.svg)](https://docs.rs/iroh-ssh)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

**Acesse qualquer máquina sem IP, atrás de um NAT/firewall, sem redirecionamento de portas ou configuração de VPN.**

```bash
# No servidor
> iroh-ssh server --persist

    Connect to this this machine:

    iroh-ssh my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330


# No cliente
> iroh-ssh user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330
# o con certificados ssh
> iroh-ssh -i ~/.ssh/id_rsa_my_cert my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330
```

**Isso é tudo o que você precisa.** (requer que o SSH (ou um servidor SSH) esteja instalado)

---

## Instalação

```bash
cargo install iroh-ssh
```

Baixe e configure automaticamente o binário para o seu sistema operacional a partir das  [GitHub Releases](https://github.com/rustonbsd/iroh-ssh/releases):

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

Verifique se a instalação foi bem-sucedida
```bash
# reinicie o terminal primeiro
> iroh-ssh --help
```

---

## Conexão do cliente

```bash
# Instale para sua distribuição primeiro (veja acima)
# Conecte-se de qualquer lugar
> iroh-ssh my-user@38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac
```

Funciona através de qualquer firewall, NAT ou rede privada. Nenhuma configuração necessária.

![Conectando ao servidor remoto](/media/t-rec_connect.gif)
<br>

---

## Configuração do servidor

```bash
# Instale para sua distribuição primeiro (veja acima)
# (use com tmux ou instale como serviço no Linux)

> iroh-ssh server --persist

    Connect to this this machine:

    iroh-ssh my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330

    (using persistent keys in /home/my-user/.ssh/irohssh_ed25519)

    Server listening for iroh connections...
    client -> iroh-ssh -> direct connect -> iroh-ssh -> local ssh :22
    Waiting for incoming connections...
    Press Ctrl+C to exit

```

ou use chaves efêmeras

```bash
# Instale para sua distribuição primeiro (veja acima)
# (use com tmux ou instale como serviço no Linux)

> iroh-ssh server

    Connect to this this machine:

    iroh-ssh my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330

    warning: (using ephemeral keys, run 'iroh-ssh server --persist' to create persistent keys)

    client -> iroh-ssh -> direct connect -> iroh-ssh -> local ssh :22
    Waiting for incoming connections...
    Press Ctrl+C to exit
    Server listening for iroh connections...

```

Exiba seu ID de nó e compartilhe-o para permitir a conexão

![Iniciando o servidor/Instalando como serviço](/media/t-rec_server_service.gif)
<br>

## Informações da conexão
```bash
// nota: funciona apenas com chaves persistentes
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



## Como funciona

```
┌─────────────┐    ┌──────────────┐     ┌─────────────────┐     ┌─────────────┐
│ iroh-ssh    │───▶│ Receptor     │────▶│ Túnel QUIC      │────▶│ servidor    │
│(Sua máquina)│    │ Interno TCP  │     │ (Rede P2P)      │     │ iroh-ssh    │
└─────────────┘    │(Sua máquina) │     └─────────────────┘     └─────────────┘
                   └──────────────┘
        │                  ▲                                           │
        ▼                  │                                           ▼
                   ┌──────────────┐                            ┌─────────────┐
        ⦜   -- ▶   │  exec:   ssh │                            │ Servidor SSH│
                   │ user@localhost                            │ (porto 22)  │
                   └──────────────┘                            └─────────────┘
```

1. **Cliente**: Cria um receptor TCP local, conecta o cliente SSH do sistema a ele
2. **Túnel**: Conexão QUIC através da rede P2P do Iroh (travessia automática de NAT)
3. **Servidor**: Encaminha a conexão para o daemon SSH que está rodando localmente (por exemplo, porta localhost:22)
4. **Autenticação**: A segurança padrão do SSH é aplicada de ponta a ponta. O túnel está sobre uma conexão QUIC criptografada.

## Casos de uso

- **Servidores remotos**: Acesse instâncias em nuvem sem expor portas SSH
- **Redes domésticas**: Conecte-se a dispositivos atrás de roteadores/firewalls
- **Redes corporativas**: Contornar políticas restritivas de rede
- **Dispositivos IoT**: SSH em sistemas embarcados em redes privadas
- **Desenvolvimento**: Acesse servidores de teste e máquinas de compilação

## Comandos

```bash
# Obtenha seu ID de nó e informações
> iroh-ssh info

# Modos de servidor
> iroh-ssh server --persist          # Modo interativo, por exemplo use tmux (porto SSH padrão 22)
> iroh-ssh server --ssh-port 2222    # Usando porta SSH personalizada (com chaves efêmeras)

# Modo serviço
> iroh-ssh service install                   # Daemon em segundo plano (apenas Linux e Windows, porta padrão 22)
> iroh-ssh service install --ssh-port 2222   # Daemon em segundo plano com porta SSH personalizada
> iroh-ssh service uninstall                 # Desinstalar serviço

# Conexão do cliente
> iroh-ssh user@<ENDPOINT_ID>                           # Conectar-se a um servidor remoto
> iroh-ssh connect user@<ENDPOINT_ID>                   # Comando de conexão explícito, funciona com todos os parâmetros e flags ssh padrão
```

## Modelo de segurança

- **Acesso por ID de nó**: Qualquer pessoa com o ID de nó pode acessar sua porta SSH
- **Autenticação SSH**: São suportadas autenticação por senha e certificados SSH
- **Chaves persistentes**: Usa um par de chaves dedicado em `.ssh/iroh_ssh_ed25519`
- **Criptografia QUIC**: Criptografia na camada de transporte entre pontos finais

## Avances

- [x] Autenticação por senha
- [x] Chaves SSH persistentes
- [x] Modo serviço no Linux
- [x] Adicionar gifs com exemplos
- [x] Adicionar flag -p para persistência
- [x] Modo serviço no Windows
- [x] (quase) todos os comandos ssh suportados
- [ ] Modo serviço no macOS

## Licença

Licenciado sob a Apache License 2.0 ou a MIT License, conforme sua escolha.
