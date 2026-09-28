# DiSpeak - Gestionnaire Principal PowerShell
$Host.UI.RawUI.WindowTitle = "DiSpeak - Gestionnaire Principal"
$Host.UI.RawUI.BackgroundColor = "Black"
$Host.UI.RawUI.ForegroundColor = "Green"
Clear-Host

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\\..")).Path
Set-Location $ProjectRoot

$ServerInternalApiToken = if ($env:SERVER_INTERNAL_API_TOKEN) { $env:SERVER_INTERNAL_API_TOKEN } else { "dispeak-internal-dev-token" }

function Show-Menu {
    Clear-Host
    Write-Host ""
    Write-Host " ================================================================" -ForegroundColor Cyan
    Write-Host "                    DiSpeak - Gestionnaire Principal" -ForegroundColor White
    Write-Host " ================================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host " Options disponibles :" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "   [1] Demarrer Backend API (Port 8000)" -ForegroundColor White
    Write-Host "   [2] Demarrer Serveur WebSocket (Port 8001)" -ForegroundColor White
    Write-Host "   [3] Demarrer Frontend Web (Port 5173)" -ForegroundColor White
    Write-Host "   [4] Demarrer TOUT (Backend + Serveur + Frontend)" -ForegroundColor White
    Write-Host "   [5] Demarrer Tauri (Application Desktop)" -ForegroundColor Green
    Write-Host "   [6] Synchroniser donnees (Backend -> Serveur WebSocket)" -ForegroundColor White
    Write-Host "   [7] Creer serveur de test avec canaux par defaut" -ForegroundColor White
    Write-Host "   [8] Arreter tous les services" -ForegroundColor Red
    Write-Host "   [9] Verifier l'etat des services" -ForegroundColor White
    Write-Host "   [0] Quitter" -ForegroundColor Red
    Write-Host ""
}

function Start-Backend {
    Clear-Host
    Write-Host ""
    Write-Host " Demarrage du Backend API..." -ForegroundColor Cyan
    Write-Host ""
    
    Set-Location "backend"
    
    Write-Host "[STEP 1] Verification de la base de donnees..." -ForegroundColor Yellow
    if (-not (Test-Path "backend.db")) {
        Write-Host "[INFO] Creation de la base de donnees..." -ForegroundColor Blue
        New-Item -ItemType File -Name "backend.db" -Force | Out-Null
    }
    
    Write-Host "[STEP 2] Compilation du backend..." -ForegroundColor Yellow
    cargo build --bin dispeak-backend
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Erreur lors de la compilation du backend" -ForegroundColor Red
        Read-Host "Appuyez sur Entree pour continuer"
        return
    }
    
    Write-Host "[STEP 3] Lancement du backend..." -ForegroundColor Yellow
    Write-Host "[OK] Backend demarre sur http://localhost:8000" -ForegroundColor Green
    Write-Host ""
    Start-Process -FilePath "cmd" -ArgumentList "/k", "set ""SERVER_INTERNAL_API_TOKEN=$ServerInternalApiToken"" && cargo run --bin dispeak-backend" -WindowStyle Normal
    Set-Location ".."
}

function Start-Server {
    Clear-Host
    Write-Host ""
    Write-Host " Demarrage du Serveur WebSocket..." -ForegroundColor Cyan
    Write-Host ""
    
    Set-Location "server"
    
    Write-Host "[STEP 1] Verification de la base de donnees..." -ForegroundColor Yellow
    if (-not (Test-Path "server.db")) {
        Write-Host "[INFO] Creation de la base de donnees..." -ForegroundColor Blue
        New-Item -ItemType File -Name "server.db" -Force | Out-Null
    }
    
    Write-Host "[STEP 2] Compilation du serveur..." -ForegroundColor Yellow
    cargo build --bin dispeak-server
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Erreur lors de la compilation du serveur" -ForegroundColor Red
        Read-Host "Appuyez sur Entree pour continuer"
        return
    }
    
    Write-Host "[STEP 3] Lancement du serveur WebSocket..." -ForegroundColor Yellow
    Write-Host "[OK] Serveur WebSocket demarre sur ws://localhost:8001" -ForegroundColor Green
    Write-Host ""
    Start-Process -FilePath "cmd" -ArgumentList "/k", "set ""SERVER_INTERNAL_API_TOKEN=$ServerInternalApiToken"" && cargo run --bin dispeak-server" -WindowStyle Normal
    Set-Location ".."
}

