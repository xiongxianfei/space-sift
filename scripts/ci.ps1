Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-RequiredCommandPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    $command = Get-Command -Name $Name -ErrorAction SilentlyContinue
    if ($null -eq $command) {
        throw "$Name is not available on PATH."
    }

    return $command.Source
}

function Invoke-RequiredCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,

        [Parameter()]
        [string[]]$Arguments = @()
    )

    $commandPath = Get-RequiredCommandPath -Name $Name
    & $commandPath @Arguments

    if ($LASTEXITCODE -ne 0) {
        $renderedArguments = if ($Arguments.Count -gt 0) {
            ' ' + ($Arguments -join ' ')
        } else {
            ''
        }

        throw "$Name$renderedArguments failed with exit code $LASTEXITCODE."
    }
}

Invoke-RequiredCommand -Name npm -Arguments @('ci')
Invoke-RequiredCommand -Name npm -Arguments @('run', 'lint')
Invoke-RequiredCommand -Name npm -Arguments @('run', 'test')
Invoke-RequiredCommand -Name npm -Arguments @('run', 'build')
Invoke-RequiredCommand -Name cargo -Arguments @('check', '--manifest-path', 'src-tauri/Cargo.toml')
