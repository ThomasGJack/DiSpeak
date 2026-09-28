@echo off
echo [INFO] Preparation de la base de données...
echo [INFO] Creation du fichier de base de données SQLite...

REM Creer le fichier de base de données vide
echo. > backend.db

echo [INFO] Base de données creee: backend.db
echo [INFO] Vous pouvez maintenant compiler le backend avec: cargo build
pause
