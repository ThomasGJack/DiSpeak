@echo off
echo 🚀 Initialisation du serveur par défaut DiSpeak
echo.

echo [STEP 1] Compilation du script d'initialisation...
cd /d "%~dp0"
cargo build --bin init_default_server

if %ERRORLEVEL% neq 0 (
    echo ❌ Erreur lors de la compilation
    pause
    exit /b 1
)

echo [STEP 2] Lancement du script d'initialisation...
echo.
echo Entrez l'ID du serveur à initialiser (ou appuyez sur Entrée pour 'default-server-123'):
cargo run --bin init_default_server

echo.
echo ✅ Script terminé !
pause



