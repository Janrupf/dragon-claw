@echo off
cargo build --release
wix extension add WixToolset.UI.wixext
wix extension add WixToolset.Firewall.wixext
wix build -arch x64 .\agent\installer.wxs -ext WixToolset.UI.wixext -ext WixToolset.Firewall.wixext	-out target/release/dragon-claw-installer.msi
