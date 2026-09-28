@echo off
for %%I in ("%~dp0..\..") do set "PROJECT_ROOT=%%~fI"
cd /d "%PROJECT_ROOT%"
echo 🎯 Création d'un serveur de test DiSpeak
echo.

echo [STEP 1] Création du serveur via l'API...
echo.

powershell -Command "
try {
    $body = @{
        name = 'Serveur de Test'
        description = 'Serveur de test avec canaux par défaut'
        max_users = 50
        icon = '🎮'
        color = '#7289DA'
    } | ConvertTo-Json

    $response = Invoke-WebRequest -Uri 'http://localhost:8000/api/servers' -Method POST -Body $body -ContentType 'application/json' -Headers @{'Authorization' = 'Bearer YOUR_TOKEN_HERE'}
    
    if ($response.StatusCode -eq 201) {
        $server = $response.Content | ConvertFrom-Json
        Write-Host '✅ Serveur créé avec l''ID:' $server.id -ForegroundColor Green
        Write-Host 'Nom:' $server.name -ForegroundColor Cyan
        Write-Host 'Description:' $server.description -ForegroundColor Cyan
        
        # Sauvegarder l'ID du serveur dans un fichier
        $server.id | Out-File -FilePath 'server-id.txt' -Encoding UTF8
        Write-Host 'ID sauvegardé dans server-id.txt' -ForegroundColor Yellow
    } else {
        Write-Host '❌ Erreur lors de la création du serveur:' $response.StatusCode -ForegroundColor Red
    }
} catch {
    Write-Host '❌ Erreur API:' $_.Exception.Message -ForegroundColor Red
    Write-Host 'Assurez-vous que le backend est démarré et accessible.' -ForegroundColor Yellow
}
"

echo.
echo [STEP 2] Initialisation des canaux par défaut...
echo.

if exist server-id.txt (
    echo ID du serveur trouvé dans server-id.txt
    cd /d "%~dp0\server"
    echo Exécution du script d'initialisation...
    cargo run --bin init_default_server
) else (
    echo ❌ Aucun ID de serveur trouvé. Veuillez d'abord créer un serveur.
)

echo.
echo ✅ Script terminé !
echo.
echo 💡 Pour rejoindre le serveur :
echo    1. Ouvrez http://localhost:5173
echo    2. Connectez-vous
echo    3. Rejoignez le serveur avec l'ID sauvegardé
echo.
pause



