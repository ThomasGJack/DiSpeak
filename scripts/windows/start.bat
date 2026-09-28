@echo off
title DiSpeak - Gestionnaire Principal
color 0A

for %%I in ("%~dp0..\..") do set "PROJECT_ROOT=%%~fI"

if "%SERVER_INTERNAL_API_TOKEN%"=="" set "SERVER_INTERNAL_API_TOKEN=dispeak-internal-dev-token"

:MAIN_MENU
cls
echo.
echo  ================================================================
echo                    DiSpeak - Gestionnaire Principal
echo  ================================================================
echo.
echo  Options disponibles :
echo.
echo    [1] Demarrer Backend API (Port 8000)
echo    [2] Demarrer Serveur WebSocket (Port 8001)  
echo    [3] Demarrer Frontend (Port 5173)
echo    [4] Demarrer TOUT (Backend + Serveur + Frontend)
echo    [5] Demarrer Tauri (Application Desktop)
echo    [6] Synchroniser donnees (Backend -> Serveur WebSocket)
echo    [7] Creer serveur de test avec canaux par defaut
echo    [8] Arreter tous les services
echo    [9] Verifier l'etat des services
echo    [0] Quitter
echo.
set /p choice="   Votre choix (0-9) : "

if "%choice%"=="1" goto START_BACKEND
if "%choice%"=="2" goto START_SERVER
if "%choice%"=="3" goto START_FRONTEND
if "%choice%"=="4" goto START_ALL
if "%choice%"=="5" goto START_TAURI
if "%choice%"=="6" goto SYNC_DATA
if "%choice%"=="7" goto CREATE_TEST_SERVER
if "%choice%"=="8" goto STOP_ALL
if "%choice%"=="9" goto CHECK_STATUS
if "%choice%"=="0" goto EXIT
goto MAIN_MENU

:START_BACKEND
cls
echo.
echo  Demarrage du Backend API...
echo.
cd /d "%PROJECT_ROOT%\backend"

echo [STEP 1] Verification de la base de donnees...
if not exist "backend.db" (
    echo [INFO] Creation de la base de donnees...
    type nul > backend.db
)

echo [STEP 2] Compilation du backend...
cargo build --bin dispeak-backend
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Erreur lors de la compilation du backend
    pause
    goto MAIN_MENU
)

echo [STEP 3] Lancement du backend...
echo [OK] Backend demarre sur http://localhost:8000
echo.
start "DiSpeak Backend" cmd /k "set SERVER_INTERNAL_API_TOKEN=%SERVER_INTERNAL_API_TOKEN% && cargo run --bin dispeak-backend"
goto MAIN_MENU

:START_SERVER
cls
echo.
echo  Demarrage du Serveur WebSocket...
echo.
cd /d "%PROJECT_ROOT%\server"

echo [STEP 1] Verification de la base de donnees...
if not exist "server.db" (
    echo [INFO] Creation de la base de donnees...
    type nul > server.db
)

echo [STEP 2] Compilation du serveur...
cargo build --bin dispeak-server
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Erreur lors de la compilation du serveur
    pause
    goto MAIN_MENU
)

echo [STEP 3] Lancement du serveur WebSocket...
echo [OK] Serveur WebSocket demarre sur ws://localhost:8001
echo.
start "DiSpeak Server" cmd /k "set SERVER_INTERNAL_API_TOKEN=%SERVER_INTERNAL_API_TOKEN% && cargo run --bin dispeak-server"
goto MAIN_MENU

:START_FRONTEND
cls
echo.
echo  Demarrage du Frontend...
echo.
cd /d "%PROJECT_ROOT%\frontend"

echo [STEP 1] Verification des dependances...
if not exist "node_modules" (
    echo [INFO] Installation des dependances...
    npm install
    if %ERRORLEVEL% neq 0 (
        echo [ERROR] Erreur lors de l'installation des dependances
        pause
        goto MAIN_MENU
    )
)

echo [STEP 2] Lancement du frontend...
echo [OK] Frontend demarre sur http://localhost:5173
echo.
start "DiSpeak Frontend" cmd /k "npm run dev"
goto MAIN_MENU

:START_TAURI
cls
echo.
echo  Demarrage de Tauri (Application Desktop)...
echo.
cd /d "%PROJECT_ROOT%\frontend"

echo [STEP 1] Verification des dependances...
if not exist "node_modules" (
    echo [INFO] Installation des dependances...
    npm install
    if %ERRORLEVEL% neq 0 (
        echo [ERROR] Erreur lors de l'installation des dependances
        pause
        goto MAIN_MENU
    )
)

echo [STEP 2] Verification de Tauri...
if not exist "src-tauri" (
    echo [ERROR] Configuration Tauri introuvable
    pause
    goto MAIN_MENU
)

echo [STEP 3] Lancement de Tauri...
echo [OK] Application desktop DiSpeak demarre
echo [INFO] Une fenetre Tauri va s'ouvrir avec l'interface DiSpeak
echo.
start "DiSpeak Tauri" cmd /k "npm run tauri:dev"
goto MAIN_MENU

:START_ALL
cls
echo.
echo  Demarrage de tous les services...
echo.

echo [STEP 1] Arret des processus existants...
taskkill /f /im dispeak-backend.exe 2>nul
taskkill /f /im dispeak-server.exe 2>nul
taskkill /f /im node.exe 2>nul
echo [OK] Processus arretes

echo.
echo [STEP 2] Nettoyage des ports...
powershell -Command "
$ports = @(8000, 8001, 5173)
foreach ($port in $ports) {
    $connections = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue
    foreach ($conn in $connections) {
        try {
            Stop-Process -Id $conn.OwningProcess -Force -ErrorAction SilentlyContinue
            Write-Host 'Port $port libere' -ForegroundColor Green
        } catch {
            Write-Host 'Impossible de liberer le port $port' -ForegroundColor Yellow
        }
    }
}
"

