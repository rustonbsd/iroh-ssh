echo "[Unit]
Description=SSH over Iroh

[Service]
Type=simple
WorkingDirectory=~
ExecStart=/bin/bash -c 'iroh-ssh server -p --ssh-port [SSHPORT]'
Restart=on-failure
RestartSec=3s

[Install]
WantedBy=multi-user.target" > /etc/systemd/system/iroh-ssh-server.service

cp [BINARYPATH] /usr/local/bin/iroh-ssh

systemctl is-active iroh-ssh-server.service
if [ $? -eq 0 ]; then
    echo "Service is already running"
    exit 0
else
    echo "Starting service..."
    systemctl enable iroh-ssh-server.service
    systemctl start iroh-ssh-server.service
fi