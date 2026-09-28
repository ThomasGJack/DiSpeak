@echo off
title DiSpeak - Demarrage Tauri
color 0B

for %%I in ("%~dp0..\..") do set "PROJECT_ROOT=%%~fI"

if "%SERVER_INTERNAL_API_TOKEN%"=="" set "SERVER_INTERNAL_API_TOKEN=dispeak-internal-dev-token"

echo.
echo  ================================================================
echo                    DiSpeak - Demarrage Tauri
echo  ================================================================
echo.

echo [STEP 1] Verification des services requis...
echo.

echo Verification Backend (Port 8000)...
powershell -Command "
$backend = Get-NetTCPConnection -LocalPort 8000 -ErrorAction SilentlyContinue
if ($backend) {
    Write-Host '[OK] Backend actif' -ForegroundColor Green
} else {
    Write-Host '[OFF] Backend inactif - Demarrage automatique...' -ForegroundColor Yellow
    Start-Process -FilePath 'cmd' -ArgumentList '/k', 'cd /d \"%PROJECT_ROOT%\backend\" && set SERVER_INTERNAL_API_TOKEN=%SERVER_INTERNAL_API_TOKEN% && cargo run --bin dispeak-backend' -WindowStyle Minimized
    Start-Sleep -Seconds 3
}
"

echo.
echo Verification Serveur WebSocket (Port 8001)...
powershell -Command "
$server = Get-NetTCPConnection -LocalPort 8001 -ErrorAction SilentlyContinue
if ($server) {
    Write-Host '[OK] Serveur WebSocket actif' -ForegroundColor Green
} else {
    Write-Host '[OFF] Serveur WebSocket inactif - Demarrage automatique...' -ForegroundColor Yellow
    Start-Process -FilePath 'cmd' -ArgumentList '/k', 'cd /d \"%PROJECT_ROOT%\server\" && set SERVER_INTERNAL_API_TOKEN=%SERVER_INTERNAL_API_TOKEN% && cargo run --bin dispeak-server' -WindowStyle Minimized
    Start-Sleep -Seconds 3
}
"

echo.
echo [STEP 2] Demarrage de Tauri...
echo.

cd /d "%PROJECT_ROOT%\frontend"

echo Verification des dependances...
if not exist "node_modules" (
    echo [INFO] Installation des dependances...
    npm install
    if %ERRORLEVEL% neq 0 (
        echo [ERROR] Erreur lors de l'installation des dependances
        pause
        exit /b 1
    )
)

echo Verification de la configuration Tauri...
if not exist "src-tauri" (
    echo [ERROR] Configuration Tauri introuvable
    echo Verifiez que le dossier src-tauri existe dans le frontend
    pause
    exit /b 1
)

echo.
echo [OK] Lancement de l'application desktop DiSpeak...
echo.
echo Une fenetre Tauri va s'ouvrir avec l'interface DiSpeak.
echo Cette fenetre est une application desktop native.
echo.
echo Pour fermer l'application, fermez simplement la fenetre Tauri.
echo.

npm run tauri:dev

echo.
echo Application fermee.
pause