echo.
echo [STEP 3] Demarrage du Backend...
cd /d "%PROJECT_ROOT%\backend"
if not exist "backend.db" type nul > backend.db
cargo build --bin dispeak-backend
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Erreur compilation backend
    pause
    goto MAIN_MENU
)
start "DiSpeak Backend" cmd /k "set SERVER_INTERNAL_API_TOKEN=%SERVER_INTERNAL_API_TOKEN% && cargo run --bin dispeak-backend"

echo [STEP 4] Attente du demarrage du backend...
timeout /t 3 /nobreak > nul

echo [STEP 5] Demarrage du Serveur WebSocket...
cd /d "%PROJECT_ROOT%\server"
if not exist "server.db" type nul > server.db
cargo build --bin dispeak-server
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Erreur compilation serveur
    pause
    goto MAIN_MENU
)
start "DiSpeak Server" cmd /k "set SERVER_INTERNAL_API_TOKEN=%SERVER_INTERNAL_API_TOKEN% && cargo run --bin dispeak-server"

echo [STEP 6] Attente du demarrage du serveur...
timeout /t 3 /nobreak > nul

echo [STEP 7] Demarrage du Frontend...
cd /d "%PROJECT_ROOT%\frontend"
if not exist "node_modules" (
    echo Installation des dependances...
    npm install
)
echo [INFO] Lancement de Tauri (application desktop)...
start "DiSpeak Frontend" cmd /k "npm run tauri:dev"

echo.
echo [OK] Tous les services sont demarres !
echo.
echo Services disponibles :
echo   - Backend API : http://localhost:8000
echo   - Serveur WebSocket : ws://localhost:8001
echo   - Frontend : http://localhost:5173
echo.
pause
goto MAIN_MENU

:SYNC_DATA
cls
echo.
echo  Synchronisation des donnees Backend -> Serveur WebSocket...
echo.
cd /d "%PROJECT_ROOT%\server"

echo [STEP 1] Compilation du script de synchronisation...
cargo build --bin sync_backend_data
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Erreur lors de la compilation du script de synchronisation
    pause
    goto MAIN_MENU
)

echo [STEP 2] Lancement de la synchronisation...
echo Ce script va synchroniser :
echo   - Les serveurs (avec proprietaires)
echo   - Les membres des serveurs
echo   - Les canaux et categories
echo.
cargo run --bin sync_backend_data

echo.
echo [OK] Synchronisation terminee !
echo Les proprietaires peuvent maintenant rejoindre leurs serveurs
echo.
pause
goto MAIN_MENU

:CREATE_TEST_SERVER
cls
echo.
echo  Creation d'un serveur de test...
echo.
cd /d "%PROJECT_ROOT%\server"

echo [STEP 1] Compilation du script d'initialisation...
cargo build --bin init_default_server
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Erreur lors de la compilation du script d'initialisation
    pause
    goto MAIN_MENU
)

echo [STEP 2] Creation du serveur de test...
echo test-server-123 | cargo run --bin init_default_server

echo.
echo [OK] Serveur de test cree !
echo ID du serveur : test-server-123
echo Vous pouvez maintenant rejoindre ce serveur depuis le frontend
echo.
pause
goto MAIN_MENU

:STOP_ALL
cls
echo.
echo  Arret de tous les services...
echo.
taskkill /f /im dispeak-backend.exe 2>nul
taskkill /f /im dispeak-server.exe 2>nul
taskkill /f /im node.exe 2>nul
echo [OK] Tous les services ont ete arretes
echo.
pause
goto MAIN_MENU

:CHECK_STATUS
cls
echo.
echo  Etat des services DiSpeak...
echo.

echo Verification des ports :
powershell -Command "
$ports = @(
    @{Port=8000; Service='Backend API'; URL='http://localhost:8000'},
    @{Port=8001; Service='Serveur WebSocket'; URL='ws://localhost:8001'},
    @{Port=5173; Service='Frontend'; URL='http://localhost:5173'}
)

foreach ($portInfo in $ports) {
    $connections = Get-NetTCPConnection -LocalPort $portInfo.Port -ErrorAction SilentlyContinue
    if ($connections) {
        Write-Host '[OK]' $portInfo.Service 'actif sur le port' $portInfo.Port -ForegroundColor Green
        Write-Host '   URL:' $portInfo.URL -ForegroundColor Cyan
    } else {
        Write-Host '[OFF]' $portInfo.Service 'inactif sur le port' $portInfo.Port -ForegroundColor Red
    }
}
"

echo.
echo Verification des processus :
tasklist /fi "imagename eq dispeak-backend.exe" 2>nul | find "dispeak-backend.exe" >nul
if %ERRORLEVEL% equ 0 (
    echo [OK] Backend en cours d'execution
) else (
    echo [OFF] Backend arrete
)

tasklist /fi "imagename eq dispeak-server.exe" 2>nul | find "dispeak-server.exe" >nul
if %ERRORLEVEL% equ 0 (
    echo [OK] Serveur WebSocket en cours d'execution
) else (
    echo [OFF] Serveur WebSocket arrete
)

tasklist /fi "imagename eq node.exe" 2>nul | find "node.exe" >nul
if %ERRORLEVEL% equ 0 (
    echo [OK] Frontend en cours d'execution
) else (
    echo [OFF] Frontend arrete
)

echo.
pause
goto MAIN_MENU

:EXIT
cls
echo.
echo  Au revoir ! Merci d'avoir utilise DiSpeak.
echo.
timeout /t 2 /nobreak > nul
exit
