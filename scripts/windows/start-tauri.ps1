# DiSpeak - Demarrage Tauri PowerShell
$Host.UI.RawUI.WindowTitle = "DiSpeak - Demarrage Tauri"
$Host.UI.RawUI.BackgroundColor = "Black"
$Host.UI.RawUI.ForegroundColor = "Cyan"
Clear-Host

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\\..")).Path
Set-Location $ProjectRoot

$ServerInternalApiToken = if ($env:SERVER_INTERNAL_API_TOKEN) { $env:SERVER_INTERNAL_API_TOKEN } else { "dispeak-internal-dev-token" }

Write-Host ""
Write-Host " ================================================================" -ForegroundColor Cyan
Write-Host "                    DiSpeak - Demarrage Tauri" -ForegroundColor White
Write-Host " ================================================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "[STEP 1] Verification des services requis..." -ForegroundColor Yellow
Write-Host ""

Write-Host "Verification Backend (Port 8000)..." -ForegroundColor Blue
$backend = Get-NetTCPConnection -LocalPort 8000 -ErrorAction SilentlyContinue
if ($backend) {
    Write-Host "[OK] Backend actif" -ForegroundColor Green
} else {
    Write-Host "[OFF] Backend inactif - Demarrage automatique..." -ForegroundColor Yellow
    Set-Location "backend"
    if (-not (Test-Path "backend.db")) { New-Item -ItemType File -Name "backend.db" -Force | Out-Null }
    Start-Process -FilePath "cmd" -ArgumentList "/k", "set ""SERVER_INTERNAL_API_TOKEN=$ServerInternalApiToken"" && cargo run --bin dispeak-backend" -WindowStyle Minimized
    Set-Location ".."
    Start-Sleep -Seconds 3
}

Write-Host ""
Write-Host "Verification Serveur WebSocket (Port 8001)..." -ForegroundColor Blue
$server = Get-NetTCPConnection -LocalPort 8001 -ErrorAction SilentlyContinue
if ($server) {
    Write-Host "[OK] Serveur WebSocket actif" -ForegroundColor Green
} else {
    Write-Host "[OFF] Serveur WebSocket inactif - Demarrage automatique..." -ForegroundColor Yellow
    Set-Location "server"
    if (-not (Test-Path "server.db")) { New-Item -ItemType File -Name "server.db" -Force | Out-Null }
    Start-Process -FilePath "cmd" -ArgumentList "/k", "set ""SERVER_INTERNAL_API_TOKEN=$ServerInternalApiToken"" && cargo run --bin dispeak-server" -WindowStyle Minimized
    Set-Location ".."
    Start-Sleep -Seconds 3
}

Write-Host ""
Write-Host "[STEP 2] Demarrage de Tauri..." -ForegroundColor Yellow
Write-Host ""

Set-Location "frontend"

Write-Host "Verification des dependances..." -ForegroundColor Blue
if (-not (Test-Path "node_modules")) {
    Write-Host "[INFO] Installation des dependances..." -ForegroundColor Yellow
    npm install
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Erreur lors de l'installation des dependances" -ForegroundColor Red
        Read-Host "Appuyez sur Entree pour continuer"
        exit 1
    }
}

Write-Host "Verification de la configuration Tauri..." -ForegroundColor Blue
if (-not (Test-Path "src-tauri")) {
    Write-Host "[ERROR] Configuration Tauri introuvable" -ForegroundColor Red
    Write-Host "Verifiez que le dossier src-tauri existe dans le frontend" -ForegroundColor Yellow
    Read-Host "Appuyez sur Entree pour continuer"
    exit 1
}

Write-Host ""
Write-Host "[OK] Lancement de l'application desktop DiSpeak..." -ForegroundColor Green
Write-Host ""
Write-Host "Une fenetre Tauri va s'ouvrir avec l'interface DiSpeak." -ForegroundColor Cyan
Write-Host "Cette fenetre est une application desktop native." -ForegroundColor Cyan
Write-Host ""
Write-Host "Pour fermer l'application, fermez simplement la fenetre Tauri." -ForegroundColor Yellow
Write-Host ""

npm run tauri:dev

Write-Host ""
Write-Host "Application fermee." -ForegroundColor Green
Read-Host "Appuyez sur Entree pour continuer"



