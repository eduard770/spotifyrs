param([Parameter(Mandatory)][string]$Executable)
$ErrorActionPreference = 'Stop'
$path = (Resolve-Path -LiteralPath $Executable).Path
if ([IO.Path]::GetExtension($path) -ne '.exe') { throw 'Seleciona o executável oficial soloist.exe.' }
$secret = Read-Host 'Chave Soloist (não será guardada pelo script)' -AsSecureString
$pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secret)
try {
    $key = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer)
    # A CLI oficial recebe a chave como argumento: processos com acesso suficiente
    # podem inspecioná-la enquanto o Soloist corre. Não registar a linha de comando.
    & $path --device-name 'SpotifyRS local' --api-key $key --ws '127.0.0.1:9090' --cache-size 100
    if ($LASTEXITCODE -eq 10) { Write-Warning 'Esta versão Soloist expirou. Descarrega uma versão oficial mais recente.' }
} finally {
    $key = $null
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer)
    $secret.Dispose()
}
