@echo off
call "D:\VisualStudio\VS2022\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d E:\work\code\MTools\src-tauri
cargo build --bin monotools-cli --release %*
