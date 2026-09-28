@echo off
echo [INFO] Installation des dependances du serveur...

REM Verification de Rust
cargo --version >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Rust n'est pas installe !
    echo [INFO] Installez Rust depuis https://rustup.rs/
    pause
    exit /b 1
)

REM Installation des dependances
echo [INFO] Installation des dependances Rust...
cargo build

if %errorlevel% neq 0 (
    echo [ERROR] Echec de l'installation des dependances
    pause
    exit /b 1
)

echo [SUCCESS] Installation terminee !
pause
