@echo off
echo [INFO] Initialisation de la base de données DiSpeak Backend...

REM Creer le dossier migrations s'il n'existe pas
if not exist "migrations" mkdir migrations

REM Creer un fichier de base de données SQLite vide
echo [INFO] Creation du fichier de base de données...
sqlite3 backend.db ""

REM Executer les migrations
echo [INFO] Execution des migrations...
cargo run --bin migrate

echo [SUCCESS] Base de données initialisee !
echo [INFO] Vous pouvez maintenant compiler avec: cargo build
pause
