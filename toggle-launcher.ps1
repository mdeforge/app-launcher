# toggle-launcher.ps1
# Sends a "toggle" command to the app launcher via named pipe
# Used by GlazeWM to toggle visibility without spawning new processes

$pipeName = "app-launcher-ipc"
$exePath = Join-Path $PSScriptRoot "target\release\al.exe"

try {
    # Try to connect to existing instance
    $pipe = New-Object System.IO.Pipes.NamedPipeClientStream(".", $pipeName, [System.IO.Pipes.PipeDirection]::Out)
    $pipe.Connect(100)  # 100ms timeout

    $sw = New-Object System.IO.StreamWriter($pipe)
    $sw.Write("toggle")
    $sw.Flush()
    $sw.Dispose()
    $pipe.Dispose()
}
catch [TimeoutException] {
    # No existing instance - start the launcher
    Start-Process -FilePath $exePath -WindowStyle Hidden
}
catch {
    # Any other error - try starting fresh
    Start-Process -FilePath $exePath -WindowStyle Hidden
}
