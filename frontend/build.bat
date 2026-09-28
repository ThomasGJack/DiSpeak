@echo off
echo [INFO] Build du frontend...

REM Verification des dependances
if not exist "node_modules" (
    echo [ERROR] Les dependances ne sont pas installees !
    echo [INFO] Lancez d'abord: install.bat
    pause
    exit /b 1
)

REM Build
echo [INFO] Compilation TypeScript et build Vite...
npm run build

if %errorlevel% neq 0 (
    echo [ERROR] Echec du build
    pause
    exit /b 1
)

echo [SUCCESS] Build termine !
pause
