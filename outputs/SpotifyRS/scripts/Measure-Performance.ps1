param(
    [string]$Executable = (Join-Path (Split-Path $PSScriptRoot -Parent) 'spotifyrs.exe'),
    [ValidateSet('idle', 'demo', 'connected')][string]$Scenario = 'demo',
    [ValidateRange(5, 3600)][int]$Seconds = 60,
    [int]$ProcessId = 0,
    [string]$OutputDirectory = (Join-Path (Split-Path $PSScriptRoot -Parent) 'measurements')
)
$ErrorActionPreference = 'Stop'
if ($Scenario -eq 'connected' -and $ProcessId -eq 0) {
    throw 'Para medir a sessão real, inicia sessão manualmente e indica -ProcessId.'
}
$owned = $ProcessId -eq 0
if ($owned) {
    $resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
    $startOptions = @{ FilePath = $resolvedExecutable; PassThru = $true }
    if ($Scenario -eq 'demo') { $startOptions.ArgumentList = '--demo' }
    # Janela interativa: permanece visível durante a medição.
    $appProcess = Start-Process @startOptions
} else {
    $appProcess = Get-Process -Id $ProcessId
    $resolvedExecutable = $appProcess.Path
}
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$samples = [System.Collections.Generic.List[object]]::new()
try {
    Start-Sleep -Seconds 3
    $appProcess.Refresh()
    if ($appProcess.HasExited) { throw 'A aplicação terminou antes da medição.' }
    $previousCpu = $appProcess.TotalProcessorTime.TotalSeconds
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $previousTime = 0.0
    for ($i = 0; $i -lt $Seconds; $i++) {
        Start-Sleep -Seconds 1
        $appProcess.Refresh()
        if ($appProcess.HasExited) { throw 'A aplicação terminou durante a medição.' }
        $now = $watch.Elapsed.TotalSeconds
        $cpu = $appProcess.TotalProcessorTime.TotalSeconds
        $samples.Add([pscustomobject]@{
            ElapsedSeconds = [math]::Round($now, 3)
            WorkingSetMiB = [math]::Round($appProcess.WorkingSet64 / 1MB, 3)
            PrivateBytesMiB = [math]::Round($appProcess.PrivateMemorySize64 / 1MB, 3)
            PeakWorkingSetMiB = [math]::Round($appProcess.PeakWorkingSet64 / 1MB, 3)
            CpuPercent = [math]::Round(100 * ($cpu - $previousCpu) / ($now - $previousTime) / [Environment]::ProcessorCount, 4)
            Threads = $appProcess.Threads.Count
            Handles = $appProcess.HandleCount
        })
        $previousCpu = $cpu
        $previousTime = $now
    }
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $prefix = Join-Path $OutputDirectory "$stamp-$Scenario"
    $samples | Export-Csv -LiteralPath "$prefix.csv" -NoTypeInformation -Encoding UTF8
    $summary = [ordered]@{
        Scenario = $Scenario
        TimestampUtc = [DateTime]::UtcNow.ToString('o')
        Samples = $samples.Count
        WarmupSeconds = 3
        DurationSeconds = [math]::Round($watch.Elapsed.TotalSeconds, 3)
        LogicalProcessors = [Environment]::ProcessorCount
        WindowsVersion = [Environment]::OSVersion.VersionString
        Executable = [IO.Path]::GetFileName($resolvedExecutable)
        ExecutableSHA256 = (Get-FileHash -LiteralPath $resolvedExecutable -Algorithm SHA256).Hash
        MeanWorkingSetMiB = [math]::Round(($samples | Measure-Object WorkingSetMiB -Average).Average, 3)
        MaxWorkingSetMiB = ($samples | Measure-Object WorkingSetMiB -Maximum).Maximum
        PeakWorkingSetMiB = ($samples | Measure-Object PeakWorkingSetMiB -Maximum).Maximum
        MeanPrivateBytesMiB = [math]::Round(($samples | Measure-Object PrivateBytesMiB -Average).Average, 3)
        MeanCpuPercent = [math]::Round(($samples | Measure-Object CpuPercent -Average).Average, 4)
        MaxThreads = ($samples | Measure-Object Threads -Maximum).Maximum
        IncludesSpotifyProcess = $false
    }
    $summary | ConvertTo-Json | Set-Content -LiteralPath "$prefix.json" -Encoding UTF8
    $summary | ConvertTo-Json
} finally {
    if ($owned -and !$appProcess.HasExited) {
        $null = $appProcess.CloseMainWindow()
        if (!$appProcess.WaitForExit(15000)) { $appProcess.Kill() }
    }
}
