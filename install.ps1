#Requires -RunAsAdministrator
param([string]$BinDir = "$env:USERPROFILE\.igris\bin", [string]$ConfigDir = "$env:USERPROFILE\.config\igris")
$Repo = "japo0nn/igris"
$Target = "x86_64-pc-windows-msvc"
function Info  { Write-Host "[INFO] $args" -ForegroundColor Green }
function Warn  { Write-Host "[WARN] $args" -ForegroundColor Yellow }
function Error { Write-Host "[ERROR] $args" -ForegroundColor Red; exit 1 }

# Rust
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Info "Installing Rust..."
    Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" -OutFile "$env:TEMP\rustup-init.exe" -UseBasicParsing
    & "$env:TEMP\rustup-init.exe" -y --default-toolchain stable
    $env:Path += ";$env:USERPROFILE\.cargo\bin"
} else { Info "Rust: $(rustc --version)" }

# Node.js 22
if (-not (Get-Command node -ErrorAction SilentlyContinue) -or (node --version) -notlike "v22*") {
    Info "Installing Node.js 22 via fnm..."
    Invoke-WebRequest -Uri "https://github.com/Schniz/fnm/releases/latest/download/fnm-windows.zip" -OutFile "$env:TEMP\fnm.zip" -UseBasicParsing
    Expand-Archive -Path "$env:TEMP\fnm.zip" -DestinationPath "$env:LOCALAPPDATA\fnm" -Force
    $env:Path += ";$env:LOCALAPPDATA\fnm"
    fnm install 22; fnm use 22; fnm default 22
} else { Info "Node.js: $(node --version)" }

# OmniRoute
if (Get-Command npm) { Info "Installing omniroute..."; npm install -g omniroute@latest } else { Error "npm not found" }

# Download binary
if (-not (Test-Path $BinDir)) { New-Item -ItemType Directory -Path $BinDir -Force | Out-Null }
Info "Downloading latest IGRIS release..."
try { $latest = (Invoke-WebRequest -Uri "https://api.github.com/repos/japo0nn/igris/releases/latest" -UseBasicParsing | ConvertFrom-Json).tag_name } catch { $latest = "v0.1.0" }
Invoke-WebRequest -Uri "https://github.com/japo0nn/igris/releases/download/$latest/igris-$Target.exe" -OutFile "$BinDir\igris.exe" -UseBasicParsing
Info "Binary saved to $BinDir\igris.exe"

# Config from repo
if (-not (Test-Path $ConfigDir)) { New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null }
if (-not (Test-Path "$ConfigDir\config.toml")) {
    Info "Downloading config.toml from repository..."
    Invoke-WebRequest -Uri "https://raw.githubusercontent.com/japo0nn/igris/main/config.toml" -OutFile "$ConfigDir\config.toml" -UseBasicParsing
}

# Secrets template
if (-not (Test-Path "$ConfigDir\secrets.toml")) {
@"
[llm]
api_key = "sk-your-openrouter-key"

[voice]
groq_api_key = "gsk-your-groq-key"

[telegram]
api_id = 12345
api_hash = "your-telegram-api-hash"
phone_number = "+1234567890"
"@ | Set-Content "$ConfigDir\secrets.toml"
    Warn "Edit $ConfigDir\secrets.toml with your API keys"
}

Write-Host "========================================" -ForegroundColor Green
Write-Host "  IGRIS + OmniRoute installed!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Binary:   $BinDir\igris.exe"
Write-Host "Config:   $ConfigDir\config.toml (from repo)"
Write-Host "Secrets:  $ConfigDir\secrets.toml"
Write-Host "OmniRoute: omniroute (global)`n"
Write-Host "Add $BinDir to PATH and run: igris`n"
