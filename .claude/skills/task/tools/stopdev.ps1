# Stops only the JobertsVault dev processes that `npm run tauri dev` starts.
$mine = Get-CimInstance Win32_Process | Where-Object {
  ($_.Name -eq 'jobertsvault.exe' -and $_.ExecutablePath -like '*\src-tauri\target\debug\*') -or
  ($_.Name -eq 'node.exe' -and $_.CommandLine -match 'JobertsVault\\node_modules\\.*(tauri\.js|vite\.js)') -or
  ($_.Name -eq 'node.exe' -and $_.CommandLine -match 'npm-cli\.js" run (tauri dev|dev)') -or
  ($_.Name -eq 'cargo.exe' -and $_.CommandLine -match 'run --no-default-features')
}
$mine | ForEach-Object { "stopping $($_.ProcessId) $($_.Name)"; Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Start-Sleep -Milliseconds 800
if (netstat -ano | Select-String '\]:1420 .*LISTENING') { "port 1420 still in use" } else { "port 1420 free" }
