# Lancer l'app

## Option Windows (recommande)

### Gestionnaire complet

- Ouvrir `cmd`
- Executer `scripts/windows/start.bat`
- Choisir l'option `4` pour demarrer Backend + Serveur temps reel + Frontend

### PowerShell

- Ouvrir PowerShell
- Executer `.\scripts\windows\start.ps1`
- Choisir l'option `4`

### Desktop Tauri (Windows)

- Executer `scripts/windows/start-tauri.bat`
- Le script demarre les services requis si necessaire puis ouvre la fenetre desktop

## Option manuelle (cross-platform)

### Terminal 1 - Backend

```bash
cd backend
cargo run --bin dispeak-backend
```

### Terminal 2 - Serveur temps reel

```bash
cd server
cargo run --bin dispeak-server
```

### Terminal 3 - Frontend (dev)

```bash
cd frontend
npm run dev
```

## URLs utiles

- Frontend : `http://127.0.0.1:5173`
- Backend : `http://127.0.0.1:8000`
- WS : `ws://127.0.0.1:8001`
- API interne serveur (health) : `http://127.0.0.1:8002/health`

## Si le demarrage echoue

- Voir `docs/07-depannage.md`