function Start-Frontend {
    Clear-Host
    Write-Host ""
    Write-Host " Demarrage du Frontend Web..." -ForegroundColor Cyan
    Write-Host ""
    
    Set-Location "frontend"
    
    Write-Host "[STEP 1] Verification des dependances..." -ForegroundColor Yellow
    if (-not (Test-Path "node_modules")) {
        Write-Host "[INFO] Installation des dependances..." -ForegroundColor Blue
        npm install
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[ERROR] Erreur lors de l'installation des dependances" -ForegroundColor Red
            Read-Host "Appuyez sur Entree pour continuer"
            return
        }
    }
    
    Write-Host "[STEP 2] Lancement du frontend..." -ForegroundColor Yellow
    Write-Host "[OK] Frontend demarre sur http://localhost:5173" -ForegroundColor Green
    Write-Host ""
    Start-Process -FilePath "cmd" -ArgumentList "/k", "npm run dev" -WindowStyle Normal
    Set-Location ".."
}

function Start-Tauri {
    Clear-Host
    Write-Host ""
    Write-Host " Demarrage de Tauri (Application Desktop)..." -ForegroundColor Cyan
    Write-Host ""
    
    Set-Location "frontend"
    
    Write-Host "[STEP 1] Verification des dependances..." -ForegroundColor Yellow
    if (-not (Test-Path "node_modules")) {
        Write-Host "[INFO] Installation des dependances..." -ForegroundColor Blue
        npm install
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[ERROR] Erreur lors de l'installation des dependances" -ForegroundColor Red
            Read-Host "Appuyez sur Entree pour continuer"
            return
        }
    }
    
    Write-Host "[STEP 2] Verification de Tauri..." -ForegroundColor Yellow
    if (-not (Test-Path "src-tauri")) {
        Write-Host "[ERROR] Configuration Tauri introuvable" -ForegroundColor Red
        Read-Host "Appuyez sur Entree pour continuer"
        return
    }
    
    Write-Host "[STEP 3] Lancement de Tauri..." -ForegroundColor Yellow
    Write-Host "[OK] Application desktop DiSpeak demarre" -ForegroundColor Green
    Write-Host "[INFO] Une fenetre Tauri va s'ouvrir avec l'interface DiSpeak" -ForegroundColor Blue
    Write-Host ""
    Start-Process -FilePath "cmd" -ArgumentList "/k", "npm run tauri:dev" -WindowStyle Normal
    Set-Location ".."
}

