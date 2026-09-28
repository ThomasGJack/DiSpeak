@echo off
echo [INFO] Installation des dependances du frontend...

REM Verification de Node.js
echo [STEP 1] Verification de Node.js...
node --version
if %errorlevel% neq 0 (
    echo [ERROR] Node.js n'est pas installe !
    echo [INFO] Installez Node.js depuis https://nodejs.org/
    pause
    exit /b 1
)

REM Nettoyage
echo [STEP 2] Nettoyage...
if exist "node_modules" (
    echo [INFO] Suppression de node_modules...
    rmdir /s /q node_modules
)
if exist "package-lock.json" (
    echo [INFO] Suppression de package-lock.json...
    del package-lock.json
)

REM Installation
echo [STEP 3] Installation des dependances...
npm install

if %errorlevel% neq 0 (
    echo [ERROR] Echec de l'installation des dependances
    pause
    exit /b 1
)

echo [SUCCESS] Installation terminee !
pause
