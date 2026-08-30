param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath,

    [Parameter(Mandatory = $true)]
    [ValidateSet('x64', 'arm64')]
    [string]$ExpectedArchitecture,

    [ValidateRange(1, 30)]
    [int]$StartupSeconds = 5
)

$ErrorActionPreference = 'Stop'

function Get-PeArchitecture([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    $reader = [System.IO.BinaryReader]::new($stream)
    try {
        if ($reader.ReadUInt16() -ne 0x5A4D) {
            throw "Not a PE executable: $Path"
        }
        $stream.Position = 0x3C
        $peOffset = $reader.ReadUInt32()
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) {
            throw "Missing PE signature: $Path"
        }
        switch ($reader.ReadUInt16()) {
            0x8664 { return 'x64' }
            0xAA64 { return 'arm64' }
            default { throw "Unsupported PE machine type in $Path" }
        }
    }
    finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

$resolvedExecutable = (Resolve-Path -LiteralPath $ExecutablePath).Path
$actualArchitecture = Get-PeArchitecture $resolvedExecutable
if ($actualArchitecture -ne $ExpectedArchitecture) {
    throw "Expected a $ExpectedArchitecture executable, found $actualArchitecture."
}

$previousUiTest = $env:MIMI_UI_TEST
$previousStandardOverlay = $env:MIMI_UI_TEST_STANDARD_OVERLAY
$previousAutoStart = $env:MIMI_AUTO_START
$process = $null
try {
    $env:MIMI_UI_TEST = '1'
    $env:MIMI_UI_TEST_STANDARD_OVERLAY = '1'
    $env:MIMI_AUTO_START = '1'
    $process = Start-Process -FilePath $resolvedExecutable -PassThru
    if ($process.WaitForExit($StartupSeconds * 1000)) {
        throw "mimi exited during the $StartupSeconds-second startup smoke test with code $($process.ExitCode)."
    }
    Write-Output "mimi $actualArchitecture UI-test process remained healthy for $StartupSeconds seconds."
}
finally {
    if ($null -ne $process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
        $process.WaitForExit(5000) | Out-Null
    }
    $env:MIMI_UI_TEST = $previousUiTest
    $env:MIMI_UI_TEST_STANDARD_OVERLAY = $previousStandardOverlay
    $env:MIMI_AUTO_START = $previousAutoStart
}
