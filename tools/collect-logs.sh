#!/bin/bash
set -x

LOG_DIR=${LOG_DIR:-/tmp/devstack-logs}
mkdir -p $LOG_DIR

sudo journalctl --dmesg -o short-precise --no-pager &> $LOG_DIR/dmesg.log
free -m > $LOG_DIR/free.txt
dpkg -l > $LOG_DIR/dpkg-l.txt
sudo ps auxf > $LOG_DIR/ps.txt
pip freeze > $LOG_DIR/pip-freeze.txt
# This is redundant with the main log (and with the action output)
rm -f $LOG_DIR/devstack.log.*

for svc in $(sudo systemctl list-unit-files --type=service --state=enabled \
        | awk '/devstack.*\.service/ {print $1}'); do
    sudo journalctl --unit "$svc" -o short-precise --no-pager &> "$LOG_DIR/$svc.log"
done

sudo find $LOG_DIR -type d -exec chmod 0755 {} \;
sudo find $LOG_DIR -type f -exec chmod 0644 {} \;