function Start-All {
    Clear-Host
    Write-Host ""
    Write-Host " Demarrage de tous les services..." -ForegroundColor Cyan
    Write-Host ""
    
    Write-Host "[STEP 1] Arret des processus existants..." -ForegroundColor Yellow
    Get-Process -Name "dispeak-backend" -ErrorAction SilentlyContinue | Stop-Process -Force
    Get-Process -Name "dispeak-server" -ErrorAction SilentlyContinue | Stop-Process -Force
    Get-Process -Name "node" -ErrorAction SilentlyContinue | Stop-Process -Force
    Write-Host "[OK] Processus arretes" -ForegroundColor Green
    
    Write-Host ""
    Write-Host "[STEP 2] Nettoyage des ports..." -ForegroundColor Yellow
    $ports = @(8000, 8001, 5173)
    foreach ($port in $ports) {
        $connections = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue
        foreach ($conn in $connections) {
            try {
                Stop-Process -Id $conn.OwningProcess -Force -ErrorAction SilentlyContinue
                Write-Host "Port $port libere" -ForegroundColor Green
            } catch {
                Write-Host "Impossible de liberer le port $port" -ForegroundColor Yellow
            }
        }
    }
    
    Write-Host ""
    Write-Host "[STEP 3] Demarrage du Backend..." -ForegroundColor Yellow
    Set-Location "backend"
    if (-not (Test-Path "backend.db")) { New-Item -ItemType File -Name "backend.db" -Force | Out-Null }
    cargo build --bin dispeak-backend
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Erreur compilation backend" -ForegroundColor Red
        Read-Host "Appuyez sur Entree pour continuer"
        return
    }
    Start-Process -FilePath "cmd" -ArgumentList "/k", "set ""SERVER_INTERNAL_API_TOKEN=$ServerInternalApiToken"" && cargo run --bin dispeak-backend" -WindowStyle Minimized
    Set-Location ".."
    
    Write-Host "[STEP 4] Attente du demarrage du backend..." -ForegroundColor Yellow
    Start-Sleep -Seconds 3
    
    Write-Host "[STEP 5] Demarrage du Serveur WebSocket..." -ForegroundColor Yellow
    Set-Location "server"
    if (-not (Test-Path "server.db")) { New-Item -ItemType File -Name "server.db" -Force | Out-Null }
    cargo build --bin dispeak-server
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Erreur compilation serveur" -ForegroundColor Red
        Read-Host "Appuyez sur Entree pour continuer"
        return
    }
    Start-Process -FilePath "cmd" -ArgumentList "/k", "set ""SERVER_INTERNAL_API_TOKEN=$ServerInternalApiToken"" && cargo run --bin dispeak-server" -WindowStyle Minimized
    Set-Location ".."
    
    Write-Host "[STEP 6] Attente du demarrage du serveur..." -ForegroundColor Yellow
    Start-Sleep -Seconds 3
    
    Write-Host "[STEP 7] Demarrage de Tauri..." -ForegroundColor Yellow
    Set-Location "frontend"
    if (-not (Test-Path "node_modules")) {
        Write-Host "Installation des dependances..." -ForegroundColor Blue
        npm install
    }
    Write-Host "[INFO] Lancement de Tauri (application desktop)..." -ForegroundColor Blue
    Start-Process -FilePath "cmd" -ArgumentList "/k", "npm run tauri:dev" -WindowStyle Normal
    Set-Location ".."
    
    Write-Host ""
    Write-Host "[OK] Tous les services sont demarres !" -ForegroundColor Green
    Write-Host ""
    Write-Host "Services disponibles :" -ForegroundColor Cyan
    Write-Host "  - Backend API : http://localhost:8000" -ForegroundColor White
    Write-Host "  - Serveur WebSocket : ws://localhost:8001" -ForegroundColor White
    Write-Host "  - Application Desktop : Fenetre Tauri" -ForegroundColor White
    Write-Host ""
    Read-Host "Appuyez sur Entree pour continuer"
}

function Sync-Data {
    Clear-Host
    Write-Host ""
    Write-Host " Synchronisation des donnees Backend -> Serveur WebSocket..." -ForegroundColor Cyan
    Write-Host ""
    
    Set-Location "server"
    
    Write-Host "[STEP 1] Compilation du script de synchronisation..." -ForegroundColor Yellow
    cargo build --bin sync_backend_data
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Erreur lors de la compilation du script de synchronisation" -ForegroundColor Red
        Read-Host "Appuyez sur Entree pour continuer"
        return
    }
    
    Write-Host "[STEP 2] Lancement de la synchronisation..." -ForegroundColor Yellow
    Write-Host "Ce script va synchroniser :" -ForegroundColor Blue
    Write-Host "  - Les serveurs (avec proprietaires)" -ForegroundColor White
    Write-Host "  - Les membres des serveurs" -ForegroundColor White
    Write-Host "  - Les canaux et categories" -ForegroundColor White
    Write-Host ""
    cargo run --bin sync_backend_data
    
    Write-Host ""
    Write-Host "[OK] Synchronisation terminee !" -ForegroundColor Green
    Write-Host "Les proprietaires peuvent maintenant rejoindre leurs serveurs" -ForegroundColor Blue
    Write-Host ""
    Read-Host "Appuyez sur Entree pour continuer"
    Set-Location ".."
}

