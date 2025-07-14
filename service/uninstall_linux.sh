
systemctl stop iroh-ssh-server.service
systemctl disable iroh-ssh-server.service
rm /etc/systemd/system/iroh-ssh-server.service
rm /usr/local/bin/iroh-ssh
systemctl daemon-reload