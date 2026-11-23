# Stops only the JobertsVault dev processes that `npm run tauri dev` starts.
# npm's and cargo's command lines don't name the project, so they're matched by parentage instead.
$all = Get-CimInstance Win32_Process
$core = @($all | Where-Object {
  ($_.Name -eq 'jobertsvault.exe' -and $_.ExecutablePath -like '*\JobertsVault\src-tauri\target\debug\*') -or
  ($_.Name -eq 'node.exe' -and $_.CommandLine -match 'JobertsVault\\node_modules\\.*(tauri\.js|vite\.js)')
})
$coreIds = @($core | ForEach-Object { $_.ProcessId })
$up1 = @($core | ForEach-Object { $_.ParentProcessId })
$up2 = @($all | Where-Object { $up1 -contains $_.ProcessId } | ForEach-Object { $_.ParentProcessId })
$extra = @($all | Where-Object {
  ($_.Name -eq 'cargo.exe' -and $coreIds -contains $_.ParentProcessId) -or
  ($_.Name -eq 'node.exe' -and $_.CommandLine -match 'npm-cli\.js" run (tauri dev|dev)' -and ($up1 + $up2) -contains $_.ProcessId)
})
foreach ($p in ($core + $extra)) {
  "stopping $($p.ProcessId) $($p.Name)"
  try { Stop-Process -Id $p.ProcessId -Force -ErrorAction Stop } catch { "could not stop $($p.ProcessId): $($_.Exception.Message)" }
}
Start-Sleep -Milliseconds 800
if (netstat -ano | Select-String ':1420 .*LISTENING') { "port 1420 still in use" } else { "port 1420 free" }