function Create-TestServer {
    Clear-Host
    Write-Host ""
    Write-Host " Creation d'un serveur de test..." -ForegroundColor Cyan
    Write-Host ""
    
    Set-Location "server"
    
    Write-Host "[STEP 1] Compilation du script d'initialisation..." -ForegroundColor Yellow
    cargo build --bin init_default_server
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Erreur lors de la compilation du script d'initialisation" -ForegroundColor Red
        Read-Host "Appuyez sur Entree pour continuer"
        return
    }
    
    Write-Host "[STEP 2] Creation du serveur de test..." -ForegroundColor Yellow
    "test-server-123" | cargo run --bin init_default_server
    
    Write-Host ""
    Write-Host "[OK] Serveur de test cree !" -ForegroundColor Green
    Write-Host "ID du serveur : test-server-123" -ForegroundColor Cyan
    Write-Host "Vous pouvez maintenant rejoindre ce serveur depuis le frontend" -ForegroundColor Blue
    Write-Host ""
    Read-Host "Appuyez sur Entree pour continuer"
    Set-Location ".."
}

function Stop-All {
    Clear-Host
    Write-Host ""
    Write-Host " Arret de tous les services..." -ForegroundColor Cyan
    Write-Host ""
    
    Get-Process -Name "dispeak-backend" -ErrorAction SilentlyContinue | Stop-Process -Force
    Get-Process -Name "dispeak-server" -ErrorAction SilentlyContinue | Stop-Process -Force
    Get-Process -Name "node" -ErrorAction SilentlyContinue | Stop-Process -Force
    Write-Host "[OK] Tous les services ont ete arretes" -ForegroundColor Green
    Write-Host ""
    Read-Host "Appuyez sur Entree pour continuer"
}

function Check-Status {
    Clear-Host
    Write-Host ""
    Write-Host " Etat des services DiSpeak..." -ForegroundColor Cyan
    Write-Host ""
    
    Write-Host "Verification des ports :" -ForegroundColor Yellow
    $ports = @(
        @{Port=8000; Service='Backend API'; URL='http://localhost:8000'},
        @{Port=8001; Service='Serveur WebSocket'; URL='ws://localhost:8001'},
        @{Port=5173; Service='Frontend'; URL='http://localhost:5173'}
    )
    
    foreach ($portInfo in $ports) {
        $connections = Get-NetTCPConnection -LocalPort $portInfo.Port -ErrorAction SilentlyContinue
        if ($connections) {
            Write-Host "[OK] $($portInfo.Service) actif sur le port $($portInfo.Port)" -ForegroundColor Green
            Write-Host "   URL: $($portInfo.URL)" -ForegroundColor Cyan
        } else {
            Write-Host "[OFF] $($portInfo.Service) inactif sur le port $($portInfo.Port)" -ForegroundColor Red
        }
    }
    
    Write-Host ""
    Write-Host "Verification des processus :" -ForegroundColor Yellow
    
    if (Get-Process -Name "dispeak-backend" -ErrorAction SilentlyContinue) {
        Write-Host "[OK] Backend en cours d'execution" -ForegroundColor Green
    } else {
        Write-Host "[OFF] Backend arrete" -ForegroundColor Red
    }
    
    if (Get-Process -Name "dispeak-server" -ErrorAction SilentlyContinue) {
        Write-Host "[OK] Serveur WebSocket en cours d'execution" -ForegroundColor Green
    } else {
        Write-Host "[OFF] Serveur WebSocket arrete" -ForegroundColor Red
    }
    
    if (Get-Process -Name "node" -ErrorAction SilentlyContinue) {
        Write-Host "[OK] Frontend en cours d'execution" -ForegroundColor Green
    } else {
        Write-Host "[OFF] Frontend arrete" -ForegroundColor Red
    }
    
    Write-Host ""
    Read-Host "Appuyez sur Entree pour continuer"
}

# Menu principal
do {
    Show-Menu
    $choice = Read-Host "   Votre choix (0-9)"
    
    switch ($choice) {
        "1" { Start-Backend }
        "2" { Start-Server }
        "3" { Start-Frontend }
        "4" { Start-All }
        "5" { Start-Tauri }
        "6" { Sync-Data }
        "7" { Create-TestServer }
        "8" { Stop-All }
        "9" { Check-Status }
        "0" { 
            Clear-Host
            Write-Host ""
            Write-Host " Au revoir ! Merci d'avoir utilise DiSpeak." -ForegroundColor Green
            Write-Host ""
            Start-Sleep -Seconds 2
            exit
        }
        default { 
            Write-Host "Choix invalide. Veuillez choisir entre 0 et 9." -ForegroundColor Red
            Start-Sleep -Seconds 2
        }
    }
} while ($true)



