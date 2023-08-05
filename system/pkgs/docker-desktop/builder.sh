#!/bin/sh

source $stdenv/setup
PATH=$dpkg/bin:$PATH

dpkg -x $src unpacked

mkdir -p $out
cp -r unpacked/* $out/

mkdir -p $out/share/applications
mv unpacked/usr/share/applications/*.desktop $out/share/applications

mkdir -p $out/etc/systemd/system
mv unpacked/usr/lib/systemd/user/docker-desktop.service $out/etc/systemd/system

cp -r unpacked/opt/docker-desktop/* $out/
