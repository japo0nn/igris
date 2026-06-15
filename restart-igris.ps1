$igris = Get-Process igris -ErrorAction SilentlyContinue
if ($igris) { $igris | Stop-Process -Force }
Start-Sleep 1
Set-Location 'C:\Users\sosa\Documents\Workspace\igris'
$build = cargo build 2>&1
if ($LASTEXITCODE -ne 0) { Write-Output "BUILD FAILED: $build"; return }
Write-Output "Build OK"
Start-Process -WindowStyle Normal "$project\target\debug\igris.exe"
Write-Output "IGRIS restarted"
